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
#[derive(Debug)]
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

/// Describes how boundaries can be determined/calculated.
#[derive(Debug)]
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
            Self::LengthMismatch { id, exp, act } => write!(f, "Chunk with id {id} had length of {act} instead of ex0ected length {exp}.")
        }
    }
}

/// Cuts the data into chunks and returns it.
pub fn chunk_range<F : BoundaryFinder>(data: &[u8], range: Range<usize>, finder: &F) -> (Vec<Chunk>, ScanStats) {
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
pub fn reassemble_chunks(chunks: &Vec<Chunk>, data_store: HashMap<ChunkId, Vec<u8>>) -> Result<Vec<u8>, DataReassemblyError> {
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