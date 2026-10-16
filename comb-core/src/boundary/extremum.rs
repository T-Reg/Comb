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
    #[must_use]
    pub fn new(h: usize) -> Self {
        Self {
            h,
        }
    }
}

impl BoundaryFinder for Extremum {
    fn find_boundaries(&self, data: &[u8], range: Range<usize>, out: &mut Vec<usize>) -> ScanStats {
        // manually make one chunk if data size is too tiny to make even one u64 out of the bytes.
        if data.len() < 8{
            return ScanStats::empty();
        }
        
        let (start, end) = calculate_scan_bounds(&range, data.len(), self.h);

        // no position has a full window inside the data and range, so nothing to scan.
        // without this the loop below still reads entries up to end + h, past the data.
        if start > end {
            return ScanStats::empty();
        }

        let mut window = Window::new(self.h);

        let len_b4 = out.len();
        out.extend(
            ((start - self.h)..=(end + self.h))
                .filter_map(|i| window.is_boundary(data, i))
                .filter(|i| *i >= start),
        );

        ScanStats {
            bytes_scanned: (end + 1).saturating_sub(start) as u64,
            rule_cuts: (out.len() - len_b4) as u64,
        }
    }

    fn mean_chunk_size(&self) -> usize {
        extremum_mean_chunk_size(self.h)
    }
}

struct Window {
    // key: index/location of the byte in data
    // value: the 8 byte value returned from value_at
    mem: VecDeque::<(usize, u64)>,

    // the algorithm checks the right side of the [i - h, i + h] window with
    // it's comparisons. This bit tracks whether the mem.front() also passes
    // the left side check
    passes_left_check: bool,

    // window size
    h: usize,
}

impl Window {
    fn new(h: usize) -> Self {
        Self {
            h,
            mem: VecDeque::with_capacity(h + 1),
            passes_left_check: false,
        }
    }
    
    fn is_boundary(&mut self, data: &[u8], i: usize) -> Option<usize> {
        let val_i = value_at(data, i);
        // expire entries that are now out of the window
        if i >= self.h && self.mem.front().is_some_and(|&(idx, _)| idx < i - self.h) {
            self.mem.pop_front();
            self.passes_left_check = false;
        }

        // take out anything smaller than F(i) as they're not maximums anymore
        let mut tie = false;
        while let Some(&(_, v)) = self.mem.back() {
            match v.cmp(&val_i) {
                Ordering::Less => {
                    self.mem.pop_back();
                },
                Ordering::Equal => {
                    tie = true;
                    self.mem.pop_back();
                },
                Ordering::Greater => break,
            }
        }

        // if at this point we cleared the whole deque, left check passed
        // baring any ties
        if self.mem.is_empty() {
            self.passes_left_check = !tie;
        }

        self.mem.push_back((i, val_i));

        let passes_right_check = i >= self.h && self.mem.front().is_some_and(|&(idx, _)| idx == i - self.h);
        (passes_right_check && self.passes_left_check)
            .then(|| i - self.h)
    }
}

#[cfg(test)]
mod tests {
    use super::Extremum;
    use crate::boundary::test_support::validate_happy_h1;
    use crate::boundary::test_support::validate_happy_h2;
    use crate::boundary::test_support::validate_backwards_range;
    use crate::boundary::test_support::validate_end_0;
    use crate::boundary::test_support::validate_short_data;
    use crate::boundary::reference::Reference;
    use crate::boundary::test_support::{assert_same, REAL_FILES};
    use crate::test_support::{ mixed_data, Rng, HS };
    
    #[test]
    fn find_boundaries_happy_h1() {
        validate_happy_h1(Extremum::new);
    }

    #[test]
    fn find_boundaries_happy_h2() {
        validate_happy_h2(Extremum::new);
    }

    #[test]
    fn find_boundaries_short_data() {
        validate_short_data(Extremum::new);
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
