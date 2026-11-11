//! Kakuro ratings and hints with human techniques (`docs/genres/kakuro.md`).
use crate::human::{DIGITS, Grid, PROFILE, analyze, initial_masks};
use crate::*;
use pencil_puzzles_core::human;
use std::{error::Error, fmt};

/// Candidate `v` of white cell `i`: digit `v + 1`.
fn literal(p: &Puzzle, grid: &Grid, i: usize, v: usize) -> Literal {
    Literal {
        place: crate::board::cell(grid.whites[i], p.shape().cols()),
        value: (v + 1).to_string(),
    }
}

fn unique(p: &Puzzle, budget: &Budget) -> Result<Answer, RateError> {
    match solve(p, budget)? {
        Solutions::None => Err(RateError::NoSolution),
        Solutions::Multiple(..) => Err(RateError::MultipleSolutions),
        Solutions::Unique(a) => Ok(a),
    }
}

fn rate_inner(p: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
    unique(p, budget)?;
    let grid = Grid::new(p);
    let start = initial_masks(&grid);
    let placed = vec![false; start.len()];
    let trace = analyze(&grid, start.clone(), placed, budget, Difficulty::Expert)?;
    if !trace.solved {
        return Err(RateError::Unrated);
    }
    let steps = human::public_steps(&trace, start, |i, v| literal(p, &grid, i, v));
    Ok((human::rating(&trace, &PROFILE), steps))
}

/// Rate a uniquely solvable puzzle by the human techniques it needs, or
/// report it unrated if they cannot finish it.
pub fn rate(p: &Puzzle, budget: &Budget) -> Result<Rating, RateError> {
    rate_inner(p, budget).map(|r| r.0)
}

/// Rate and list the steps.
pub fn rate_with_steps(p: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
    rate_inner(p, budget)
}

/// Why no hint could be given.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HintError {
    /// The answer does not fit the puzzle or disagrees with the solution.
    InvalidAnswer(String),
    /// The puzzle cannot be rated.
    Rating(RateError),
}

impl fmt::Display for HintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAnswer(s) => f.write_str(s),
            Self::Rating(e) => e.fmt(f),
        }
    }
}

impl Error for HintError {}

impl From<BudgetExceeded> for HintError {
    fn from(_: BudgetExceeded) -> Self {
        Self::Rating(RateError::BudgetExceeded)
    }
}

impl From<RateError> for HintError {
    fn from(e: RateError) -> Self {
        Self::Rating(e)
    }
}

/// The next step for partial digits that agree with the solution: the
/// first technique a solver would use, after routine bookkeeping. Hints
/// never reveal the solution; if the techniques cannot continue, the puzzle
/// is reported unrated.
pub fn hint(p: &Puzzle, a: &Answer, budget: &Budget) -> Result<Hint, HintError> {
    if !p.fits(a) {
        return Err(HintError::InvalidAnswer(
            "the answer does not fit the puzzle".into(),
        ));
    }
    let solution = unique(p, budget)?;
    let grid = Grid::new(p);
    let cols = p.shape().cols();
    let mut start = initial_masks(&grid);
    let mut placed = vec![false; start.len()];
    for (k, &i) in grid.whites.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        match a.get(r, c) {
            Mark::Unmarked => {}
            Mark::Digit(d) if solution.get(r, c) == Mark::Digit(d) => {
                start[k] = (1 << (d - 1)) & DIGITS;
                placed[k] = true;
            }
            _ => {
                return Err(HintError::InvalidAnswer(format!(
                    "the mark in row {}, column {} is wrong",
                    r + 1,
                    c + 1
                )));
            }
        }
    }
    if placed.iter().all(|&p| p) {
        return Ok(Hint::Solved);
    }
    let trace = analyze(&grid, start.clone(), placed, budget, Difficulty::Expert)?;
    if let Some(step) =
        human::first_skill_step(&trace, start.clone(), |i, v| literal(p, &grid, i, v))
    {
        return Ok(Hint::Step(step));
    }
    // Bookkeeping alone may finish the grid. Otherwise the techniques
    // cannot continue, and a hint never reveals the solution.
    if trace.solved
        && let Some(step) = human::public_steps(&trace, start, |i, v| literal(p, &grid, i, v))
            .into_iter()
            .next()
    {
        return Ok(Hint::Step(step));
    }
    Err(HintError::Rating(RateError::Unrated))
}
