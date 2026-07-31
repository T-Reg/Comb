# Comb

**Current Status**: Not Started

Comb is a content-defined chunkcing (CDC) engine and delivery client built with two proven techniques:

1. **Hierarchical nested-boundary chunking** (proven by bup, 2010): chunk boundaries at a configurable number of 
granularity tiers where every boundary is guaranteed to also be a boundary for the tiers above it.
2. **Global cross-corpus deduplication** (proven by Hugging Face Xet): one content-addressed chunk index shared across 
all games and all versions, so shared files, middleware and reused assets are downloaded once per machine.

On top of this engine sits a a delivery client with resumable downloads and progress events, OS-native disk 
pre-allocation, self-repair, and safe extraction so this engine can be used to serve media such as games or VM Images 
efficiently.

Distribution requires nothing but any S3-compatible object store.  No custom server.  More compatibilities may come, but
not planned as of  now.