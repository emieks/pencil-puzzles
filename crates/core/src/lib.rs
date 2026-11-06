//! Code shared by every pencil-puzzles genre.
//!
//! - [`Difficulty`] and [`Rating`]: the difficulty scale every genre uses
//!   and the ratings genres report (`docs/difficulty.md`).
//! - [`SplitMix64`]: the random number generator every genre's generator
//!   uses, so that a seed gives the same puzzle on every platform.
//! - [`Budget`]: caps on how much work an operation may do.
//! - [`text`] and [`id`]: the text format and puzzle IDs every genre shares.
//! - [`grid`]: neighbours and floods on rectangular grids.
//! - [`Solutions`], [`Violation`], [`Step`]: what solvers, checkers and
//!   raters report.
//! - [`human`]: the human solve loop genres plug their techniques into.
//!
//! # Features
//!
//! - `serde`: implements `Serialize` for the shared types, and `Deserialize`
//!   for [`Difficulty`] and [`Rating`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod budget;
mod difficulty;
mod error;
mod genre;
pub mod grid;
pub mod id;
mod place;
mod rating;
mod record;
mod rng;
mod solutions;
pub mod text;

pub use budget::{Budget, BudgetExceeded};
pub use difficulty::{Difficulty, ParseDifficultyError};
pub use error::{ParseError, Position};
pub use place::{CandidateFact, Hint, Literal, PatternProof, Place, Step, Violation};
pub use rating::{Metrics, RateError, Rating};
pub use record::Record;
pub use rng::SplitMix64;
pub use solutions::Solutions;

pub use genre::{Generation, Genre, Metadata, Rule};

mod view;
pub use view::{ClueInput, MarkOption, PuzzleEditor, View, ViewCell};

#[cfg(feature = "serde")]
pub mod serde_helpers;

pub mod human;
pub mod search;
