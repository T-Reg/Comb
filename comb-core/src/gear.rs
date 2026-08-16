// SplitMix64, reference implementation by Sebastiano Vigna (public domain):
// https://prng.di.unimi.it/splitmix64.c
// This is a very common algorithm for generating pseudo-random numbers
// state: the seed or the state from previous iteration
// returns:  the random number generated this run and the state after this run (to be fed to next
// iteration)
const fn splitmix64_next(mut state: u64) -> (u64, u64) {
    state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z = z ^ (z >> 31);
    (z, state)
}

const PRNG_SEED: u64 = 0xAAC0111BC0111BAA;

const fn build_table() -> GearTable {
    let mut table: GearTable = [0; 256];
    let mut state: u64 = PRNG_SEED;
    let mut i: usize = 0;
    while i < 256 {
        (table[i], state) = splitmix64_next(state);
        i = i + 1;
    }
    table
}

pub type GearTable = [u64; 256];

const DEFAULT_TABLE: GearTable = build_table();

pub struct Hasher<'t> {
    table: &'t GearTable,
    state: u64
}

impl<'t> Hasher<'t> {
    pub fn new(table: &'t GearTable) -> Hasher<'t> {
        Self { table, state: 0 }
    }

    pub fn reset(&mut self) {
        self.state = 0;
    }

    pub fn is_match(&self, mask: u64) -> bool {
        self.state & mask == 0
    }

    pub fn roll_bytes(&mut self, bytes: &[u8]) -> & u64 {
        for &byte in bytes {
            Self::roll_byte(self, &byte);
        }
        &self.state
    }

    fn roll_byte(&mut self, byte: & u8) {
        self.state = (self.state << 1).wrapping_add(self.table[*byte as usize]);
    }
}

impl Default for Hasher<'static> {
    fn default() -> Self { Self { table: &DEFAULT_TABLE, state: 0 } }
}



#[cfg(test)]
mod tests {
    use crate::gear::{ DEFAULT_TABLE, Hasher };
    use crate::gear::splitmix64_next;
    use gearhash::Hasher as external_hasher;

    #[test]
    fn is_match_tests() {
        let mut hasher = Hasher::default();
        hasher.state = 0xF;
        assert!(hasher.is_match(0x10));
        assert!(!hasher.is_match(0x11));
        assert!(!hasher.is_match(0xFF));
        hasher.state = 0xF0;
        assert!(!hasher.is_match(0xFF));
        assert!(!hasher.is_match(0xF0));
        assert!(hasher.is_match(0x0F));
        hasher.state = 0xF00;
        assert!(hasher.is_match(0xFF));
        assert!(hasher.is_match(0xF));
    }

    #[test]
    fn roll_bytes_produces_same_as_other_crate() {
        let bytes: [u8; 10] = [
            0x2,
            0xA,
            0x0,
            0xA,
            0xC,
            0x9,
            0x9,
            0x5,
            0xB,
            0x3
        ];

        let mut hasher = Hasher::default();
        let mut comb_results: [u64; 10] = [0; 10];
        for i in 0..10 {
            hasher.roll_bytes(&bytes);
            comb_results[i] = hasher.state;
        }

        let mut gearhash_results: [u64; 10] = [0; 10];
        let mut hasher: external_hasher = external_hasher::new(&DEFAULT_TABLE);
        for i in 0..10 {
            hasher.update(&bytes[0..i+1]);
            gearhash_results[i] = hasher.get_hash();
            hasher.set_hash(0);
        }

        assert_eq!(gearhash_results, comb_results);
    }

    #[test]
    fn splitmix64_next_correctness() {
        let seed: u64 = 1234567; // basic seed with known outputs to assert
        let expected: [u64; 5] = [
            6457827717110365317,
            3203168211198807973,
            9817491932198370423,
            4593380528125082431,
            16408922859458223821,
        ];
        let mut state = seed;
        for i in 0..5 {
            let x : u64;
            (x, state) = splitmix64_next(state);
            assert_eq!(x, expected[i]);
        }
    }
}
