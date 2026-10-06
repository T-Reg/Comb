//! A slow, naive boundary finder for testing

use std::range::Range;
use crate::boundary::BoundaryFinder;
use crate::boundary::ScanStats;

/// Boundary finder with a naive slow implementation
#[derive(Debug)]
pub struct Reference {
    h: usize,
}

impl Reference {
    /// Constructor where h is the window size for boundary rule. A position, p, is a
    /// boundary if it is the strict maximum within (p - h)..(p + h).
    pub fn new(h: usize) -> Self {
        Self {
            h: h,
        }
    }

    fn is_boundary(&self, data: &[u8], index: usize) -> bool {
        let val = Self::value_at(data, index);
        for other in (index - &self.h)..=(index + &self.h) {
            if other == index {
                continue;
            }
                
            if Self::value_at(data, other) >= val {
                return false;
            }
        }

        return true;
    }

    #[inline]
    fn value_at(data: &[u8], index: usize) -> u64 {
        u64::from_be_bytes(data[index..index+8].try_into().unwrap())
    }
}

impl BoundaryFinder for Reference {
    fn find_boundaries(&self, data: &[u8], range: Range<usize>, out: &mut Vec<usize>) -> ScanStats {
        assert!(range.start <= range.end, "invalid range: start > end");
        assert!(range.end <= data.len(), "invalid range: end > data length");
        
        let start = std::cmp::max(self.h, range.start + 1);
        let end = std::cmp::min(range.end - 1, data.len() - 8 - self.h);

        let mut rule_cuts_count = 0;
        for i in start..=end {
            if self.is_boundary(data, i) {
                rule_cuts_count += 1;
                out.push(i as usize);
            }
        }

        ScanStats {
            bytes_scanned: (range.end - range.start) as u64,
            rule_cuts: rule_cuts_count,
        }
    }

    fn mean_chunk_size(&self) -> usize {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::Reference;
    use crate::boundary::BoundaryFinder;

    const TEST_DATA: [u8; 32] = 
        [50, 12, 127, 1, 230, 71, 60, 0, 
        1, 1, 1, 1, 200, 199, 2, 3,
        4, 5, 6, 7, 8, 7, 6, 100, 
        99, 98, 97, 34, 3, 3, 90, 91];
    
    #[test]
    fn find_boundaries_happy_h1() {
        let finder: Reference = Reference::new(1);
        let mut output = Vec::new();

        finder.find_boundaries(&TEST_DATA, (0..TEST_DATA.len()).into(), &mut output);

        assert_eq!(output, vec![2, 4, 12, 20, 23]);

        // smaller window with one less boundary
        output = Vec::new();

        finder.find_boundaries(&TEST_DATA, (0..23).into(), &mut output);

        assert_eq!(output, vec![2, 4, 12, 20]);

        // range in middle
        output = Vec::new();

        finder.find_boundaries(&TEST_DATA, (3..13).into(), &mut output);

        assert_eq!(output, vec![4, 12]);

        // range with no boundaries
        output = Vec::new();

        finder.find_boundaries(&TEST_DATA, (4..12).into(), &mut output);

        assert_eq!(output, vec![]);

        // range of 0 len
        output = Vec::new();

        finder.find_boundaries(&TEST_DATA, (2..2).into(), &mut output);

        assert_eq!(output, vec![]);
    }

    #[test]
    fn find_boundaries_happy_h2() {
        let finder: Reference = Reference::new(2);
        let mut output = Vec::new();

        finder.find_boundaries(&TEST_DATA, (0..TEST_DATA.len()).into(), &mut output);

        assert_eq!(output, vec![4, 12, 20]);
    }

    #[test]
    #[should_panic(expected = "invalid range: end > data length")]
    fn find_boundary_range_end_too_high() {
        let finder: Reference = Reference::new(1);
        let mut output = Vec::new();
        finder.find_boundaries(&TEST_DATA, (0..9999).into(), &mut output);
    }

    #[test]
    #[should_panic(expected = "invalid range: start > end")]
    fn find_boundary_range_start_larger_than_end() {
        let finder: Reference = Reference::new(1);
        let mut output = Vec::new();
        finder.find_boundaries(&TEST_DATA, (24..12).into(), &mut output);
    }
}