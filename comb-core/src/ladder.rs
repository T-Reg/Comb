//! Simple construct of boundary finders for the different tiers in the ladder of chunk sizes.
//! To be used by the resolver so it can easily switch between tiers.

use crate::boundary::BoundaryFinder;
use std::error::Error;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

/// Contains the boundary finders for each tier, guaranteed to be in strictly ascending order of
/// mean chunk size: index 0 is the finest tier, the last index is the coarsest.
#[derive(Debug)]
pub struct Ladder<F> {
    tiers: Vec<F>,
}

impl<F: BoundaryFinder> Ladder<F> {
    /// Build a ladder with the boundary finders. Must be in strictly ascending order by
    /// `mean_chunk_size`, finest first.
    ///
    /// # Errors
    ///
    /// - `LadderError::Empty` if tiers is empty.
    /// - `LadderError::NotAscending` if any tier's mean chunk size is not strictly larger than
    ///   the one before it.
    pub fn new(tiers: Vec<F>) -> Result<Self, LadderError> {
        if tiers.is_empty() {
            return Err(LadderError::Empty);
        }

        for (i, pair) in tiers.windows(2).enumerate() {
            let prev = pair[0].mean_chunk_size();
            let curr = pair[1].mean_chunk_size();
            if curr <= prev {
                return Err(LadderError::NotAscending {
                    index: i + 1,
                    prev,
                    curr,
                });
            }
        }

        Ok(Self { tiers })
    }

    /// The finders for each tier, finest first.
    #[must_use]
    pub fn tiers(&self) -> &[F] {
        &self.tiers
    }
}

/// Error when building a ladder
#[derive(Debug, PartialEq, Eq)]
pub enum LadderError {
    /// A ladder needs at least one tier.
    Empty,

    /// The tier's mean chunk size was not strictly larger than the previous tier's.
    NotAscending {
        /// index of the offending tier.
        index: usize,
        /// mean chunk size of the tier before it.
        prev: usize,
        /// mean chunk size of the offending tier.
        curr: usize,
    },
}

impl Error for LadderError {}

impl Display for LadderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Cannot create a ladder with no tiers."),
            Self::NotAscending { index, prev, curr } => write!(
                f,
                "Tier {index} has mean chunk size {curr}, which is not larger than the previous \
                 tier's {prev}. Tiers must be in strictly ascending order."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Ladder, LadderError};
    use crate::boundary::BoundaryFinder;
    use crate::boundary::Extremum;

    #[test]
    fn new_valid_ladder() {
        let ladder = Ladder::new(vec![
            Extremum::new(2),
            Extremum::new(32),
            Extremum::new(512),
        ])
        .expect("valid ladder");
        let means: Vec<usize> = ladder
            .tiers()
            .iter()
            .map(BoundaryFinder::mean_chunk_size)
            .collect();
        assert_eq!(means, vec![5, 65, 1025]);
    }

    #[test]
    fn new_single_tier() {
        assert!(Ladder::new(vec![Extremum::new(8)]).is_ok());
    }

    #[test]
    fn new_empty() {
        let tiers: Vec<Extremum> = Vec::new();
        assert_eq!(Ladder::new(tiers).unwrap_err(), LadderError::Empty);
    }

    #[test]
    fn new_descending() {
        let err = Ladder::new(vec![Extremum::new(32), Extremum::new(2)]).unwrap_err();
        assert_eq!(
            err,
            LadderError::NotAscending {
                index: 1,
                prev: 65,
                curr: 5
            }
        );
    }

    #[test]
    fn new_duplicate() {
        let err =
            Ladder::new(vec![Extremum::new(2), Extremum::new(8), Extremum::new(8)]).unwrap_err();
        assert_eq!(
            err,
            LadderError::NotAscending {
                index: 2,
                prev: 17,
                curr: 17
            }
        );
    }
}
