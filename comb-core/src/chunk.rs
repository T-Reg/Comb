//! Chunking layer that uses the boundary finder to build the actual chunks from data.

use crate::chunk_id::ChunkId;
use crate::boundary::BoundaryFinder;
use crate::boundary::ScanStats;
use std::range::Range;
use std::collections::HashMap;
use std::fmt::Display;
use std::error::Error;
use std::fmt;
use std::fmt::Formatter;

/// The whole reason we are here: one chunk of the data.
#[derive(Debug, PartialEq, Eq)]
pub struct Chunk {
    /// id of the chunk, a Blake3 hash of the data
    pub id: ChunkId,

    /// the start index
    pub offset: usize,

    /// the number of bytes in the chunk
    pub len: usize,

    /// the type of boundary for the end of this chunk.
    /// for example, if the end of the chunk was cut due to a 
    /// max size clamp, the type is "Clamp"
    pub end: BoundaryKind,
}

impl Chunk {
    /// The index of the last byte in the chunk, exclusive.
    pub fn end_pos(&self) -> usize {
        self.offset + self.len
    }
}

/// Describes how boundaries can be determined/calculated.
#[derive(Debug, PartialEq, Eq)]
pub enum BoundaryKind {
    /// The most common.  Means this boundary was cut here due
    /// to the content defined rule of the boundary finder.
    Rule,

    /// Means this boundary was the result of a max chunk size
    /// clamp.
    Clamp,

    /// The end of the chunk is the end of the whole range
    RangeEnd,
}

/// Error when reassembling data chunks
#[derive(Debug, PartialEq, Eq)]
pub enum DataReassemblyError {
    /// Chunk was not in the data store.  Can happen if some data gets corrupted 
    /// deleted or the download failed.
    MissingChunk(ChunkId),

    /// the chunk's metadata expected a different amount of bytes than the actual chunk in 
    /// the data store.
    LengthMismatch {
        /// chunk id of the chunk with length mismatch.
        id: ChunkId, 
        /// expected size based on chunk metadata.
        exp: usize,
        /// actual size observed in the data store.
        act: usize },
}

impl Error for DataReassemblyError {}

impl Display for DataReassemblyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingChunk(id) => write!(f, "Chunk with id {id} was not found in the data store during data reassembly."),
            Self::LengthMismatch { id, exp, act } => write!(f, "Chunk with id {id} had length of {act} instead of expected length {exp}.")
        }
    }
}

/// Cuts the data into chunks based on the boundaries found by the BoundaryFinder.
/// if range has length 0, empty Vec is returned.
///
/// # Panics
///
/// - if range.start > range.end
/// - if range.end > data.len()
/// - if range.end <= 0
pub fn chunk_range<F : BoundaryFinder>(data: &[u8], range: Range<usize>, finder: &F) -> (Vec<Chunk>, ScanStats) {
    assert!(range.start <= range.end, "invalid range: start > end");
    assert!(range.end <= data.len(), "invalid range: end > data length");
    assert!(range.end > 0, "invalid range: end == 0");
    
    if range.end == range.start {
        return (Vec::new(), ScanStats::empty());
    }
    
    let mut boundary_indexes = Vec::new();
    let stats = finder.find_boundaries(data, range, &mut boundary_indexes);

    let mut chunks = Vec::new();
    let mut start = range.start;
    for &index in &boundary_indexes {
        chunks.push(Chunk {
            id: ChunkId::calculate(&data[start..index as usize]),
            offset: start,
            len: index - start,
            end: BoundaryKind::Rule,
        });
        start = index;
    }

    // one last chunk at the end
    chunks.push(Chunk {
        id: ChunkId::calculate(&data[start..range.end]),
        offset: start,
        len: range.end - start,
        end: BoundaryKind::RangeEnd,
    });

    (chunks, stats)
}

/// Reassembles chunks in correct order to rebuild the original bytes.
/// for now using an in memory store as we build this out.  But the complete
/// version must support on disk or s3 data stores.
/// Checks if the data len matches up, but doesn't check for certain the data 
/// isn't corrupt or changed.  That is planned for later though.
pub fn reassemble_chunks(chunks: &Vec<Chunk>, data_store: &HashMap<ChunkId, Vec<u8>>) -> Result<Vec<u8>, DataReassemblyError> {
    let mut result = Vec::new();

    for chunk in chunks {
        if !data_store.contains_key(&chunk.id) {
            return Err(DataReassemblyError::MissingChunk(chunk.id));
        }
        
        let bytes = data_store.get(&chunk.id)
            .expect("Failed to get chunk bytes from data store.");
        if bytes.len() != chunk.len {
            return Err(DataReassemblyError::LengthMismatch { id: chunk.id, exp: chunk.len, act: bytes.len() });
        }

        result.extend_from_slice(&bytes);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{ chunk_range, reassemble_chunks, BoundaryKind, Chunk, DataReassemblyError };
    use crate::chunk_id::ChunkId;
    use crate::boundary::ScanStats;
    use crate::test_support::{ Rng, mixed_data, HS };
    use crate::boundary::extremum::Extremum;
    use crate::boundary::reference::Reference;
    use std::collections::HashMap;
    use std::range::Range;
    use std::sync::LazyLock;

    static CASES: LazyLock<Vec<Vec<u8>>> = LazyLock::new(|| {
        (0..30u64).map(|seed| {
            let mut rng = Rng::new(seed);
            let len = 1 + rng.below(2000);
            mixed_data(&mut rng, len)
        }).collect()
    });
    
    
    #[test]
    fn test_chunk_full_data() {
        for data in CASES.iter() {
            validate_chunking(&data, (0..data.len()).into());
        }
    }

    #[test]
    fn test_chunk_inner_range() {
        for data in CASES.iter() {
            validate_chunking(&data, (((data.len() / 10) as usize)..data.len() / 2).into());
        }
    }

    fn validate_chunking(data: &[u8], range: Range<usize>) {
        for h in HS {
            let finder = Extremum::new(h);
            let reference = Reference::new(h);
            let (chunks, stats) = chunk_range(&data, range, &finder);
            let (ref_chunks, ref_stats) = chunk_range(&data, range, &reference);

            // chunks and stats agree with reference
            assert_eq!(chunks, ref_chunks, "Chunks did not match reference chunks.");
            assert_eq!(stats, ref_stats, "ScanStats did not match reference ScanStats.");

            // first chunk starts at start, last chunk ends at the end
            assert_eq!(chunks[0].offset, range.start, "the first data chunk did not start on expected index.");
            let last_chunk = &chunks[chunks.len() - 1];
            assert_eq!(last_chunk.end_pos(), range.end, "the last data chunk didn't end on expected index.");
            assert_eq!(last_chunk.end, BoundaryKind::RangeEnd, "The last chunk had unexpected boundary kind.");

            // no gaps or overlaps in the chunks.
            println!("created {} chunks.", chunks.len());
            if chunks.len() > 1 {
                for i in 0..chunks.len() - 1 {
                    let chunk_1 = &chunks[i];
                    let chunk_2 = &chunks[i + 1];
                    assert_eq!(chunk_1.end_pos(), chunk_2.offset, "Two adjacent chunks have indexes that don't line up.");
                }
            }

            let mut data_store = HashMap::new();
            for chunk in &chunks {
                data_store.insert(chunk.id, data[chunk.offset..chunk.end_pos()].to_vec());
            }

            // reassemble the chunks and assert the data the same as before
            match reassemble_chunks(&chunks, &data_store) {
                Ok(reassembled) => {
                    assert_eq!(&reassembled, &data[range.start..range.end], "reassembled data did not original data.");
                },
                Err(e) => {
                    panic!("hit unexpected error reassembling the data: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_chunk_one_byte_file() {
        let data: [u8; 1] = [10];
        for h in HS {
            let finder = Extremum::new(h);
            let (chunks, _stats) = chunk_range(&data, (0..1).into(), &finder);

            assert_eq!(chunks.len(), 1, "Expected one chunk when data size is one.");
            assert_eq!(chunks[0], Chunk {
                id: ChunkId::calculate(&data),
                offset: 0,
                len: 1,
                end: BoundaryKind::RangeEnd,
            }, "The single chunk did not cover the whole one byte file.");
        }
    }

    #[test]
    fn test_chunk_empty_data() {
        // an empty range inside non-empty data gives no chunks and no scan work
        let data = &CASES[0];
        let mid = data.len() / 2;
        let finder = Extremum::new(1);
        let (chunks, stats) = chunk_range(data, (mid..mid).into(), &finder);

        assert!(chunks.is_empty(), "Expected no chunks for an empty range.");
        assert_eq!(stats, ScanStats::empty(), "Expected empty ScanStats for an empty range.");
    }

    #[test]
    #[should_panic(expected = "invalid range: start > end")]
    fn test_chunk_invalid_range() {
        let data = &CASES[0];
        let finder = Extremum::new(1);
        chunk_range(data, (data.len()..0).into(), &finder);
    }

    #[test]
    #[should_panic(expected = "invalid range: end > data length")]
    fn test_chunk_walks_outside_data_len() {
        let data = &CASES[0];
        let finder = Extremum::new(1);
        chunk_range(data, (0..data.len() + 1).into(), &finder);
    }

    #[test]
    #[should_panic(expected = "invalid range: end == 0")]
    fn test_chunk_end_is_0() {
        let data = &CASES[0];
        let finder = Extremum::new(1);
        chunk_range(data, (0..0).into(), &finder);
    }

    #[test]
    fn test_reassemble_not_in_data_store() {
        let data = &CASES[0];
        let finder = Extremum::new(1);
        let (chunks, _stats) = chunk_range(data, (0..data.len()).into(), &finder);

        let mut data_store = HashMap::new();
        for chunk in &chunks {
            data_store.insert(chunk.id, data[chunk.offset..chunk.end_pos()].to_vec());
        }

        // remove by id rather than skipping the insert, since identical chunks share an id
        let missing = chunks[chunks.len() - 1].id;
        data_store.remove(&missing);
        assert_eq!(reassemble_chunks(&chunks, &data_store),
            Err(DataReassemblyError::MissingChunk(missing)),
            "Expected MissingChunk error for the chunk left out of the data store.");
    }

    #[test]
    fn test_reassemble_mismatch_length() {
        let data = &CASES[0];
        let finder = Extremum::new(1);
        let (chunks, _stats) = chunk_range(data, (0..data.len()).into(), &finder);

        let mut data_store = HashMap::new();
        for chunk in &chunks {
            data_store.insert(chunk.id, data[chunk.offset..chunk.end_pos()].to_vec());
        }

        // corrupt the first chunk by adding an extra byte
        let first = &chunks[0];
        data_store.get_mut(&first.id).unwrap().push(0);

        assert_eq!(reassemble_chunks(&chunks, &data_store),
            Err(DataReassemblyError::LengthMismatch { id: first.id, exp: first.len, act: first.len + 1 }),
            "Expected LengthMismatch error for the chunk with an extra byte.");
    }
}