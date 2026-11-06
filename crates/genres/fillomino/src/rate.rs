//! Fillomino ratings and hints with human techniques (`docs/genres/fillomino.md`).
use crate::human::{PROFILE, analyze_state, bit, initial_masks, solved};
use crate::*;
use pencil_puzzles_core::human;
use std::{error::Error, fmt};

/// Fillomino rater version.
pub const RATER_VERSION: u32 = 1;

/// Candidate `v` of cell `i` as players write it: size `v + 1`, or `16+`
/// for every size from 16 up.
fn literal(p: &Puzzle, i: usize, v: usize) -> Literal {
    Literal {
        place: crate::board::cell(i, p.shape().cols()),
        value: if v == 15 {
            "16+".into()
        } else {
            (v + 1).to_string()
        },
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
    let start = initial_masks(p);
    let trace = analyze_state(p, start.clone(), budget, Difficulty::Expert, false)?;
    // A region of 16 or more cells has no exact size among the candidates.
    if !solved(&trace) {
        return Err(RateError::Unrated);
    }
    let steps = human::public_steps(&trace, start, |i, v| literal(p, i, v));
    Ok((human::rating(&trace, &PROFILE, RATER_VERSION), steps))
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
    /// The answer has the wrong shape or disagrees with the solution.
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

/// The next step for partial numbers that agree with the solution: the
/// first technique a solver would use, after routine bookkeeping. Hints
/// never reveal the solution; if the techniques cannot continue, the puzzle
/// is reported unrated.
pub fn hint(p: &Puzzle, a: &Answer, budget: &Budget) -> Result<Hint, HintError> {
    if a.shape() != p.shape() {
        return Err(HintError::InvalidAnswer(
            "the answer has the wrong shape".into(),
        ));
    }
    let solution = unique(p, budget)?;
    let cols = p.shape().cols();
    let mut start = initial_masks(p);
    for (i, mask) in start.iter_mut().enumerate() {
        let (r, c) = (i / cols, i % cols);
        match a.get(r, c) {
            Mark::Unmarked => {}
            Mark::Number(v) if Mark::Number(v) == solution.get(r, c) => *mask = bit(v),
            _ => {
                return Err(HintError::InvalidAnswer(format!(
                    "the mark in row {}, column {} is wrong",
                    r + 1,
                    c + 1
                )));
            }
        }
    }
    if start.iter().all(|m| m.count_ones() == 1) {
        return Ok(Hint::Solved);
    }
    let trace = analyze_state(p, start.clone(), budget, Difficulty::Expert, false)?;
    if let Some(step) = human::first_skill_step(&trace, start.clone(), |i, v| literal(p, i, v)) {
        return Ok(Hint::Step(step));
    }
    // Bookkeeping alone may finish the grid. Otherwise the techniques
    // cannot continue, and a hint never reveals the solution.
    if solved(&trace)
        && let Some(step) = human::public_steps(&trace, start, |i, v| literal(p, i, v))
            .into_iter()
            .next()
    {
        return Ok(Hint::Step(step));
    }
    Err(HintError::Rating(RateError::Unrated))
}
