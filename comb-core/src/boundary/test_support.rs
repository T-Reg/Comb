use super::{BoundaryFinder, ScanStats};
use std::range::Range;

pub(super) const TEST_DATA: [u8; 32] = [
    50, 12, 127, 1, 230, 71, 60, 0, 1, 1, 1, 1, 200, 199, 2, 3, 4, 5, 6, 7, 8, 7, 6, 100, 99, 98,
    97, 34, 3, 3, 90, 91,
];

pub(super) fn validate_happy_h1<F: BoundaryFinder>(new: fn(usize) -> F) {
    let finder = new(1);
    let mut output = Vec::new();

    // full data boundary scan
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

pub(super) fn validate_happy_h2<F: BoundaryFinder>(new: fn(usize) -> F) {
    let finder = new(2);
    let mut output = Vec::new();

    finder.find_boundaries(&TEST_DATA, (0..TEST_DATA.len()).into(), &mut output);

    assert_eq!(output, vec![4, 12, 20]);
}

pub(super) fn validate_backwards_range<F: BoundaryFinder>(new: fn(usize) -> F) {
    let finder = new(1);
    let mut output = Vec::new();
    finder.find_boundaries(&TEST_DATA, (24..12).into(), &mut output);
}

pub(super) fn validate_end_0<F: BoundaryFinder>(new: fn(usize) -> F) {
    let finder = new(1);
    let mut output = Vec::new();
    finder.find_boundaries(&TEST_DATA, (0..0).into(), &mut output);
}

/// Data too short for any position to have a full window gives no boundaries and no
/// panics. A position p needs p >= h and p + h + 8 <= len, so the first length that
/// can have a boundary is 8 + 2h. The data puts one strict peak at h, so at exactly
/// that length the only boundary is h.
pub(super) fn validate_short_data<F: BoundaryFinder>(new: fn(usize) -> F) {
    // h starts at 1: with h == 0 the only candidate is p == 0, which D2 excludes
    for h in [1, 2, 3, 5, 8, 16, 33, 64] {
        let finder = new(h);
        let min_len = 8 + 2 * h;

        for len in 0..=min_len {
            let data: Vec<u8> = (0..len)
                .map(|i| if i == h { 0xFF } else { (i % 7) as u8 })
                .collect();
            let mut output = Vec::new();
            let stats = finder.find_boundaries(&data, (0..len).into(), &mut output);

            if len < min_len {
                assert_eq!(output, vec![], "h {h}, len {len}: expected no boundaries.");
                assert_eq!(
                    stats,
                    ScanStats::empty(),
                    "h {h}, len {len}: expected empty stats."
                );
            } else {
                assert_eq!(
                    output,
                    vec![h],
                    "h {h}, len {len}: expected only the peak at h."
                );
            }
        }
    }
}

/// Real-world-ish input: this crate's own source files, embedded at compile time.
pub(super) const REAL_FILES: [(&str, &[u8]); 4] = [
    ("extremum.rs", include_bytes!("extremum.rs")),
    ("reference.rs", include_bytes!("reference.rs")),
    ("local_max.rs", include_bytes!("local_max.rs")),
    ("boundary.rs", include_bytes!("../boundary.rs")),
];

/// Runs both finders over the same input and asserts identical output and stats.
pub(super) fn assert_same<E: BoundaryFinder, A: BoundaryFinder>(
    expected: &E,
    actual: &A,
    data: &[u8],
    range: Range<usize>,
    context: &str,
) {
    let mut expected_out = Vec::new();
    let mut actual_out = Vec::new();
    let expected_stats = expected.find_boundaries(data, range, &mut expected_out);
    let actual_stats = actual.find_boundaries(data, range, &mut actual_out);

    assert_eq!(
        actual_out, expected_out,
        "boundaries differ: {context}, range {range:?}"
    );
    assert_eq!(
        actual_stats, expected_stats,
        "stats differ: {context}, range {range:?}"
    );
}
