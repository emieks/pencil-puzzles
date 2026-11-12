//! Binairo (also known as Takuzu): fill the grid with 0s and 1s so that no
//! three equal digits are next to each other in a line, every row and column
//! holds as many 0s as 1s, and no two rows or two columns are the same.
//!
//! The rules, formats and techniques are described in
//! `docs/genres/binairo.md` in the repository.
//!
//! ```
//! use pencil_puzzles_binairo::{Budget, GenerateOptions, Puzzle, Solutions};
//!
//! let budget = Budget::unlimited();
//! let options = GenerateOptions::new("8x8".parse().unwrap(), 42);
//! let generated = pencil_puzzles_binairo::generate(&options, &budget).unwrap();
//! let solutions = pencil_puzzles_binairo::solve(&generated.puzzle, &budget).unwrap();
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
pub use generate::{GenerateError, GenerateOptions, Generated, generate};
pub use genre::Binairo;
pub use model::{Puzzle, Record, parse_records, write_record};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating,
    Solutions, Step, Violation,
};
pub use rate::{HintError, hint, rate, rate_with_steps};
pub use shape::{MAX_SIZE, Params, Shape};
