//! Simple construct of boundary funders for the different tiers in the ladder of chunk sizes.
//! To be used by the resolver so it can easily switch between tiers.

use crate::boundary::BoundaryFinder;

/// Contains the boundary finders for each tier, guaranteed to be in ascending order of mean chunk size.
#[derive(Debug)]
pub struct Ladder<F: BoundaryFinder> {
    /// The finders that finds boundaries for each tier
    pub tiers: Vec<F>,
}

impl<F: BoundaryFinder> Ladder<F> {
    /// Build a ladder with the boundary finders.

    pub fn new(tiers: Vec<F>) -> Self {
        assert!(!tiers.is_empty(), "Cannot create a ladder with no tiers");

        let mut prev = tiers[0].mean_chunk_size();
        for i in 1..tiers.len() {
            let curr = tiers[i].mean_chunk_size();
            assert!(curr > prev);
            prev = curr;
        }

        Self { tiers }
    }
}
