//! Seeded clue removal with exact ratings and proven uniqueness (G2-G5).

use crate::{Answer, Budget, BudgetExceeded, Difficulty, Params, Puzzle, Rating};
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::{error::Error, fmt};
/// Generator version (V3).
pub const GENERATOR_VERSION: u32 = 4;
/// Reproducible generator inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GenerateOptions {
    /// Current generator version; historical versions are retired.
    pub version: u32,
    /// Shape and generator parameters.
    pub params: Params,
    /// Exact requested level, or any level below Extreme.
    pub difficulty: Option<Difficulty>,
    /// Reproducible seed.
    pub seed: u64,
    /// Maximum candidates screened.
    pub max_attempts: u32,
}
impl GenerateOptions {
    /// Options accepting Easy through Expert, at most 10,000 candidates.
    pub fn new(params: Params, seed: u64) -> Self {
        Self {
            version: GENERATOR_VERSION,
            params,
            difficulty: None,
            seed,
            max_attempts: 10_000,
        }
    }
}
/// A generated puzzle and its proof witness.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Generated {
    /// Unique puzzle.
    pub puzzle: Puzzle,
    /// Its complete solution.
    pub solution: Answer,
    /// Exact canonical rating.
    pub rating: Rating,
    /// Reproduction seed.
    pub seed: u64,
    /// Generator version.
    pub generator_version: u32,
    /// Candidates tried.
    pub attempts: u32,
}
/// Bounded generation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenerateError {
    /// The shape or human reasoning repertoire cannot deliver this request.
    UnsupportedRequest(&'static str),
    /// The requested generator version is retired or unavailable.
    UnsupportedVersion(u32),
    /// No puzzle was found within the attempt limit.
    AttemptsExhausted {
        /// Candidates screened.
        attempts: u32,
    },
    /// The deterministic work limit ran out.
    BudgetExceeded,
}
impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRequest(reason) => f.write_str(reason),
            Self::UnsupportedVersion(v) => write!(
                f,
                "Sudoku generator version {v} is retired or unsupported; use version {GENERATOR_VERSION}"
            ),
            Self::AttemptsExhausted { attempts } => write!(
                f,
                "no Sudoku of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a Sudoku was found")
            }
        }
    }
}
impl Error for GenerateError {}
impl From<BudgetExceeded> for GenerateError {
    fn from(_: BudgetExceeded) -> Self {
        Self::BudgetExceeded
    }
}
fn shuffle<T>(values: &mut [T], rng: &mut SplitMix64) {
    for i in (1..values.len()).rev() {
        values.swap(i, rng.below(i as u64 + 1) as usize);
    }
}
fn complete(
    params: Params,
    rng: &mut SplitMix64,
    budget: &Budget,
) -> Result<Answer, BudgetExceeded> {
    fn fill(
        mut board: crate::board::Board<'_>,
        rng: &mut SplitMix64,
    ) -> Result<Option<Answer>, BudgetExceeded> {
        if !board.budget.spend(1) {
            return Err(BudgetExceeded);
        }
        if !board.propagate()? {
            return Ok(None);
        }
        let Some(cell) = board.branch_cell() else {
            return Ok(Some(board.answer()));
        };
        let mut candidates: Vec<_> = (0..board.topology.n)
            .map(|v| 1u16 << v)
            .filter(|bit| board.masks[cell] & bit != 0)
            .collect();
        shuffle(&mut candidates, rng);
        for bit in candidates {
            let mut next = board.clone();
            next.masks[cell] = bit;
            if let Some(answer) = fill(next, rng)? {
                return Ok(Some(answer));
            }
        }
        Ok(None)
    }
    let empty = Puzzle::new(params.shape, vec![0; params.shape.cells()]).expect("validated shape");
    Ok(fill(crate::board::Board::new(&empty, budget), rng)?
        .expect("every supported empty Sudoku has a solution"))
}
/// Sudoku candidates: a completed grid, its current clues, and the order in
/// which clues (or symmetric clue pairs) are tried for removal.
struct Digger {
    params: Params,
    symmetric: bool,
}

#[derive(Clone)]
struct Candidate {
    solution: Answer,
    givens: Vec<u8>,
    order: Vec<usize>,
    next: usize,
    /// Best clue set seen at Expert, by level then proof size, for restarts nearby.
    best: Option<(Vec<u8>, (Difficulty, u64))>,
    rounds: u32,
}

/// Rounds of putting clues back before starting from a new grid.
const ROUNDS: u32 = 32;

impl Digger {
    fn order(&self, rng: &mut SplitMix64) -> Vec<usize> {
        let cells = self.params.shape.cells();
        let mut order: Vec<_> = (0..if self.symmetric {
            cells.div_ceil(2)
        } else {
            cells
        })
            .collect();
        shuffle(&mut order, rng);
        order
    }
}

impl Construct for Digger {
    type Candidate = Candidate;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        let solution = complete(self.params, rng, budget)?;
        Ok(Candidate {
            givens: solution.cells.clone(),
            solution,
            order: self.order(rng),
            next: 0,
            best: None,
            rounds: 0,
        })
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        Puzzle::new(self.params.shape, c.givens.clone()).expect("validated generated clues")
    }

    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        Ok(crate::human::analyze(p, budget, cap, false)?.outcome())
    }

    fn change(
        &self,
        c: &mut Candidate,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Candidate> {
        let &Outcome::Solved(level, work) = outcome else {
            return Move::Restart;
        };
        let total = c.givens.len();
        while c.next < c.order.len() {
            let i = c.order[c.next];
            c.next += 1;
            let j = if self.symmetric { total - 1 - i } else { i };
            if c.givens[i] == 0 && c.givens[j] == 0 {
                continue;
            }
            let mut fewer = c.clone();
            fewer.givens[i] = 0;
            fewer.givens[j] = 0;
            return Move::Try(fewer);
        }
        // Every clue was tried and the puzzle is still below the cap.
        if self.symmetric || c.rounds >= ROUNDS {
            return Move::Restart;
        }
        if c.best
            .as_ref()
            .is_none_or(|(_, best)| (level, work) >= *best)
        {
            c.best = Some((c.givens.clone(), (level, work)));
        }
        let mut more = c.clone();
        more.givens = c.best.as_ref().expect("just set").0.clone();
        let mut missing: Vec<_> = (0..total).filter(|&i| more.givens[i] == 0).collect();
        shuffle(&mut missing, rng);
        for &i in missing.iter().take(4 + rng.below(5) as usize) {
            more.givens[i] = more.solution.cells[i];
        }
        more.order = self.order(rng);
        more.next = 0;
        more.rounds += 1;
        Move::Force(more)
    }

    fn finished(&self, c: &Candidate) -> bool {
        // Done when no clue is left to try; already-empty cells don't count.
        let total = c.givens.len();
        c.order[c.next..].iter().all(|&i| {
            let j = if self.symmetric { total - 1 - i } else { i };
            c.givens[i] == 0 && c.givens[j] == 0
        })
    }

    fn patience(&self) -> u32 {
        // Rejected removals are normal; a candidate ends when its order does.
        u32::MAX
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(crate::RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop and certify the exact canonical
/// level. Default and Easy-Hard puzzles have rotationally symmetric givens:
/// clues are removed in symmetric pairs while the puzzle stays solvable at
/// the requested level. Expert removes single clues and, when a puzzle stays
/// too easy, puts a few clues back into the best one so far and tries again.
/// Extreme has no supported human repertoire. Only the current generator
/// version is supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    if options.version != GENERATOR_VERSION {
        return Err(GenerateError::UnsupportedVersion(options.version));
    }
    if options.difficulty == Some(Difficulty::Extreme) {
        return Err(GenerateError::UnsupportedRequest(
            "Extreme Sudoku generation is unavailable: no Extreme human technique profile is supported",
        ));
    }
    if options.params.shape.size() == 1
        && options
            .difficulty
            .is_some_and(|level| level != Difficulty::Easy)
    {
        return Err(GenerateError::UnsupportedRequest(
            "a 1x1 Sudoku can only have Easy difficulty",
        ));
    }
    let digger = Digger {
        params: options.params,
        symmetric: options.difficulty != Some(Difficulty::Expert),
    };
    match search(
        &digger,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => Ok(Generated {
            puzzle: digger.puzzle(&found.candidate),
            solution: found.candidate.solution,
            rating: found.rating,
            seed: options.seed,
            generator_version: GENERATOR_VERSION,
            attempts: found.attempts,
        }),
        Err(SearchError::BudgetExceeded) => Err(GenerateError::BudgetExceeded),
        Err(SearchError::AttemptsExhausted(attempts)) => {
            Err(GenerateError::AttemptsExhausted { attempts })
        }
    }
}
