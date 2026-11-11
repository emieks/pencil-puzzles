//! Sudoku: fill every row, column and box with each digit once.
//! See `docs/genres/sudoku.md` for rules, formats and the rating model.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
mod board;
mod generate;
mod genre;
mod human;
mod model;
mod rate;
pub use board::{check, count_solutions, solve};
pub use generate::{GenerateError, GenerateOptions, Generated, generate};
pub use genre::Sudoku;
pub use model::{Answer, MAX_SIZE, Params, Puzzle, Record, Shape, parse_records, write_record};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating,
    Solutions, Step, Violation,
};
pub use rate::{Analysis, HintError, analyze, hint, rate, rate_with_steps};
