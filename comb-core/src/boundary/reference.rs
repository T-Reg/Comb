//! A slow naive boundary finder for testing that finds boundaries based on local maximums, 
//! same as extremum.rs except how it does it is much slower and meant for testing/validating.

use std::range::Range;
use crate::boundary::BoundaryFinder;
use crate::boundary::ScanStats;
use super::local_max::value_at;
use super::local_max::extremum_mean_chunk_size;
use super::local_max::calculate_scan_bounds;

/// Boundary finder with a naive slow implementation
#[derive(Debug)]
pub struct Reference {
    h: usize,
}

impl Reference {
    /// Constructor where h is the window size for boundary rule. A position, p, is a
    /// boundary if it is the strict maximum within (p - h)..(p + h).
    pub fn new(h: usize) -> Self {
        Self { h }
    }

    /// p is a boundary if every other entry in its window is strictly smaller.
    fn is_boundary(&self, data: &[u8], p: usize) -> bool {
        let val = value_at(data, p);
        ((p - self.h)..=(p + self.h))
            .filter(|&other| other != p)
            .all(|other| value_at(data, other) < val)
    }
}

impl BoundaryFinder for Reference {
    fn find_boundaries(&self, data: &[u8], range: Range<usize>, out: &mut Vec<usize>) -> ScanStats {
        // manually make one chunk if data size is too tiny to make even one u64 out of the bytes.
        if data.len() < 8 {
            return ScanStats::empty();
        }
        
        let (start, end) = calculate_scan_bounds(&range, data.len(), self.h);

        // no position has a full window inside the data and range, so nothing to scan.
        if start > end {
            return ScanStats::empty();
        }

        let len_b4 = out.len();
        out.extend((start..=end).filter(|&p| self.is_boundary(data, p)));

        ScanStats {
            bytes_scanned: (end + 1 - start) as u64,
            rule_cuts: (out.len() - len_b4) as u64,
        }
    }

    fn mean_chunk_size(&self) -> usize {
        extremum_mean_chunk_size(self.h)
    }
}

#[cfg(test)]
mod tests {
    use super::Reference;
    use crate::boundary::test_support::validate_happy_h1;
    use crate::boundary::test_support::validate_happy_h2;
    use crate::boundary::test_support::validate_backwards_range;
    use crate::boundary::test_support::validate_end_0;
    use crate::boundary::test_support::validate_short_data;
    
    #[test]
    fn find_boundaries_happy_h1() {
        validate_happy_h1(Reference::new);
    }

    #[test]
    fn find_boundaries_happy_h2() {
        validate_happy_h2(Reference::new);
    }

    #[test]
    fn find_boundaries_short_data() {
        validate_short_data(Reference::new);
    }

    #[test]
    #[should_panic(expected = "invalid range: start > end")]
    fn find_boundary_range_start_larger_than_end() {
        validate_backwards_range(Reference::new);
    }

    #[test]
    #[should_panic(expected = "invalid range: end == 0")]
    fn find_boundary_range_end_is_0() {
        validate_end_0(Reference::new);
    }
}