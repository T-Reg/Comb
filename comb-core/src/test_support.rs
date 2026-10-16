pub(super) const HS: [usize; 9] = [0, 1, 2, 3, 5, 8, 16, 33, 64];

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
