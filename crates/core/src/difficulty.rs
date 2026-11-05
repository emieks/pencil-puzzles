//! The difficulty scale shared by every genre.

use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// How hard a puzzle is to solve, on a scale shared by every genre.
///
/// A puzzle's difficulty is the lowest level whose techniques are enough to
/// solve it, always using the easiest technique that makes progress. Each
/// genre lists its techniques and their levels on its genre page; the shared
/// meaning of each level is in `docs/difficulty.md` in the repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Difficulty {
    /// Direct, familiar deductions from a single rule or clue.
    Easy,
    /// Standard combinations and small local patterns.
    Medium,
    /// Advanced patterns or short "if this cell were X, a rule would break"
    /// chains.
    Hard,
    /// Sophisticated multi-pattern reasoning and longer chains.
    Expert,
    /// The most demanding supported reasoning. No genre generates it yet.
    Extreme,
}

impl Difficulty {
    /// Every difficulty, from easiest to hardest.
    pub const ALL: [Self; 5] = [
        Self::Easy,
        Self::Medium,
        Self::Hard,
        Self::Expert,
        Self::Extreme,
    ];

    /// The lowercase name, such as `"easy"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
            Self::Expert => "expert",
            Self::Extreme => "extreme",
        }
    }
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Difficulty {
    type Err = ParseDifficultyError;

    /// Parses a difficulty's name, ignoring case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|difficulty| difficulty.as_str().eq_ignore_ascii_case(s))
            .ok_or_else(|| ParseDifficultyError(s.to_owned()))
    }
}

/// An error from parsing a [`Difficulty`] that isn't one of `easy`,
/// `medium`, `hard`, `expert` or `extreme`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseDifficultyError(String);

impl fmt::Display for ParseDifficultyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown difficulty {:?}; expected easy, medium, hard, expert or extreme",
            self.0
        )
    }
}

impl Error for ParseDifficultyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for difficulty in Difficulty::ALL {
            assert_eq!(difficulty.to_string().parse(), Ok(difficulty));
        }
        assert_eq!("HARD".parse(), Ok(Difficulty::Hard));
        assert!("impossible".parse::<Difficulty>().is_err());
    }

    #[test]
    fn levels_are_ordered() {
        assert!(Difficulty::ALL.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
