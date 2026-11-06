//! Hashi ratings and hints with human techniques (`docs/genres/hashi.md`).
use crate::board::{Layout, solutions};
use crate::human::{PROFILE, analyze_state, initial_masks};
use crate::*;
use pencil_puzzles_core::human;
use std::{error::Error, fmt};

/// Hashi rater version.
pub const RATER_VERSION: u32 = 1;

/// Candidate `v` of slot `s`: `v` bridges between its islands.
fn literal(l: &Layout, s: usize, v: usize) -> Literal {
    Literal {
        place: l.place(s),
        value: v.to_string(),
    }
}

/// The bridges on each slot of the only solution.
fn unique(l: &Layout, budget: &Budget) -> Result<Vec<u8>, RateError> {
    let mut found = solutions(l, budget)?;
    match found.len() {
        0 => Err(RateError::NoSolution),
        1 => Ok(found.remove(0)),
        _ => Err(RateError::MultipleSolutions),
    }
}

fn rate_inner(p: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
    let l = Layout::new(p);
    unique(&l, budget)?;
    let start = initial_masks(&l);
    let trace = analyze_state(&l, start.clone(), budget, Difficulty::Expert, false)?;
    if !trace.solved {
        return Err(RateError::Unrated);
    }
    let steps = human::public_steps(&trace, start, |s, v| literal(&l, s, v));
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

/// The next step for drawn bridges that agree with the solution: the first
/// technique a solver would use, after routine bookkeeping. Bridges not
/// drawn are undecided. Hints never reveal the solution; if the techniques
/// cannot continue, the puzzle is reported unrated.
pub fn hint(p: &Puzzle, a: &Answer, budget: &Budget) -> Result<Hint, HintError> {
    if a.shape() != p.shape() {
        return Err(HintError::InvalidAnswer(
            "the answer has the wrong shape".into(),
        ));
    }
    let l = Layout::new(p);
    let counts = a
        .counts(&l)
        .map_err(|e| HintError::InvalidAnswer(e.message().into()))?;
    let solution = unique(&l, budget)?;
    let mut start = initial_masks(&l);
    for (s, mask) in start.iter_mut().enumerate() {
        if counts[s] == 0 {
            continue;
        }
        if counts[s] != solution[s] {
            return Err(HintError::InvalidAnswer(format!(
                "the bridges {} are wrong",
                l.place(s)
            )));
        }
        *mask = 1 << counts[s];
    }
    if counts == solution {
        return Ok(Hint::Solved);
    }
    let trace = analyze_state(&l, start.clone(), budget, Difficulty::Expert, false)?;
    if let Some(step) = human::first_skill_step(&trace, start.clone(), |s, v| literal(&l, s, v)) {
        return Ok(Hint::Step(step));
    }
    // Bookkeeping alone may finish the puzzle. Otherwise the techniques
    // cannot continue, and a hint never reveals the solution.
    if trace.solved
        && let Some(step) = human::public_steps(&trace, start, |s, v| literal(&l, s, v))
            .into_iter()
            .next()
    {
        return Ok(Hint::Step(step));
    }
    Err(HintError::Rating(RateError::Unrated))
}
