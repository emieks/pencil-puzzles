//! Contracts shared by every genre (I1, P1-P10).
//! Kakurasu and Sudoku exercise both binary and digit-valued answers.
use crate::{
    Budget, BudgetExceeded, Difficulty, Hint, ParseError, RateError, Rating, Record, Solutions,
    Violation, text::Document,
};
use std::{error::Error, fmt::Display, str::FromStr};

/// A numbered rule's short name (P1).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Rule {
    /// The stable rule number, starting at 1.
    pub number: u32,
    /// A short name; the full rule lives on the genre page.
    pub name: &'static str,
}
/// Metadata used by registries and tools (P1).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub struct Metadata {
    /// Canonical lowercase ASCII id (N1).
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Additional accepted names, unique across the registry (N2).
    pub aliases: &'static [&'static str],
    /// One sentence explaining the goal.
    pub summary: &'static str,
    /// Stable rule numbers and short names.
    pub rules: &'static [Rule],
}
impl Metadata {
    /// Creates metadata with no aliases or rules. Set those fields as needed.
    pub const fn new(id: &'static str, name: &'static str, summary: &'static str) -> Self {
        Self {
            id,
            name,
            summary,
            aliases: &[],
            rules: &[],
        }
    }
}
impl Rule {
    /// A numbered short rule name.
    pub const fn new(number: u32, name: &'static str) -> Self {
        Self { number, name }
    }
}
/// A generated record with complete rating and work statistics (P7).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Generation<P, A> {
    /// Puzzle, checked solution and reproducibility metadata.
    pub record: Record<P, A>,
    /// Full L18 rating.
    pub rating: Rating,
    /// Number of candidates tried, including the returned candidate.
    pub attempts: u32,
}
impl<P, A> Generation<P, A> {
    /// A generation result. The record must contain its solution and F6 metadata.
    pub fn new(record: Record<P, A>, rating: Rating, attempts: u32) -> Self {
        Self {
            record,
            rating,
            attempts,
        }
    }
}
/// The typed interface implemented by every genre (I1).
///
/// Parsing performs validation only, without solving or rating. Algorithms
/// follow G1-G10 and accept deterministic work budgets. Core imposes no serde
/// bounds; JSON adaptation belongs to the umbrella crate. Coordinates in all
/// returned data start at 0. Implementations must not perform I/O.
pub trait Genre: 'static {
    /// Grid allocation limits enforced before text blocks are copied.
    fn text_limits() -> crate::text::GridLimits {
        crate::text::GridLimits::default()
    }

    /// Validated shape, with stable ID encoding and a valid default (P2).
    type Shape: Clone + Default + Display + FromStr<Err = ParseError>;
    /// Validated generation parameters with stable text encoding (P2).
    type Params: Clone + Default + Display + FromStr<Err = ParseError>;
    /// Validated puzzle clues (P3).
    type Puzzle: Clone;
    /// Complete or partial marks with text encoding (P4).
    type Answer: Clone + Display + FromStr<Err = ParseError>;
    /// Actionable generation failure (attempt limit or budget).
    type GenerateError: Error + Send + Sync + 'static;
    /// Actionable hint failure, including inconsistent partial answers.
    type HintError: Error + Send + Sync + 'static;
    /// Immutable metadata, including stable rule numbers (P1).
    fn metadata() -> &'static Metadata;
    /// Convert a validated shape to parameters with default generator settings.
    fn params_for_shape(shape: Self::Shape) -> Self::Params;
    /// Read one shared text document, validating all keys and shared data (F6-F8).
    fn parse_document(document: Document)
    -> Result<Record<Self::Puzzle, Self::Answer>, ParseError>;
    /// Write canonical text, preserving shared metadata and the optional solution.
    fn write_record(record: &Record<Self::Puzzle, Self::Answer>) -> String;
    /// Write the stable, canonical puzzle ID (F9-F12).
    fn id(puzzle: &Self::Puzzle) -> String;
    /// Read a bounded puzzle ID, reporting the first erroneous character.
    fn from_id(id: &str) -> Result<Self::Puzzle, ParseError>;
    /// Read partial marks and reject an answer of the wrong shape.
    fn parse_answer(puzzle: &Self::Puzzle, text: &str) -> Result<Self::Answer, ParseError>;
    /// An answer with no player marks (L2).
    fn empty_answer(puzzle: &Self::Puzzle) -> Self::Answer;
    /// Convert unmarked variables to the genre's finished default (P4).
    fn finished(answer: &Self::Answer) -> Self::Answer;
    /// Prove zero, one or multiple solutions; multiple returns two distinct witnesses (P5).
    fn solve(
        puzzle: &Self::Puzzle,
        budget: &Budget,
    ) -> Result<Solutions<Self::Answer>, BudgetExceeded>;
    /// Rate a uniquely solvable puzzle by L14-L20, or report why it cannot be rated.
    fn rate(puzzle: &Self::Puzzle, budget: &Budget) -> Result<Rating, RateError>;
    /// Rate and list every deduction in canonical order (P9, L19).
    fn rate_with_steps(
        puzzle: &Self::Puzzle,
        budget: &Budget,
    ) -> Result<(Rating, Vec<crate::Step>), RateError>;
    /// Whether `generate` accepts these parameters and level. When false,
    /// `generate` reports an unsupported request. Every level but Extreme by
    /// default.
    fn supports(params: &Self::Params, difficulty: Option<Difficulty>) -> bool {
        let _ = params;
        difficulty != Some(Difficulty::Extreme)
    }
    /// Generate a unique, truly rated puzzle; limits affect success only (P7, G5).
    fn generate(
        params: &Self::Params,
        difficulty: Option<Difficulty>,
        seed: u64,
        max_attempts: u32,
        budget: &Budget,
    ) -> Result<Generation<Self::Puzzle, Self::Answer>, Self::GenerateError>;
    /// Report every broken rule instance as structured data (P8), never sentences.
    fn check(
        puzzle: &Self::Puzzle,
        answer: &Self::Answer,
        budget: &Budget,
    ) -> Result<Vec<Violation>, Box<dyn Error + Send + Sync>>;
    /// Give the first skill step after silent bookkeeping, or solved status (P9).
    fn hint(
        puzzle: &Self::Puzzle,
        answer: &Self::Answer,
        budget: &Budget,
    ) -> Result<Hint, Self::HintError>;
    /// Build a visual clue editor; an existing puzzle supplies its own shape.
    fn editor(shape: Self::Shape, puzzle: Option<&Self::Puzzle>) -> crate::PuzzleEditor;
    /// Validate visual editor fields in their canonical order and make a puzzle.
    fn puzzle_from_editor(
        shape: Self::Shape,
        values: &[String],
    ) -> Result<Self::Puzzle, ParseError>;
    /// Build a cell view with outside clues for a validated answer (P10, I4).
    /// A mismatched answer shape is ignored, preserving the puzzle view.
    fn view(puzzle: &Self::Puzzle, answer: Option<&Self::Answer>) -> crate::View;
    /// Read row-major mark values from a client, validating count and values.
    fn answer_from_values(
        puzzle: &Self::Puzzle,
        values: &[String],
    ) -> Result<Self::Answer, ParseError>;
    /// Draw for terminal output. Any answer must have the puzzle's shape.
    fn render(puzzle: &Self::Puzzle, answer: Option<&Self::Answer>) -> String;
}
