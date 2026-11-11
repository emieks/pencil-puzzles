//! Nurikabe ratings and hints with human techniques (`docs/genres/nurikabe.md`).
use crate::board::{LAND, SEA};
use crate::human::{PROFILE, analyze_state, initial_masks};
use crate::*;
use pencil_puzzles_core::human;
use std::{error::Error, fmt};
/// Value `v` (0 land, 1 sea) of cell `i`, as players write it.
fn literal(p: &Puzzle, i: usize, v: usize) -> Literal {
    Literal {
        place: Place::Cell {
            row: (i / p.shape().cols()) as u32,
            col: (i % p.shape().cols()) as u32,
        },
        value: if v == 0 { "-" } else { "#" }.into(),
    }
}
fn rate_inner(p: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
    match solve(p, budget)? {
        Solutions::None => return Err(RateError::NoSolution),
        Solutions::Multiple(..) => return Err(RateError::MultipleSolutions),
        Solutions::Unique(_) => {}
    }
    // Binary steps set no placements, so placement metrics would be zeros.
    let trace = analyze_state(p, initial_masks(p), budget, Difficulty::Expert, false)?;
    if !trace.solved {
        return Err(RateError::Unrated);
    }
    let steps = human::public_steps(&trace, initial_masks(p), |i, v| literal(p, i, v));
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
/// A failure to provide a valid next hint.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HintError {
    /// The answer has the wrong shape or disagrees with the solution.
    InvalidAnswer(String),
    /// The puzzle cannot be uniquely rated.
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
impl From<ParseError> for HintError {
    fn from(e: ParseError) -> Self {
        Self::InvalidAnswer(e.to_string())
    }
}
/// The next step for consistent partial marks: the first technique a
/// solver would use, after routine bookkeeping. Hints never reveal the
/// solution; if the supported techniques cannot continue, the puzzle is
/// reported unrated.
pub fn hint(p: &Puzzle, a: &Answer, budget: &Budget) -> Result<Hint, HintError> {
    if a.shape() != p.shape() {
        return Err(HintError::InvalidAnswer(
            "the answer has the wrong shape".into(),
        ));
    }
    let solution = match solve(p, budget)? {
        Solutions::None => return Err(HintError::Rating(RateError::NoSolution)),
        Solutions::Multiple(..) => return Err(HintError::Rating(RateError::MultipleSolutions)),
        Solutions::Unique(a) => a,
    };
    let cols = p.shape().cols();
    let mut masks = initial_masks(p);
    for (i, mask) in masks.iter_mut().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let m = a.get(r, c);
        if m == Mark::Unmarked {
            continue;
        }
        if m != solution.get(r, c) {
            return Err(HintError::InvalidAnswer(format!(
                "the mark in row {}, column {} is wrong",
                r + 1,
                c + 1
            )));
        }
        *mask = if m == Mark::Shaded { SEA } else { LAND };
    }
    if masks.iter().all(|m| m.count_ones() == 1) {
        return Ok(Hint::Solved);
    }
    let trace = analyze_state(p, masks.clone(), budget, Difficulty::Expert, false)?;
    if let Some(step) = human::first_skill_step(&trace, masks.clone(), |i, v| literal(p, i, v)) {
        return Ok(Hint::Step(step));
    }
    // Bookkeeping alone may finish the grid. Otherwise the supported
    // techniques cannot continue, and a hint never reveals the solution.
    if trace.solved
        && let Some(step) = human::public_steps(&trace, masks, |i, v| literal(p, i, v))
            .into_iter()
            .next()
    {
        return Ok(Hint::Step(step));
    }
    Err(HintError::Rating(RateError::Unrated))
}
