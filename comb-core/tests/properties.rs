//! Property tests for the chunking layer, ensure the basic properties of the chunker remains true.

use comb_core::boundary::BoundaryFinder;
use comb_core::boundary::extremum::Extremum;
use comb_core::chunk::{BoundaryKind, chunk_range, reassemble_chunks};
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    /// Probably the most important: reassembling the chunks always produce the same data as before
    /// they were chunked.
    #[test]
    fn prop_round_trip_stable(
            data in prop::collection::vec(any::<u8>(), 0..4096),
            h in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>()) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let finder = Extremum::new(h);
        let (chunks, _stats) = chunk_range(&data, range, &finder);

        let mut data_store = HashMap::new();
        for chunk in &chunks {
            data_store.insert(chunk.id, data[chunk.offset..chunk.end_pos()].to_vec());
        }

        // reassemble the chunks and assert the data the same as before
        match reassemble_chunks(&chunks, &data_store) {
            Ok(reassembled) => {
                assert_eq!(
                    &reassembled,
                    &data[range.start..range.end],
                    "reassembled data did not original data."
                );
            }
            Err(e) => {
                panic!("hit unexpected error reassembling the data: {e}");
            }
        }
    }

    // two chunking runs with same data and parameters always produces the same chunks.
    #[test]
    fn prop_determinism(data in prop::collection::vec(any::<u8>(), 0..4096),
            h in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>()) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let finder = Extremum::new(h);
        let (chunks_1, stats_1) = chunk_range(&data, range, &finder);
        let (chunks_2, stats_2) = chunk_range(&data, range, &finder);

        assert_eq!(chunks_1, chunks_2, "Two identical chunking runs produced different output.");
        assert_eq!(stats_1, stats_2, "Two identical chunking runs produced different stats.");
    }

    // all chunks boundaries are spaced at least h + 1 apart.
    #[test]
    fn prop_min_spacing(data in prop::collection::vec(any::<u8>(), 0..4096),
            h in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>()) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let finder = Extremum::new(h);
        let (chunks, _stats) = chunk_range(&data, range, &finder);

        let mut prev = 0usize;
        for chunk in chunks {
            if chunk.end == BoundaryKind::Rule {
                if prev == 0 {
                    prev = chunk.end_pos();
                } else {
                    assert!(h < chunk.end_pos() - prev,
                           "A chunk did not follow the min spacing theorem.");
                    prev = chunk.end_pos();
                }
            }
        }
    }

    // flipping one byte at index i never changes a boundary decision outside [i-h-7, i+h].
    #[test]
    fn prop_edit_locality(data in prop::collection::vec(any::<u8>(), 1..4096),
            h in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>(),
            e in any::<prop::sample::Index>(),
            mask in 1u8..=255) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        // find_boundaries asserts range.end > 0, unlike chunk_range it has no empty range early return.
        prop_assume!(a_i != b_i);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let finder = Extremum::new(h);

        let i = e.index(data.len());
        let mut edited = data.clone();
        edited[i] ^= mask;

        let mut before = Vec::new();
        let mut after = Vec::new();
        let _ = finder.find_boundaries(&data, range, &mut before);
        let _ = finder.find_boundaries(&edited, range, &mut after);

        // byte i is read by the u64 values at i-7..=i, and those values sit in the windows of
        // positions i-7-h..=i+h, so only decisions in that span are allowed to change.
        let affected = i.saturating_sub(h + 7)..=i + h;
        let before: Vec<_> = before.into_iter().filter(|p| !affected.contains(p)).collect();
        let after: Vec<_> = after.into_iter().filter(|p| !affected.contains(p)).collect();

        assert_eq!(before, after, "A one byte edit changed a boundary outside the locality window.");
    }

    // every boundary the finder emits is strictly inside the range: range.start < p < range.end.
    #[test]
    fn prop_endpoint_exclusion(data in prop::collection::vec(any::<u8>(), 0..4096),
            h in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>()) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        // find_boundaries asserts range.end > 0, unlike chunk_range it has no empty range early return.
        prop_assume!(a_i != b_i);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let finder = Extremum::new(h);

        let mut boundaries = Vec::new();
        let _ = finder.find_boundaries(&data, range, &mut boundaries);

        for p in boundaries {
            assert!(range.start < p && p < range.end,
                   "Boundary {} is not strictly inside range {}..{}.", p, range.start, range.end);
        }
    }

    // chunks cover the range exactly: no gaps, no overlaps, no empty chunks. low entropy data is
    // mixed in so clamp cuts show up, uniform random bytes almost never hit the clamp.
    #[test]
    fn prop_tiling(data in prop_oneof![
                prop::collection::vec(any::<u8>(), 0..4096),
                prop::collection::vec(0u8..2, 0..4096),
                prop::collection::vec(Just(0u8), 0..4096),
            ],
            h in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>()) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let finder = Extremum::new(h);
        let (chunks, _stats) = chunk_range(&data, range, &finder);

        if range.start == range.end {
            assert!(chunks.is_empty(), "An empty range produced chunks.");
        } else {
            assert_eq!(chunks.first().unwrap().offset, range.start, "First chunk does not start at range.start.");
            assert_eq!(chunks.last().unwrap().end_pos(), range.end, "Last chunk does not end at range.end.");

            for chunk in &chunks {
                assert!(chunk.len > 0, "Found an empty chunk at offset {}.", chunk.offset);
            }

            for pair in chunks.windows(2) {
                assert_eq!(pair[0].end_pos(), pair[1].offset,
                          "Gap or overlap between chunks at offset {}.", pair[0].end_pos());
            }
        }
    }

    // ensure that for any range [a, b), find_boundaries(data, a..b) is equivalent to
    // find_boundaries(data, 0..data.len()) filtered to [a, b)
    #[test]
    fn prop_descent_equivalence(data in prop_oneof![
                prop::collection::vec(any::<u8>(), 0..4096),
                prop::collection::vec(0u8..2, 0..4096),
                prop::collection::vec(Just(0u8), 0..4096),
            ],
            h in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>()) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let finder = Extremum::new(h);

        let mut boundaries_ab: Vec<usize> = Vec::new();
        finder.find_boundaries(&data, range, &mut boundaries_ab);

        let mut boundaries_full: Vec<usize> = Vec::new();
        finder.find_boundaries(&data, (0..data.len()).into(), &mut boundaries_full);
        let boundaries_filtered: Vec<usize> = boundaries_full.into_iter()
            .filter(|p| range.start < *p && range.end > *p).collect();

        assert_eq!(
            boundaries_ab,
            boundaries_filtered,
            "Expected find boundaries of a..b to be the same as a full boundary scan, filtered to a..b"
        );
    }

    // If we have two h values, h_fine and h_coarse where h_fine < h_coarse, boundaries found by h_coarse are a subset of
    // of those from h_fine.
    #[test]
    fn prop_nested_boundaries(data in prop_oneof![
                prop::collection::vec(any::<u8>(), 0..4096),
                prop::collection::vec(0u8..2, 0..4096),
                prop::collection::vec(Just(0u8), 0..4096),
            ],
            h1 in 0usize..128,
            h2 in 0usize..128,
            a in any::<prop::sample::Index>(),
            b in any::<prop::sample::Index>(),) {
        let a_i = a.index(data.len() + 1);
        let b_i = b.index(data.len() + 1);
        let range = if a_i < b_i { (a_i..b_i).into() } else { (b_i..a_i).into() };
        let h_fine = if h1 < h2 { h1 } else { h2 };
        let h_coarse = if h1 < h2 { h2 } else { h1 };
        let finder_fine = Extremum::new(h_fine);
        let finder_coarse = Extremum::new(h_coarse);

        let mut boundaries_fine: Vec<usize> = Vec::new();
        finder_fine.find_boundaries(&data, range, &mut boundaries_fine);

        let mut boundaries_coarse: Vec<usize> = Vec::new();
        finder_coarse.find_boundaries(&data, range, &mut boundaries_coarse);

        assert!(boundaries_coarse.iter().all(|b| boundaries_fine.contains(b)));
    }
}
