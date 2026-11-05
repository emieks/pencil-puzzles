//! Solve, generate and rate Kakurasu puzzles.
//!
//! Kakurasu is a grid with a clue for every row and column. Each cell is
//! worth its column number to its row, and its row number to its column,
//! counting from 1. Shade cells so that every row's and column's shaded cells
//! add up to its clue. The rules, formats and techniques are described in
//! `docs/genres/kakurasu.md` in the repository.
//!
//! ```
//! use pencil_puzzles_kakurasu::{Budget, GenerateOptions, Params, Puzzle, Solutions};
//!
//! let budget = Budget::unlimited();
//!
//! // Generate a 6x6 puzzle. The same options always give the same puzzle.
//! let options = GenerateOptions::new("6x6".parse::<Params>().unwrap(), 42);
//! let generated = pencil_puzzles_kakurasu::generate(&options, &budget).unwrap();
//! println!("{}", generated.puzzle.render(None));
//!
//! // Every generated puzzle has exactly one solution.
//! let solutions = pencil_puzzles_kakurasu::solve(&generated.puzzle, &budget).unwrap();
//! assert_eq!(solutions, Solutions::Unique(generated.solution.clone()));
//!
//! // Puzzles can be read from text, or from an ID, then solved or rated.
//! let puzzle: Puzzle = "genre: kakurasu\nrows: 4 5 4 10\ncols: 5 6 7 7".parse().unwrap();
//! assert_eq!(Puzzle::from_id(&puzzle.id()).unwrap(), puzzle);
//! println!("{}", pencil_puzzles_kakurasu::rate(&puzzle, &budget).unwrap().level);
//! ```
//!
//! # Features
//!
//! - `serde`: implements `Serialize` and `Deserialize` for [`Puzzle`],
//!   [`Answer`], [`Shape`], [`Params`] and the shared types.
//!
//! This crate is part of the pencil-puzzles collection, which also has a
//! command-line tool. Most people use it through the `pencil-puzzles` crate,
//! as `pencil_puzzles::kakurasu`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod answer;
mod board;
mod check;
mod generate;
mod human;
mod line;
mod puzzle;
mod rate;
mod shape;
mod solver;

pub use answer::{Answer, Mark};
pub use check::check;
pub use generate::{GENERATOR_VERSION, GenerateError, GenerateOptions, Generated, generate};
pub use pencil_puzzles_core::{
    Budget, BudgetExceeded, Difficulty, Hint, Literal, ParseError, Place, RateError, Rating, Step,
    Violation,
};
pub use puzzle::{Puzzle, parse_records, write_record};
pub use rate::{HintError, RATER_VERSION, hint, rate, rate_with_steps};
pub use shape::{MAX_SIZE, Params, Shape};
pub use solver::{Solutions, count_solutions, solve};

/// A Kakurasu puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

#[cfg(feature = "serde")]
mod json;

mod genre;
pub use genre::Kakurasu;
