//! Hashi: join numbered islands with straight bridges, so that each island
//! has as many bridges as its number, no bridges cross and every island is
//! connected.
//!
//! The rules, formats and techniques are described in
//! `docs/genres/hashi.md` in the repository.
//!
//! ```
//! use pencil_puzzles_hashi::{Budget, GenerateOptions, Puzzle, Solutions};
//!
//! let budget = Budget::unlimited();
//! let options = GenerateOptions::new("7x7".parse().unwrap(), 42);
//! let generated = pencil_puzzles_hashi::generate(&options, &budget).unwrap();
//! let solutions = pencil_puzzles_hashi::solve(&generated.puzzle, &budget).unwrap();
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
pub use answer::{Answer, Cell};
pub use board::{check, count_solutions, solve};
pub use generate::{GENERATOR_VERSION, GenerateError, GenerateOptions, Generated, generate};
pub use genre::Hashi;
pub use model::{MAX_NUMBER, Puzzle, Record, parse_records, write_record};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating,
    Solutions, Step, Violation,
};
pub use rate::{HintError, RATER_VERSION, hint, rate, rate_with_steps};
pub use shape::{MAX_SIZE, Params, Shape};
