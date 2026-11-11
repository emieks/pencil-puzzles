//! Nurikabe: shade a connected sea with no 2x2 pools, so that each number is
//! the size of its island.
//!
//! The rules, formats and techniques are described in
//! `docs/genres/nurikabe.md` in the repository.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
mod answer;
mod bits;
mod board;
mod generate;
mod genre;
mod human;
mod model;
mod rate;
mod shape;
pub use answer::{Answer, Mark};
pub use board::{check, count_solutions, solve};
pub use generate::{GenerateError, GenerateOptions, Generated, generate};
pub use genre::Nurikabe;
pub use model::{Puzzle, Record, parse_records, write_record};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating,
    Solutions, Step, Violation,
};
pub use rate::{HintError, hint, rate, rate_with_steps};
pub use shape::{MAX_SIZE, Params, Shape};

#[cfg(test)]
mod reference_tests;
