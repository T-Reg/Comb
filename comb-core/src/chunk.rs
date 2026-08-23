// Takes the raw bytes and splits them into chunks using our tiered approach to use more granular
// chunks for chunks that have changed since the previous version.

use crate::gear::RollingHasher;

struct ChunkSplitter<'a> {
    buffer: &'a Vec<u8>,
    scanner: BoundaryScanner<'a>,
}

impl<'a> ChunkSplitter<'a> {
    pub fn new(buffer: &'a Vec<u8>) -> Self { Self { buffer, scanner: BoundaryScanner::new() } }

    pub fn next(&mut self) -> Chunk {
        // TODO
        Chunk::new(vec![0])
    }
}

struct BoundaryScanner<'a> {
    hasher: RollingHasher<'a>
}

impl<'a> BoundaryScanner<'a> {
    pub fn new() -> Self { Self { hasher: Default::default() } }

    // returns the offset of the next chunk boundary starting from the start_pos
    pub fn next_boundary(&mut self, buffer: &'a Vec<u8>, start_pos: &u64, tier: &u8) -> u64 {
        let mut curr_pos = *start_pos;

        if curr_pos >= buffer.len() as u64 {
            return curr_pos;
        }

        loop {
            self.hasher.roll_byte(&buffer[curr_pos as usize]);
            curr_pos += 1;

            if self.hasher.is_match(Self::get_mask(tier)) {
                break curr_pos
            }

            if curr_pos >= buffer.len() as u64 {
                break curr_pos
            }
        }
    }

    fn get_mask(tier: &u8) -> u64 {
        0x3F
    }
}

struct Chunk {
    bytes : Vec<u8>,
}

impl Chunk {
    pub fn new(bytes : Vec<u8>) -> Self { Self { bytes } }
}

#[cfg(test)]
mod tests {
    use std::vec;
    use crate::gear::splitmix64_next;
    use crate::chunk::BoundaryScanner;

    #[test]
    fn boundary_scanner_finds_boundary() {
        let mut scanner = BoundaryScanner::new();
        let mut buffer: Vec<u8> = Vec::with_capacity(1024);

        let mut state = 34890238490; //arbitrary seed.
        for _ in 0..1024 {
            let next_val: u64;
            (next_val, state) = splitmix64_next(state);
            buffer.push(next_val as u8);
        }

        let mut actual: Vec<u64> = vec![0];
        // assumes buffer of 0x3F
        let expected: Vec<u64> = vec![0, 43, 119, 304, 327, 375, 416, 525, 550, 569, 596, 634, 676,
                                      702, 704, 747, 902, 980, 991, 1023, 1024];

        let mut boundary = scanner.next_boundary(&buffer, &0, &0);
        while boundary < buffer.len() as u64 {
            actual.push(boundary);
            boundary = scanner.next_boundary(&buffer, &boundary, &0);
        }
        actual.push(buffer.len() as u64);

        assert_eq!(expected, actual);
    }
}