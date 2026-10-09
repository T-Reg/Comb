//! Small mod for code common to local maximum based boundary finders

use std::range::Range;

/// Calculate big endian 8 byte value at a location.
#[inline]
pub(super) fn value_at(data: &[u8], index: usize) -> u64 {
    u64::from_be_bytes(data[index..index+8].try_into().unwrap())
}

pub(super) fn calculate_scan_bounds(range: &Range<usize>,
  data_len: usize, h: usize) -> (usize, usize) {
    assert!(range.start <= range.end, "invalid range: start > end");
    assert!(range.end <= data_len, "invalid range: end > data length");
    assert!(range.end > 0, "invalid range: end == 0");
        
    let start = std::cmp::max(h, range.start + 1);
    let end = std::cmp::min(range.end - 1, data_len.saturating_sub(8 + h));
    (start, end)
}

/// Calculate the mean chunk size produced by extremum algorithms
/// based on the window size, h.
#[inline]
pub(super) fn extremum_mean_chunk_size(h: usize) -> usize {
    2 * h + 1
}