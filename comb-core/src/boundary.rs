//! Finds chunk boundaries in a dataset

pub mod reference;
pub mod extremum;

use std::range::Range;

/// Statistics about the boundary scan (ie. bytes scanned)
#[derive(Debug)]
pub struct ScanStats {
    /// bytes scanned
    pub bytes_scanned: u64,

    /// boundaries created as a result of rules boundary rules, as opposed to
    /// something like max size clamp.
    pub rule_cuts: u64,
}

/// Finds chunk boundaries in the passed data.
pub trait BoundaryFinder {
    /// Appends to 'out' all chunk boundaries in the data set within the given range.
    /// All boundaries, p, will be appended in ascending order and satisfy:
    /// range.start < p < range.end
    /// 'data' is the whole file. The whole file is passed due to the necesity of accessing
    /// data outside the range near the range boundaries due to extremum algorithm sliding 
    /// window calculations
    /// At tier 1, the range is just 0..data.len()
    fn find_boundaries(&self, data: &[u8], range: Range<usize>, out: &mut Vec<usize>) -> ScanStats;

    /// The expected mean chunk size for this boundary finder.
    fn mean_chunk_size(&self) -> usize;

    /// the max chunk size at which point we clamp and declare an arbitrary chunk boundary.
    /// by default, it is 8 x mean_chunk_size.
    fn max_chunk_size(&self) -> usize {
        8 * self.mean_chunk_size()
    }
}