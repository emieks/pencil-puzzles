//! Canonical human technique profiles; unsupported reasoning remains unrated.
use crate::{
    Answer, Budget, BudgetExceeded, Difficulty, Hint, Literal, Place, Puzzle, RateError, Rating,
    Solutions, Step, solve,
};
use pencil_puzzles_core::human::{self, Trace};
use std::{error::Error, fmt};
/// Current human rater version. Versions 1 and 2 are retired.
pub const RATER_VERSION: u32 = 3;
/// A unique puzzle's human analysis, including an explicit unrated outcome.
#[derive(Clone, Debug)]
pub struct Analysis {
    /// Present only when the supported human repertoire finishes.
    pub rating: Option<Rating>,
    /// Canonical steps with individual pattern premises and dependencies.
    pub steps: Vec<Step>,
    /// Row-major indices of cells still unresolved.
    pub unresolved: Vec<usize>,
}
fn place(p: &Puzzle, i: usize) -> Place {
    Place::Cell {
        row: (i / p.shape().size()) as u32,
        col: (i % p.shape().size()) as u32,
    }
}
fn trace(p: &Puzzle, a: &Trace) -> Vec<Step> {
    trace_from(p, a, crate::human::initial_masks(p))
}
fn literal(p: &Puzzle, i: usize, v: usize) -> Literal {
    Literal {
        place: place(p, i),
        value: (v + 1).to_string(),
    }
}
fn trace_from(p: &Puzzle, a: &Trace, masks: Vec<u16>) -> Vec<Step> {
    human::public_steps(a, masks, |i, v| literal(p, i, v))
}
/// Analyze a uniquely solvable Sudoku without guessing or revealing its solution.
pub fn analyze(puzzle: &Puzzle, budget: &Budget) -> Result<Analysis, RateError> {
    match solve(puzzle, budget)? {
        Solutions::None => return Err(RateError::NoSolution),
        Solutions::Multiple(..) => return Err(RateError::MultipleSolutions),
        Solutions::Unique(_) => {}
    }
    // Easier techniques always come first, so one Expert run gives the same
    // trace as the first of the Easy, Medium, Hard and Expert runs to finish.
    let a = crate::human::analyze(puzzle, budget, Difficulty::Expert, true)?;
    let steps = trace(puzzle, &a);
    let unresolved = a
        .masks
        .iter()
        .enumerate()
        .filter(|(_, m)| m.count_ones() != 1)
        .map(|(i, _)| i)
        .collect();
    let rating = a
        .solved
        .then(|| human::rating(&a, &crate::human::PROFILE, RATER_VERSION));
    Ok(Analysis {
        rating,
        steps,
        unresolved,
    })
}
/// Rate a unique Sudoku, or return Unrated if supported human techniques stall.
pub fn rate(puzzle: &Puzzle, budget: &Budget) -> Result<Rating, RateError> {
    analyze(puzzle, budget)?.rating.ok_or(RateError::Unrated)
}
/// Explicitly reject retired rater versions rather than emulating them.
pub fn rate_version(puzzle: &Puzzle, version: u32, budget: &Budget) -> Result<Rating, RateError> {
    if version != RATER_VERSION {
        return Err(RateError::UnsupportedVersion(version));
    }
    rate(puzzle, budget)
}
/// Rate with the canonical, replayable human explanation trace.
pub fn rate_with_steps(puzzle: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
    let a = analyze(puzzle, budget)?;
    Ok((a.rating.ok_or(RateError::Unrated)?, a.steps))
}
/// A failure to provide a valid human hint.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HintError {
    /// A player mark is inconsistent or has the wrong shape.
    InvalidAnswer(String),
    /// Unique human analysis could not complete.
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
/// Return the next human step; unsupported logic produces Unrated, never a reveal.
pub fn hint(puzzle: &Puzzle, answer: &Answer, budget: &Budget) -> Result<Hint, HintError> {
    if answer.shape() != puzzle.shape() {
        return Err(HintError::InvalidAnswer(
            "the answer has the wrong shape".into(),
        ));
    }
    let solution = match solve(puzzle, budget)? {
        Solutions::None => return Err(HintError::Rating(RateError::NoSolution)),
        Solutions::Multiple(..) => return Err(HintError::Rating(RateError::MultipleSolutions)),
        Solutions::Unique(a) => a,
    };
    let mut masks = crate::human::initial_masks(puzzle);
    // Player marks are written-in digits, like givens: clearing their peers
    // is bookkeeping, not a naked single.
    let mut placed = crate::human::given_cells(puzzle);
    for (i, &v) in answer.cells.iter().enumerate() {
        if v != 0 {
            placed[i] = true;
            if v != solution.cells[i] {
                return Err(HintError::InvalidAnswer(format!(
                    "the mark at cell {i} disagrees with the solution"
                )));
            }
            masks[i] &= 1 << (v - 1);
        }
    }
    if masks.iter().all(|m| m.count_ones() == 1) {
        return Ok(Hint::Solved);
    }
    let initial = masks.clone();
    let choices = crate::human::next_steps(puzzle, &masks, &placed, budget, Difficulty::Expert)?;
    if choices.is_empty() {
        return Err(HintError::Rating(RateError::Unrated));
    }
    let a = crate::human::analyze_state(puzzle, masks, placed, budget, Difficulty::Expert, false)?;
    // Bookkeeping is applied silently: the hint is the first step a solver
    // has to spot. If bookkeeping alone finishes the grid, show it.
    human::first_skill_step(&a, initial.clone(), |i, v| literal(puzzle, i, v))
        .or_else(|| trace_from(puzzle, &a, initial).into_iter().next())
        .map(Hint::Step)
        .ok_or(HintError::Rating(RateError::Unrated))
}

#[cfg(test)]
mod human_rating_tests {
    use super::*;
    #[test]
    fn retired_versions_fail_before_spending_work() {
        let p = Puzzle::new("1x1".parse().unwrap(), vec![0]).unwrap();
        let budget = Budget::new(0);
        for version in [1, 2] {
            assert_eq!(
                rate_version(&p, version, &budget),
                Err(RateError::UnsupportedVersion(version))
            );
        }
        assert_eq!(budget.spent(), 0);
    }
    #[test]
    fn public_trace_replays_and_hints_respect_partial_marks() {
        let p = Puzzle::new(
            "4x4".parse().unwrap(),
            vec![1, 2, 3, 0, 3, 4, 1, 0, 2, 1, 4, 0, 4, 3, 2, 0],
        )
        .unwrap();
        let a = analyze(&p, &Budget::unlimited()).unwrap();
        assert!(a.unresolved.is_empty());
        assert!(a.rating.is_some());
        let mut masks = crate::human::initial_masks(&p);
        for (index, step) in a.steps.iter().enumerate() {
            let proof = step.proof.as_ref().unwrap();
            assert!(proof.predecessors.iter().all(|&i| (i as usize) < index));
            for fact in &proof.facts {
                if let Place::Cell { row, col } = fact.place {
                    let m = fact
                        .candidates
                        .iter()
                        .fold(0u16, |m, v| m | (1 << (v.parse::<usize>().unwrap() - 1)));
                    assert_eq!(masks[row as usize * 4 + col as usize], m);
                }
            }
            for lit in &step.removed {
                if let Place::Cell { row, col } = lit.place {
                    masks[row as usize * 4 + col as usize] &=
                        !(1 << (lit.value.parse::<usize>().unwrap() - 1));
                }
            }
        }
        assert!(masks.iter().all(|m| m.count_ones() == 1));
        let mut answer = Answer::new(p.shape());
        answer.cells[3] = 4;
        if let Hint::Step(step) = hint(&p, &answer, &Budget::unlimited()).unwrap() {
            assert!(
                !step
                    .removed
                    .iter()
                    .any(|l| l.place == place(&p, 3) && l.value == "4")
            );
        }
    }
}
