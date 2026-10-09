use std::range::Range;
use super::BoundaryFinder;

pub(super) const TEST_DATA: [u8; 32] = 
        [50, 12, 127, 1, 230, 71, 60, 0, 
        1, 1, 1, 1, 200, 199, 2, 3,
        4, 5, 6, 7, 8, 7, 6, 100, 
        99, 98, 97, 34, 3, 3, 90, 91];

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

/// Tiny deterministic PRNG (xorshift64) so failures are reproducible from the seed.
pub(super) struct Rng(u64);

impl Rng {
    pub(super) fn new(seed: u64) -> Self {
        // xorshift gets stuck at 0 forever, so force the seed non-zero
        Self(seed | 1)
    }

    pub(super) fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Value in 0..n. n must be > 0.
    pub(super) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Data built from short segments of different shapes, each chosen to stress
/// a different part of the deque algorithm.
pub(super) fn mixed_data(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(len);
    while data.len() < len {
        let run = 1 + rng.below(64);
        match rng.below(4) {
            // random bytes: the normal case
            0 => {
                for _ in 0..run {
                    data.push(rng.next() as u8);
                }
            }
            // one repeated byte: lots of equal values, hits the tie path
            1 => {
                let b = rng.next() as u8;
                data.extend(std::iter::repeat_n(b, run));
            }
            // zeros: low-entropy data with no strict maximum
            2 => data.extend(std::iter::repeat_n(0, run)),
            // descending ramp: every value smaller than the last, so the deque grows long
            _ => {
                let mut b = rng.next() as u8;
                for _ in 0..run {
                    data.push(b);
                    b = b.wrapping_sub(1);
                }
            }
        }
    }
    data.truncate(len);
    data
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

    assert_eq!(actual_out, expected_out, "boundaries differ: {context}, range {range:?}");
    assert_eq!(actual_stats, expected_stats, "stats differ: {context}, range {range:?}");
}
