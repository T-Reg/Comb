//! 

trait ChunkIndex {
    fn contains(&self, id: &ChunkId) -> bool;
}