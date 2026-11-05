//! Places on a board, and what checkers and raters say about them.

use std::fmt;

/// A place on a rectangular board, counted from 0 (D5). Used by violations
/// (P8) and by the steps a rater lists (L19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Place {
    /// One cell.
    Cell {
        /// The row, counted from 0.
        row: u32,
        /// The column, counted from 0.
        col: u32,
    },
    /// A whole row, counted from 0.
    Row(u32),
    /// A whole column, counted from 0.
    Col(u32),
    /// A numbered region, such as a Sudoku box, counted from 0.
    Region(u32),
    /// The whole board.
    Board,
}

impl fmt::Display for Place {
    /// Shown to people, so rows and columns count from 1 (D5).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cell { row, col } => write!(
                f,
                "row {}, column {}",
                u64::from(*row) + 1,
                u64::from(*col) + 1
            ),
            Self::Row(row) => write!(f, "row {}", u64::from(*row) + 1),
            Self::Col(col) => write!(f, "column {}", u64::from(*col) + 1),
            Self::Region(region) => write!(f, "region {}", u64::from(*region) + 1),
            Self::Board => f.write_str("the whole board"),
        }
    }
}

/// A broken rule instance, found by a checker (P8).
///
/// It holds data, not sentences: tools turn it into a message, so that
/// messages can be translated.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Violation {
    /// The rule's number, as on the genre page (R1 is 1).
    pub rule: u32,
    /// Where the broken rule instance is.
    pub place: Place,
    /// Numbers that explain the violation, by name, such as `("total", 9)`
    /// and `("clue", 7)`. Each genre page documents its names.
    pub values: Vec<(&'static str, i64)>,
}

impl Violation {
    /// A violation of rule `rule` at `place`.
    pub fn new(rule: u32, place: Place, values: Vec<(&'static str, i64)>) -> Self {
        Self {
            rule,
            place,
            values,
        }
    }
}

/// A statement that a variable has a value, such as "the cell in row 1,
/// column 2 is shaded" (a literal, in the terms of `docs/difficulty.md`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Literal {
    /// The variable.
    pub place: Place,
    /// The value, written as in the genre's text format, such as `#`.
    pub value: String,
}

/// One step of a rating, for hints and explanations (L19).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Step {
    /// The level of the technique used.
    pub level: crate::Difficulty,
    /// The technique's stable name, such as `hidden_single`.
    pub technique: String,
    /// The candidates the step removed.
    pub removed: Vec<Literal>,
    /// Pattern premises and earlier deductions supporting this step, when available.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub proof: Option<PatternProof>,
}

/// Candidate information used as a premise of a human pattern.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CandidateFact {
    /// The cell or other variable involved.
    pub place: Place,
    /// All candidates remaining at this point, in canonical order.
    pub candidates: Vec<String>,
}

/// One pattern inference in a replayable proof graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PatternProof {
    /// Exact candidate-state premises; printed clues are root facts.
    pub facts: Vec<CandidateFact>,
    /// Zero-based indices of earlier steps establishing these premises.
    pub predecessors: Vec<u32>,
    /// Size of this pattern's explanation, not a forcing-branch shortcut.
    pub inference_nodes: u32,
}

/// The next step for a player, given their partial answer (P9, L19).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Hint {
    /// The answer is already complete and correct.
    Solved,
    /// The next step a solver would take, after routine bookkeeping.
    Step(Step),
}
