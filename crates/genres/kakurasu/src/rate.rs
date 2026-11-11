//! Rating and hints with Kakurasu's human techniques (`docs/genres/kakurasu.md`).

use pencil_puzzles_core::human;
use pencil_puzzles_core::{Budget, Difficulty, Hint, Literal, Place, RateError, Rating, Step};

use crate::human::{PROFILE, SHADED, UNSHADED, analyze_state, initial_masks};
use crate::solver::solve;
use crate::{Answer, Mark, Puzzle, Solutions};

/// Rates a puzzle by the human techniques it needs.
///
/// ```
/// use pencil_puzzles_kakurasu::{Budget, Difficulty, Puzzle};
///
/// let puzzle: Puzzle = "rows: 4 5 4 10\ncols: 5 6 7 7".parse().unwrap();
/// let rating = pencil_puzzles_kakurasu::rate(&puzzle, &Budget::unlimited()).unwrap();
/// assert!(rating.level <= Difficulty::Expert);
/// ```
pub fn rate(puzzle: &Puzzle, budget: &Budget) -> Result<Rating, RateError> {
    rate_inner(puzzle, budget).map(|(rating, _)| rating)
}

/// Like [`rate`], and also lists the steps.
pub fn rate_with_steps(puzzle: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
    rate_inner(puzzle, budget)
}

fn rate_inner(puzzle: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
    match solve(puzzle, budget)? {
        Solutions::None => return Err(RateError::NoSolution),
        Solutions::Multiple(..) => return Err(RateError::MultipleSolutions),
        Solutions::Unique(_) => {}
    }
    let trace = analyze_state(
        puzzle,
        initial_masks(puzzle),
        budget,
        Difficulty::Expert,
        false,
    )?;
    if !trace.solved {
        return Err(RateError::Unrated);
    }
    let steps = human::public_steps(&trace, initial_masks(puzzle), |i, v| literal(puzzle, i, v));
    Ok((human::rating(&trace, &PROFILE), steps))
}

/// Value `v` (0 shaded, 1 unshaded) of cell `i`, as players write it.
fn literal(puzzle: &Puzzle, i: usize, v: usize) -> Literal {
    Literal {
        place: Place::Cell {
            row: (i / puzzle.cols()) as u32,
            col: (i % puzzle.cols()) as u32,
        },
        value: if v == 0 { "#" } else { "-" }.to_owned(),
    }
}

/// The next step for a player: the first technique a solver would use from
/// the marks so far, after routine bookkeeping. Hints never reveal the
/// solution: if the supported techniques cannot continue, the result is
/// [`HintError::Unrated`].
///
/// Fails if the puzzle doesn't have exactly one solution, or if a marked
/// cell disagrees with the solution.
pub fn hint(puzzle: &Puzzle, answer: &Answer, budget: &Budget) -> Result<Hint, HintError> {
    if answer.shape() != puzzle.shape() {
        return Err(HintError::WrongShape);
    }
    let solution = match solve(puzzle, budget).map_err(|_| HintError::BudgetExceeded)? {
        Solutions::None => return Err(HintError::NoSolution),
        Solutions::Multiple(..) => return Err(HintError::MultipleSolutions),
        Solutions::Unique(solution) => solution,
    };
    let cols = puzzle.cols();
    let mut masks = initial_masks(puzzle);
    for (i, mask) in masks.iter_mut().enumerate() {
        let (row, col) = (i / cols, i % cols);
        match answer.get(row, col) {
            Mark::Unmarked => {}
            mark if mark != solution.get(row, col) => {
                return Err(HintError::Mistake { row, col });
            }
            Mark::Shaded => *mask = SHADED,
            Mark::Unshaded => *mask = UNSHADED,
        }
    }
    if masks.iter().all(|m| m.count_ones() == 1) {
        return Ok(Hint::Solved);
    }
    let trace = analyze_state(puzzle, masks.clone(), budget, Difficulty::Expert, false)
        .map_err(|_| HintError::BudgetExceeded)?;
    if let Some(step) = human::first_skill_step(&trace, masks.clone(), |i, v| literal(puzzle, i, v))
    {
        return Ok(Hint::Step(step));
    }
    // Bookkeeping alone may finish the grid. Otherwise the supported
    // techniques cannot continue, and a hint never reveals the solution.
    if trace.solved
        && let Some(step) = human::public_steps(&trace, masks, |i, v| literal(puzzle, i, v))
            .into_iter()
            .next()
    {
        return Ok(Hint::Step(step));
    }
    Err(HintError::Unrated)
}

/// An error from [`hint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HintError {
    /// The answer has a different shape from the puzzle.
    WrongShape,
    /// The puzzle has no solution.
    NoSolution,
    /// The puzzle has more than one solution.
    MultipleSolutions,
    /// A marked cell, counted from 0, disagrees with the solution.
    Mistake {
        /// The cell's row.
        row: usize,
        /// The cell's column.
        col: usize,
    },
    /// The work budget ran out.
    BudgetExceeded,
    /// The supported human techniques cannot continue from these marks.
    Unrated,
}

impl std::fmt::Display for HintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongShape => f.write_str("the answer is not the same shape as the puzzle"),
            Self::NoSolution => f.write_str("the puzzle has no solution"),
            Self::MultipleSolutions => f.write_str("the puzzle has more than one solution"),
            Self::Mistake { row, col } => write!(
                f,
                "the mark in row {}, column {} is wrong",
                row + 1,
                col + 1
            ),
            Self::BudgetExceeded => f.write_str("the work budget ran out"),
            Self::Unrated => {
                f.write_str("the supported human techniques cannot continue; no hint is given")
            }
        }
    }
}

impl std::error::Error for HintError {}
