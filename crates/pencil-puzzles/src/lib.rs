//! Generators, solvers and difficulty raters for pencil puzzles.
//!
//! Each genre is a module, behind a feature of the same name. All genres are
//! enabled by default. To compile fewer genres, turn off default features and
//! enable the ones you need.
//!
//! ```
//! # #[cfg(feature = "kakurasu")] {
//! use pencil_puzzles::Difficulty;
//! use pencil_puzzles::kakurasu::{self, GenerateOptions};
//!
//! let mut options = GenerateOptions::new("6x6".parse().unwrap(), 42);
//! options.difficulty = Some(Difficulty::Hard);
//! let generated = kakurasu::generate(&options, &pencil_puzzles_core::Budget::unlimited()).unwrap();
//! assert_eq!(generated.rating.level, Difficulty::Hard);
//! # }
//! ```
//!
//! # Features
//!
//! - `all-genres` (default): every genre.
//! - One feature per genre, such as `kakurasu`.
//! - `serde`: implements `Serialize` and `Deserialize` for each genre's
//!   puzzles, solutions and ratings.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use pencil_puzzles_core::*;

/// Fillomino: regions numbered with their sizes, same sizes never touching.
#[cfg(feature = "fillomino")]
pub use pencil_puzzles_fillomino as fillomino;

/// Kakurasu: shade cells so that every row and column adds up to its clue.
#[cfg(feature = "kakurasu")]
pub use pencil_puzzles_kakurasu as kakurasu;

mod registry;
pub use registry::{DynGenre, Generated, Input, RegistryError, find, genres, read};

/// Sudoku: fill rows, columns and boxes with distinct digits.
#[cfg(feature = "sudoku")]
pub use pencil_puzzles_sudoku as sudoku;

/// Nurikabe: numbered islands and one connected shaded sea.
#[cfg(feature = "nurikabe")]
pub use pencil_puzzles_nurikabe as nurikabe;
