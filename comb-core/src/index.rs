//! The trait that answers the question "Do we already have this chunk?"

use std::collections::HashSet;
use crate::chunk_id::ChunkId;

/// Quick lookup for if we have already seen the given chunk id.
pub trait ChunkIndex {
    /// Is a chunk with exactly these bytes (Blake3 hash) already known?
    fn contains(&self, id: &ChunkId) -> bool;
}

pub struct InMemoryIndex {
    chunk_set: HashSet<ChunkId>,
}

impl InMemoryIndex {
    /// Initialize a new empty index
    pub fn new() -> Self {
        Self { chunk_set: HashSet::<ChunkId>::new() }
    }

    /// Returns true if newly inserted, false if it already was added
    pub fn insert(&mut self, id: ChunkId) -> bool {
        self.chunk_set.insert(id)
    }

    /// The number of unique chunk ids in the index
    pub fn len(&self) -> usize {
        self.chunk_set.len()
    }

    /// True if no chunk ids are in the set.
    pub fn is_empty(&self) -> bool {
        self.chunk_set.is_empty()
    }
}

impl ChunkIndex for InMemoryIndex {
    /// Is a chunk with exactly these bytes (Blake3 hash) already known?
    fn contains(&self, id: &ChunkId) -> bool {
        self.chunk_set.contains(id)
    }
}