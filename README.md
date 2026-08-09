# Comb

**Current Status**: Early Development

Comb is a content-defined chunking (CDC) engine and delivery client for efficiently distributing large, frequently 
updated binaries such as games, VM images, and ML artifacts.

1. **Hierarchical nested-boundary chunking** (proven by bup, 2010): chunk boundaries at a configurable number of 
granularity tiers where every boundary is guaranteed to also be a boundary for the tiers above it.
2. **Global cross-corpus deduplication** (proven by Hugging Face Xet): one content-addressed chunk index shared across 
all games and all versions, so shared files, middleware and reused assets are downloaded once per machine.

Where Comb differs: resolving a new build works top-down. Big coarse chunks are checked against the index first, and the
search descends to finer granularity only where a chunk isn't already known. Unchanged content costs a handful of cheap 
coarse lookups and a few large index entries, while fine-grained chunks, and the metadata that comes with them, are 
spent only on bytes that actually change. Small patches stay small, manifests stay lean, and stable data stays nearly 
free to re-verify, both at publish time and at repair time.

On top of this engine sits a a delivery client with resumable downloads and progress events, OS-native disk 
pre-allocation, self-repair, and safe extraction so this engine can be used to serve media such as games or VM Images 
efficiently.

Distribution requires nothing but any S3-compatible object store.  No custom server.  More compatibilities may come, but
not planned as of  now.