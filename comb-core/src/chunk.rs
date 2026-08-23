// Takes the raw bytes and splits them into chunks using our tiered approach to use more granular
// chunks for chunks that have changed since the previous version.

use crate::gear::RollingHasher;

pub struct ChunkSplitter<'a> {
    buffer: &'a Vec<u8>,
    scanner: BoundaryScanner<'a>,
    pointer: usize,
}

impl<'a> ChunkSplitter<'a> {
    pub fn new(buffer: &'a Vec<u8>) -> Self { Self { buffer, scanner: BoundaryScanner::new(), pointer: 0 } }

    pub fn has_next(&self) -> bool {
        self.pointer >= self.buffer.len()
    }

    pub fn next(&mut self) -> Chunk {
        let chunk_end = self.scanner.next_boundary(self.buffer, &self.pointer, &0);
        let result = Chunk::new(self.buffer[self.pointer..chunk_end].to_vec());
        self.pointer = chunk_end;
        result
    }
}

pub struct Chunk {
    bytes : Vec<u8>,
}

impl Chunk {
    pub fn new(bytes : Vec<u8>) -> Self { Self { bytes } }
}

struct BoundaryScanner<'a> {
    hasher: RollingHasher<'a>
}

impl<'a> BoundaryScanner<'a> {
    pub fn new() -> Self { Self { hasher: Default::default() } }

    // returns the offset of the next chunk boundary starting from the start_pos
    pub fn next_boundary(&mut self, buffer: &'a Vec<u8>, start_pos: &usize, tier: &u8) -> usize {
        let mut curr_pos = *start_pos;

        if curr_pos >= buffer.len() {
            return curr_pos;
        }

        loop {
            self.hasher.roll_byte(&buffer[curr_pos as usize]);
            curr_pos += 1;

            if self.hasher.is_match(Self::get_mask(tier)) {
                break curr_pos
            }

            if curr_pos >= buffer.len() {
                break curr_pos
            }
        }
    }

    fn get_mask(tier: &u8) -> u64 {
        0x3F
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::vec;
    use crate::gear::splitmix64_next;
    use crate::chunk::{BoundaryScanner, ChunkSplitter, Chunk};

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

        let mut actual: Vec<usize> = vec![0];
        // assumes buffer of 0x3F for now.
        let expected: Vec<usize> = vec![0, 43, 119, 304, 327, 375, 416, 525, 550, 569, 596, 634, 676,
                                      702, 704, 747, 902, 980, 991, 1023, 1024];

        let mut boundary = scanner.next_boundary(&buffer, &0, &0);
        while boundary < buffer.len() {
            actual.push(boundary);
            boundary = scanner.next_boundary(&buffer, &boundary, &0);
        }
        actual.push(buffer.len());

        assert_eq!(expected, actual);
    }

    #[test]
    fn chunk_splitter_builds_chunks() {
        let mut buffer: Vec<u8> = vec![0; 1024];

        // Setup such that each value in buffer is set to the index of the chunk it should be a
        // part of after chunk splitter runs.
        let expected_boundaries: Vec<usize> = vec![0, 43, 119, 304, 327, 375, 416, 525, 550, 569,
                                                   596, 634, 676, 702, 704, 747, 902, 980, 991,
                                                   1023, 1024];
        let mut prev: usize = 0;
        for (index, boundary) in expected_boundaries.iter().enumerate() {
            buffer[prev..*boundary].fill(index as u8);
            prev = *boundary;
        }

        // loop through each chunk the splitter makes and assert all values eq it's index.
        let mut splitter = ChunkSplitter::new(&buffer);
        let mut index = 0;
        while splitter.has_next() {
            let curr: Chunk = splitter.next();
            assert_all_elements_eq(&curr.bytes, index);
            index += 1;
        }
    }

    fn assert_all_elements_eq<T: PartialEq + Debug>(vec : &Vec<T>, value: T) {
        for curr in vec {
            assert_eq!(*curr, value);
        }
    }
}