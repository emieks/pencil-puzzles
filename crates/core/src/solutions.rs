//! What a solver found.

/// The solutions of a puzzle, as far as uniqueness is concerned (P5).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Solutions<A> {
    /// The puzzle has no solution.
    None,
    /// The puzzle has exactly one solution.
    Unique(A),
    /// The puzzle has more than one solution. Two different ones are given.
    Multiple(A, A),
}

impl<A> Solutions<A> {
    /// Whether the puzzle has exactly one solution.
    pub fn is_unique(&self) -> bool {
        matches!(self, Self::Unique(_))
    }

    /// A solution, if the puzzle has any.
    pub fn any(&self) -> Option<&A> {
        match self {
            Self::None => None,
            Self::Unique(solution) | Self::Multiple(solution, _) => Some(solution),
        }
    }

    /// Applies `f` to every solution.
    pub fn map<B>(self, mut f: impl FnMut(A) -> B) -> Solutions<B> {
        match self {
            Self::None => Solutions::None,
            Self::Unique(a) => Solutions::Unique(f(a)),
            Self::Multiple(a, b) => Solutions::Multiple(f(a), f(b)),
        }
    }
}
