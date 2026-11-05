//! A puzzle together with the shared keys read with it.

use crate::text::SharedKeys;

/// A puzzle as read from text, with its solution and the other shared keys
/// (F6), so that they can be written back unchanged (F8).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Record<P, A> {
    /// The puzzle.
    pub puzzle: P,
    /// The solution recorded with the puzzle, already checked by its genre.
    pub solution: Option<A>,
    /// The other shared keys. Their `solution` is always `None`; the checked
    /// solution is in [`solution`](Self::solution).
    pub shared: SharedKeys,
}

impl<P, A> Record<P, A> {
    /// A record of a puzzle alone.
    pub fn new(puzzle: P) -> Self {
        Self {
            puzzle,
            solution: None,
            shared: SharedKeys::default(),
        }
    }
}
