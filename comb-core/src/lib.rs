//! Comb is a content-defined chunking (CDC) engine and delivery client for efficiently distributing
//! large, frequently updated binaries such as games, VM images, and ML artifacts.
//! This API includes the chunking algorithm that splits the data into chunks and assigns a hash to
//! each chunk using a fast rolling hash algorithm.

mod chunk;
mod gear;