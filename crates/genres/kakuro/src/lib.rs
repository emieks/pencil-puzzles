//! Kakuro: write a digit from 1 to 9 in every white cell so that each run
//! adds up to the sum in the block before it, with no digit repeated in a
//! run.
//!
//! The rules, formats and techniques are described in
//! `docs/genres/kakuro.md` in the repository.
//!
//! ```
//! use pencil_puzzles_kakuro::{Budget, GenerateOptions, Puzzle, Solutions};
//!
//! let budget = Budget::unlimited();
//! let options = GenerateOptions::new("6x6".parse().unwrap(), 42);
//! let generated = pencil_puzzles_kakuro::generate(&options, &budget).unwrap();
//! let solutions = pencil_puzzles_kakuro::solve(&generated.puzzle, &budget).unwrap();
//! assert_eq!(solutions, Solutions::Unique(generated.solution.clone()));
//! assert_eq!(Puzzle::from_id(&generated.puzzle.id()).unwrap(), generated.puzzle);
//! ```
//!
//! # Features
//!
//! - `serde`: implements `Serialize` and `Deserialize` for [`Puzzle`],
//!   [`Answer`], [`Shape`] and [`Params`].
#![forbid(unsafe_code)]
#![warn(missing_docs)]
mod answer;
mod board;
mod generate;
mod genre;
mod human;
mod model;
mod rate;
mod shape;
pub use answer::{Answer, Mark};
pub use board::{check, count_solutions, solve};
pub use generate::{GENERATOR_VERSION, GenerateError, GenerateOptions, Generated, generate};
pub use genre::Kakuro;
pub use model::{Cell, MAX_RUN, MAX_SUM, Puzzle, Record, parse_records, write_record};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating,
    Solutions, Step, Violation,
};
pub use rate::{HintError, RATER_VERSION, hint, rate, rate_with_steps};
pub use shape::{MAX_SIZE, MIN_SIZE, Params, Shape};
