//! Generating puzzles with exactly one solution.

use std::error::Error;
use std::fmt;

use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty, RateError, Rating, SplitMix64};

use crate::human::{analyze_state, initial_masks};
use crate::{Answer, Mark, Params, Puzzle};

/// What to generate (P7).
///
/// ```
/// use pencil_puzzles_kakurasu::{Budget, Difficulty, GenerateOptions, Params};
///
/// let mut options = GenerateOptions::new(Params::default(), 1);
/// options.difficulty = Some(Difficulty::Medium);
/// let generated = pencil_puzzles_kakurasu::generate(&options, &Budget::unlimited()).unwrap();
/// assert_eq!(generated.rating.level, Difficulty::Medium);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GenerateOptions {
    /// The parameters, including the shape.
    pub params: Params,
    /// Only accept puzzles of this level. If `None`, accept any puzzle the
    /// human techniques can solve: Easy to Expert (L21).
    pub difficulty: Option<Difficulty>,
    /// The seed. The same options always give the same puzzle (G5).
    pub seed: u64,
    /// How many candidate puzzles to try before giving up.
    pub max_attempts: u32,
}

impl GenerateOptions {
    /// The default for [`max_attempts`](Self::max_attempts).
    pub const DEFAULT_MAX_ATTEMPTS: u32 = 10_000;

    /// Options for a puzzle of any level up to Expert.
    pub fn new(params: Params, seed: u64) -> Self {
        Self {
            params,
            difficulty: None,
            seed,
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
        }
    }
}

/// A generated puzzle (P7).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Generated {
    /// The puzzle. It has exactly one solution.
    pub puzzle: Puzzle,
    /// Its solution.
    pub solution: Answer,
    /// Its rating, the same as [`rate`](crate::rate) gives.
    pub rating: Rating,
    /// The seed it was generated from.
    pub seed: u64,
    /// How many candidate puzzles were tried, including this one.
    pub attempts: u32,
}

/// An error from [`generate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GenerateError {
    /// The request cannot be met, such as an Extreme puzzle.
    UnsupportedRequest(&'static str),
    /// No suitable puzzle was found within the attempt limit.
    AttemptsExhausted {
        /// The number of candidates tried.
        attempts: u32,
    },
    /// The work budget ran out.
    BudgetExceeded,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRequest(reason) => f.write_str(reason),
            Self::AttemptsExhausted { attempts } => write!(
                f,
                "no suitable puzzle found in {attempts} attempts; try a different size or difficulty"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a puzzle was found")
            }
        }
    }
}

impl Error for GenerateError {}

/// Kakurasu candidates are shaded grids; the clues follow from the shading.
struct Shader {
    params: Params,
}

impl Construct for Shader {
    type Candidate = Answer;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, _: &Budget) -> Result<Answer, BudgetExceeded> {
        let shape = self.params.shape;
        Ok(random_answer(rng, shape.rows(), shape.cols()))
    }

    fn puzzle(&self, solution: &Answer) -> Puzzle {
        Puzzle::from_solution(solution)
    }

    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        Ok(analyze_state(p, initial_masks(p), budget, cap, false)?.outcome())
    }

    /// Flip a cell the solver could not work out, since that is where the
    /// ambiguity or difficulty is; if it solved the puzzle too easily, flip
    /// any cell.
    fn change(
        &self,
        solution: &mut Answer,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Answer> {
        let cells = self.params.shape.cells();
        let index = match outcome {
            Outcome::Stuck(open) if !open.is_empty() => open[rng.below(open.len() as u64) as usize],
            _ => rng.below(cells as u64) as usize,
        };
        let cols = self.params.shape.cols();
        let (row, col) = (index / cols, index % cols);
        let mut next = solution.clone();
        next.set(
            row,
            col,
            if solution.is_shaded(row, col) {
                Mark::Unshaded
            } else {
                Mark::Shaded
            },
        );
        Move::Try(next)
    }

    fn patience(&self) -> u32 {
        // About two rejected flips per line before starting over.
        2 * (self.params.shape.rows() + self.params.shape.cols()) as u32
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generates a puzzle with exactly one solution (P7, G2–G5).
///
/// Starts from a randomly shaded grid and screens the puzzle its clues make
/// with the human techniques up to the requested level. A cell the
/// techniques could not work out is flipped (or any cell, if the puzzle was
/// too easy), and the change is kept unless the puzzle got further from the
/// requested level. After a grid's worth of rejected changes it starts over.
/// The returned puzzle is rated in full and checked for uniqueness.
///
/// The attempt limit and budget only decide whether a puzzle is found, not
/// which one: the same options always give the same puzzle (G5). Extreme is
/// not supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    if options.difficulty == Some(Difficulty::Extreme) {
        return Err(GenerateError::UnsupportedRequest(
            "Extreme Kakurasu generation is unavailable: no Extreme human technique profile is supported",
        ));
    }
    let shader = Shader {
        params: options.params,
    };
    match search(
        &shader,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => Ok(Generated {
            puzzle: shader.puzzle(&found.candidate),
            solution: found.candidate,
            rating: found.rating,
            seed: options.seed,
            attempts: found.attempts,
        }),
        Err(SearchError::BudgetExceeded) => Err(GenerateError::BudgetExceeded),
        Err(SearchError::AttemptsExhausted(attempts)) => {
            Err(GenerateError::AttemptsExhausted { attempts })
        }
    }
}

/// A grid with between 35% and 65% of its cells shaded.
fn random_answer(rng: &mut SplitMix64, rows: usize, cols: usize) -> Answer {
    let per_mille = 350 + rng.below(301);
    let shape = crate::Shape::new(rows, cols).expect("the shape is valid");
    let mut answer = Answer::new(shape);
    for row in 0..rows {
        for col in 0..cols {
            let mark = if rng.below(1000) < per_mille {
                Mark::Shaded
            } else {
                Mark::Unshaded
            };
            answer.set(row, col, mark);
        }
    }
    answer
}
