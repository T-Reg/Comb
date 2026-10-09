//! Local Max boundary finder implementation using BBG 2010 SS8 algorithm.
//! That is to say mich faster than the reference implementation
//! in reference.rs. Computes in one clean pass with <= 2 comparisons
//! per position.  Suitable for real use.

use std::range::Range;
use std::cmp::Ordering;
use super::local_max::value_at;
use super::local_max::extremum_mean_chunk_size;
use super::local_max::calculate_scan_bounds;
use crate::boundary::BoundaryFinder;
use crate::boundary::ScanStats;
use std::collections::VecDeque;

/// Finds boundaries based on local maximums.
/// say p(i) is the byte vale at index i. A position, p(i), is a
/// boundary if p(i) is the strict maximum in range (i - h)..(i + h).
#[derive(Debug)]
pub struct Extremum {
    h: usize,
}

impl Extremum {
    /// Constructor where h is the window size for boundary rule. A position, p, is a
    /// boundary if it is the strict maximum within (p - h)..(p + h).
    pub fn new(h: usize) -> Self {
        Self {
            h: h,
        }
    }

    fn is_boundary(&self, data: &[u8], i: usize, mem: &mut VecDeque<(usize, u64)>, passes_left_check: &mut bool) -> bool {
        let val_i = value_at(data, i);
        // expire entries that are now out of the window
        if i >= self.h && mem.front().is_some_and(|&(idx, _)| idx < i - self.h) {
            mem.pop_front();
            *passes_left_check = false;
        }

        // take out anything smaller than F(i) as they're not maximums anymore
        let mut tie = false;
        while let Some(&(_, v)) = mem.back() {
            match v.cmp(&val_i) {
                Ordering::Less => {
                    mem.pop_back();
                },
                Ordering::Equal => {
                    tie = true;
                    mem.pop_back();
                },
                Ordering::Greater => break,
            }
        }

        // if at this point we cleared the whole deque, left check passed
        // baring any ties
        if mem.is_empty() {
            *passes_left_check = !tie;
        }

        mem.push_back((i, val_i));

        let passes_right_check = i >= self.h && mem.front().is_some_and(|&(idx, _)| idx == i - self.h);
        return passes_right_check && *passes_left_check;
    }
}

impl BoundaryFinder for Extremum {
    fn find_boundaries(&self, data: &[u8], range: Range<usize>, out: &mut Vec<usize>) -> ScanStats {
        let (start, end) = calculate_scan_bounds(&range, data.len(), self.h);
        let mut rule_cuts_count = 0;
        
        // key: index/location
        // value: the 8 byte value returned from value_at
        let mut mem: VecDeque<(usize, u64)>= VecDeque::with_capacity(self.h + 1);

        // the algorithm checks the right side of the [i - h, i + h] window with
        // it's comparisons. This bit tracks whether the mem.front() also passes
        // the left side check
        let mut passes_left_check = false;

        for i in (start - self.h)..=(end + self.h) {
            if self.is_boundary(data, i, &mut mem, &mut passes_left_check) && i - self.h >= start {
                rule_cuts_count += 1;
                out.push((i - self.h) as usize);
            }
        }

        ScanStats {
            bytes_scanned: (end+ 1).saturating_sub(start) as u64,
            rule_cuts: rule_cuts_count,
        }
    }

    fn mean_chunk_size(&self) -> usize {
        extremum_mean_chunk_size(self.h)
    }
}

#[cfg(test)]
mod tests {
    use super::Extremum;
    use crate::boundary::test_support::validate_happy_h1;
    use crate::boundary::test_support::validate_happy_h2;
    use crate::boundary::test_support::validate_backwards_range;
    use crate::boundary::test_support::validate_end_0;
    use crate::boundary::reference::Reference;
    use crate::boundary::test_support::{assert_same, mixed_data, Rng, REAL_FILES};

    const HS: [usize; 9] = [0, 1, 2, 3, 5, 8, 16, 33, 64];
    
    #[test]
    fn find_boundaries_happy_h1() {
        validate_happy_h1(Extremum::new);
    }

    #[test]
    fn find_boundaries_happy_h2() {
        validate_happy_h2(Extremum::new);
    }

    #[test]
    #[should_panic(expected = "invalid range: start > end")]
    fn find_boundary_range_start_larger_than_end() {
        validate_backwards_range(Extremum::new);
    }

    #[test]
    #[should_panic(expected = "invalid range: end == 0")]
    fn find_boundary_range_end_is_0() {
        validate_end_0(Extremum::new);
    }

    #[test]
    fn matches_reference_on_generated_data() {
        for seed in 1..=30u64 {
            let mut rng = Rng::new(seed);
            let len = 1 + rng.below(2000);
            let data = mixed_data(&mut rng, len);

            for h in HS {
                let reference = Reference::new(h);
                let extremum = Extremum::new(h);
                let context = format!("seed {seed}, h {h}, len {len}");

                assert_same(&reference, &extremum, &data, (0..len).into(), &context);

                for _ in 0..10 {
                    let end = 1 + rng.below(len);   // end >= 1: end == 0 is a deliberate panic
                    let start = rng.below(end + 1); // start <= end
                    assert_same(&reference, &extremum, &data, (start..end).into(), &context);
                }
            }
        }
    }

    #[test]
    fn matches_reference_on_real_files() {
        for (name, data) in REAL_FILES {
            for h in [1, 4, 16, 64, 256] {
                let context = format!("file {name}, h {h}");
                assert_same(
                    &Reference::new(h),
                    &Extremum::new(h),
                    data,
                    (0..data.len()).into(),
                    &context,
                );
            }
        }
    }
}
