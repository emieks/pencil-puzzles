//! Hitori: shade cells so that no number repeats among the unshaded cells
//! of a row or column. Shaded cells never touch, and the unshaded cells form
//! one connected group.
//!
//! The rules, formats and techniques are described in
//! `docs/genres/hitori.md` in the repository.
//!
//! ```
//! use pencil_puzzles_hitori::{Budget, GenerateOptions, Puzzle, Solutions};
//!
//! let budget = Budget::unlimited();
//! let options = GenerateOptions::new("5x5".parse().unwrap(), 42);
//! let generated = pencil_puzzles_hitori::generate(&options, &budget).unwrap();
//! let solutions = pencil_puzzles_hitori::solve(&generated.puzzle, &budget).unwrap();
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
pub use genre::Hitori;
pub use model::{Puzzle, Record, parse_records, write_record};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating,
    Solutions, Step, Violation,
};
pub use rate::{HintError, RATER_VERSION, hint, rate, rate_with_steps};
pub use shape::{MAX_SIZE, Params, Shape};
