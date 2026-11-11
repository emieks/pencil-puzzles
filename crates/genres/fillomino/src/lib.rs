//! Fillomino: divide the grid into regions and write each region's size in
//! its cells. Regions of the same size never share an edge.
//!
//! The rules, formats and techniques are described in
//! `docs/genres/fillomino.md` in the repository.
//!
//! ```
//! use pencil_puzzles_fillomino::{Budget, GenerateOptions, Puzzle, Solutions};
//!
//! let budget = Budget::unlimited();
//! let options = GenerateOptions::new("5x5".parse().unwrap(), 42);
//! let generated = pencil_puzzles_fillomino::generate(&options, &budget).unwrap();
//! let solutions = pencil_puzzles_fillomino::solve(&generated.puzzle, &budget).unwrap();
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
pub use genre::Fillomino;
pub use model::{Puzzle, Record, parse_records, write_record};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating,
    Solutions, Step, Violation,
};
pub use rate::{HintError, hint, rate, rate_with_steps};
pub use shape::{MAX_SIZE, Params, Shape};
