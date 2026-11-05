//! Ratings shared by every genre (`docs/difficulty.md`).

use std::error::Error;
use std::fmt;

use crate::{BudgetExceeded, Difficulty};

/// The rating of a puzzle: the human techniques its canonical solution
/// path needs (`docs/difficulty.md`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Rating {
    /// The level of the hardest technique needed, Easy to Expert.
    pub level: Difficulty,
    /// The total size of the step explanations. Unstable: it may change
    /// between rater versions and is not a calibrated estimate of solving
    /// time.
    pub score: u64,
    /// The techniques used, excluding routine bookkeeping, in sorted order.
    pub techniques: Vec<String>,
    /// The number of steps, excluding routine bookkeeping.
    pub steps: u32,
    /// The rater version of the genre that produced this rating.
    pub rater_version: u32,
    /// Research measurements of the solution path, for evaluation tools.
    /// Unstable.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub metrics: Option<Metrics>,
}

impl Rating {
    /// A rating at `level` with no techniques, steps or metrics.
    pub fn new(level: Difficulty, rater_version: u32) -> Self {
        Self {
            level,
            score: 0,
            techniques: Vec::new(),
            steps: 0,
            rater_version,
            metrics: None,
        }
    }
}

/// Per-step measurements of a canonical solution path. Unstable; used by
/// evaluation tools, not part of the difficulty definition.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Metrics {
    /// The genre's technique profile, such as `sudoku-human`.
    pub profile: String,
    /// The profile's version.
    pub profile_version: u32,
    /// Direct placements available before each step; zero for bookkeeping.
    /// Empty when not measured.
    pub available_moves: Vec<u32>,
    /// Productive choices of the chosen technique before each step.
    pub productive_choices: Vec<u32>,
    /// Variables with one candidate before each step.
    pub solved_cells: Vec<u32>,
}

/// An error from rating a puzzle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RateError {
    /// The puzzle has no solution.
    NoSolution,
    /// The puzzle has more than one solution.
    MultipleSolutions,
    /// The work budget ran out.
    BudgetExceeded,
    /// The supported human techniques could not complete a unique puzzle.
    Unrated,
    /// A requested historical algorithm is retired or unavailable.
    UnsupportedVersion(u32),
}

impl fmt::Display for RateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::UnsupportedVersion(version) = self {
            return write!(
                f,
                "rater version {version} is retired or unsupported; use the current rater version"
            );
        }
        f.write_str(match self {
            Self::NoSolution => "the puzzle has no solution",
            Self::MultipleSolutions => "the puzzle has more than one solution",
            Self::BudgetExceeded => "the work budget ran out before the rating finished",
            Self::Unrated => "the puzzle is unique but the supported human techniques did not solve it; no difficulty is assigned",
            Self::UnsupportedVersion(_) => unreachable!(),
        })
    }
}

impl Error for RateError {}

impl From<BudgetExceeded> for RateError {
    fn from(_: BudgetExceeded) -> Self {
        Self::BudgetExceeded
    }
}
