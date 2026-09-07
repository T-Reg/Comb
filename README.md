# Comb

**Current Status**: Early Development.

Comb is a content-defined chunking (CDC) engine and delivery client for efficiently distributing large, frequently
updated binaries such as games, VM images, and ML artifacts.

The engine chunks files across multiple granularity tiers, so a new build can be compared against a known previous
version coarsely first, descending to finer detail only where content has actually changed. Unchanged data stays
cheap to re-verify; changed data is isolated and re-sent precisely, keeping patches and manifests small.

Chunking is hashless: boundaries are derived directly from the content rather than from a rolling hash. This keeps
multiple nested tiers achievable without the normalization machinery rolling-hash chunkers need, and leaves room for
vector/SIMD acceleration. The specifics are still being worked out and aren't public yet.

On top of the engine sits a delivery client: resumable downloads with progress events, OS-native disk
pre-allocation, chunk-precise self-repair, and safe extraction.

Distribution requires nothing but any S3-compatible object store. No custom server. More compatibilities may come,
but none are planned as of now.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
