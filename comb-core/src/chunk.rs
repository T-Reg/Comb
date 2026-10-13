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

#[derive(Debug, Eq, PartialEq)]
/// Stats produced by the chunking run
pub struct ChunkingStats {
    /// The stats carried over from the boundary scan
    pub scan_stats: ScanStats,

    /// The number of cuts made due to max size clamping
    pub clamp_cuts: u64,
}

impl ChunkingStats {
    /// initialize with stats from the boundary scan
    pub fn new(scan: ScanStats) -> Self {
        Self {
            scan_stats: scan,
            clamp_cuts: 0,
        }
    }

    /// Initualize with empty/default values.
    pub fn empty() -> Self {
        Self {
            scan_stats: ScanStats::empty(),
            clamp_cuts: 0,
        }
    }
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
pub fn chunk_range<F : BoundaryFinder>(data: &[u8], range: Range<usize>, finder: &F) -> (Vec<Chunk>, ChunkingStats) {
    assert!(range.start <= range.end, "invalid range: start > end");
    assert!(range.end <= data.len(), "invalid range: end > data length");
    assert!(range.end > 0, "invalid range: end == 0");
    
    if range.end == range.start {
        return (Vec::new(), ChunkingStats::empty());
    }
    
    let mut boundary_indexes = Vec::new();
    let scan_stats = finder.find_boundaries(data, range, &mut boundary_indexes);

    let mut chunks = Vec::new();
    let mut stats = ChunkingStats::new(scan_stats);
    let mut start = range.start;
    let max_len = finder.max_chunk_size();
    for &index in &boundary_indexes {
        // If this chunk would be larger than the max chunk len, make clamped chunks
        while index - start > max_len {
            push_clamped_chunk(data, start, max_len, &mut chunks);
            start += max_len;
            stats.clamp_cuts += 1;
        }
        
        chunks.push(Chunk {
            id: ChunkId::calculate(&data[start..index as usize]),
            offset: start,
            len: index - start,
            end: BoundaryKind::Rule,
        });
        start = index;
    }

    // one last chunk at the end
    while range.end - start > max_len {
        push_clamped_chunk(data, start, max_len, &mut chunks);
        start += max_len;
        stats.clamp_cuts += 1;
    }
    chunks.push(Chunk {
        id: ChunkId::calculate(&data[start..range.end]),
        offset: start,
        len: range.end - start,
        end: BoundaryKind::RangeEnd,
    });

    (chunks, stats)
}

fn push_clamped_chunk(data: &[u8], start: usize, max_len: usize, chunks: &mut Vec<Chunk>) {
    chunks.push(Chunk {
        id: ChunkId::calculate(&data[start..(start + max_len)]),
        offset: start,
        len: max_len,
        end: BoundaryKind::Clamp,
    });
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
    use crate::boundary::{ BoundaryFinder, ScanStats };
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

            // no chunk is over max size, and every clamp chunk is exactly max size
            let max_size = finder.max_chunk_size();
            for chunk in &chunks {
                assert!(chunk.len <= max_size, "chunk at {} was longer than max size.", chunk.offset);
                if chunk.end == BoundaryKind::Clamp {
                    assert_eq!(chunk.len, max_size, "clamp chunk at {} was not max size.", chunk.offset);
                }
            }

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
    fn test_lots_of_same_bytes_clamp() {
        let finder = Extremum::new(1);
        let max_size = finder.max_chunk_size();
        let data_len: usize = 10 * max_size;
        let data = vec![0xFFu8; data_len];

        let (chunks, stats) = chunk_range(&data, (0..data_len).into(), &finder);

        assert_eq!(chunks.len(), 10, "expected 10 chunks");
        for (i, chunk) in chunks.iter().enumerate() {
            let exp_start = i * max_size;
            assert_eq!(chunk, &Chunk{
                id: ChunkId::calculate(&data[exp_start..exp_start + max_size]),
                offset: exp_start,
                len: max_size,
                end: if i == chunks.len() - 1 { BoundaryKind::RangeEnd } else { BoundaryKind::Clamp },
            });
        }

        assert_eq!(stats.scan_stats.rule_cuts, 0, "expected no rule cuts in repeated bytes.");
        assert_eq!(stats.clamp_cuts, 9, "expected 9 clamp cuts in repeated bytes of this len.");
    }

    #[test]
    fn test_clamp_restarts_from_range_start() {
        // cuts land at range.start + k * max_size, not on a k * max_size grid from 0
        let finder = Extremum::new(1);
        let max_size = finder.max_chunk_size();
        let start = 5;
        let data = vec![0xFFu8; start + 3 * max_size];

        let (chunks, stats) = chunk_range(&data, (start..data.len()).into(), &finder);

        assert_eq!(chunks.len(), 3, "expected 3 chunks");
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.offset, start + i * max_size, "chunk {i} did not start at range.start + k * max_size.");
            assert_eq!(chunk.len, max_size, "chunk {i} was not max_size long.");
        }

        assert_eq!(stats.scan_stats.rule_cuts, 0, "expected no rule cuts.");
        assert_eq!(stats.clamp_cuts, 2, "expected 2 clamp cuts in repeated bytes of this len.");
    }

    #[test]
    fn test_clamp_restarts_from_last_rule_boundary() {
        // random bytes give rule boundaries, then a long 0xFF run forces clamps
        let finder = Extremum::new(1);
        let max_size = finder.max_chunk_size();
        let mut rng = Rng::new(7);
        let mut data = mixed_data(&mut rng, 301);
        data.extend(std::iter::repeat_n(0xFF, 5 * max_size + 3));
        data.extend(mixed_data(&mut rng, 300));
        let range: Range<usize> = (0..data.len()).into();

        let mut rule_bounds = Vec::new();
        finder.find_boundaries(&data, range, &mut rule_bounds);
        let (chunks, _stats) = chunk_range(&data, range, &finder);

        // the gap between rule boundaries that spans the 0xFF run
        let mut prev = range.start;
        let (gap_start, gap_end) = rule_bounds.iter()
            .find_map(|&b| {
                let gap = (prev, b);
                prev = b;
                (b - gap.0 > max_size).then_some(gap)
            })
            .expect("expected a gap between rule boundaries longer than max_size");
        assert_ne!(gap_start % max_size, 0, "test data is weak: gap start is on the k * max_size grid.");

        // inside the gap: clamps every max_size from gap_start, then a Rule chunk up to gap_end
        let first = chunks.iter().position(|c| c.offset == gap_start)
            .expect("no chunk starts at the rule boundary before the gap");
        let mut offset = gap_start;
        for chunk in &chunks[first..] {
            if gap_end - offset > max_size {
                assert_eq!(chunk.offset, offset, "clamp chunk did not start max_size after the previous cut.");
                assert_eq!(chunk.len, max_size, "clamp chunk was not max_size long.");
                assert_eq!(chunk.end, BoundaryKind::Clamp, "expected a Clamp chunk inside the gap.");
                offset += max_size;
            } else {
                assert_eq!(chunk.offset, offset, "last chunk in the gap started in the wrong place.");
                assert_eq!(chunk.end_pos(), gap_end, "last chunk in the gap did not end on the rule boundary.");
                assert_eq!(chunk.end, BoundaryKind::Rule, "last chunk in the gap should end on a rule boundary.");
                break;
            }
        }
    }

    #[test]
    fn test_chunk_exactly_max_size_is_not_clamped() {
        // a run of exactly max_size fits, so it ends on RangeEnd with no clamp
        let finder = Extremum::new(1);
        let max_size = finder.max_chunk_size();
        let data = vec![0xFFu8; max_size];

        let (chunks, stats) = chunk_range(&data, (0..data.len()).into(), &finder);

        assert_eq!(chunks.len(), 1, "expected a single chunk");
        assert_eq!(chunks[0].len, max_size, "chunk was not max_size long.");
        assert_eq!(chunks[0].end, BoundaryKind::RangeEnd, "chunk of exactly max_size should not be clamped.");
        assert_eq!(stats.scan_stats.rule_cuts, 0, "expected no rule cuts.");
        assert_eq!(stats.clamp_cuts, 0, "expected no clamp cuts.");
    }

    #[test]
    fn test_chunk_one_over_max_size_clamps_once() {
        let finder = Extremum::new(1);
        let max_size = finder.max_chunk_size();
        let data = vec![0xFFu8; max_size + 1];

        let (chunks, _stats) = chunk_range(&data, (0..data.len()).into(), &finder);

        assert_eq!(chunks.len(), 2, "expected two chunks");
        assert_eq!((chunks[0].len, &chunks[0].end), (max_size, &BoundaryKind::Clamp), "first chunk should be a full clamp.");
        assert_eq!((chunks[1].len, &chunks[1].end), (1, &BoundaryKind::RangeEnd), "second chunk should be the one leftover byte.");
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
        assert_eq!(stats.scan_stats, ScanStats::empty(), "Expected empty ScanStats for an empty range.");
        assert_eq!(stats.clamp_cuts, 0, "Expected no clamp cuts in empty data.");
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