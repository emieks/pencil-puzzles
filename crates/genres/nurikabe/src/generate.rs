//! Reproducible connected-sea construction with uniqueness and exact ratings.
use crate::human::{analyze_state, initial_masks};
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::grid::{flood, neighbours};
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::{error::Error, fmt};
/// Generator version (V3).
pub const GENERATOR_VERSION: u32 = 2;
/// Reproducible generator inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GenerateOptions {
    /// Shape and generator parameters.
    pub params: Params,
    /// Exact requested level, or any level from Easy to Expert.
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
    /// The request cannot be met, such as an Extreme puzzle.
    UnsupportedRequest(&'static str),
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
            Self::AttemptsExhausted { attempts } => write!(
                f,
                "no Nurikabe of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a Nurikabe was found")
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
/// A Nurikabe candidate: its shading and which land cell of each island
/// holds the island's number.
#[derive(Clone)]
struct Candidate {
    sea: Vec<bool>,
    numbered: Vec<bool>,
}

struct Grower {
    params: Params,
    neighbours: Vec<Vec<usize>>,
}

/// Tries to find a valid flip before starting over.
const FLIP_TRIES: usize = 8;

impl Grower {
    fn cols(&self) -> usize {
        self.params.shape.cols()
    }

    /// Whether a shading has a nonempty, connected sea, no 2x2 pool, and
    /// (beyond 1x1) at least one island to number.
    fn valid(&self, sea: &[bool]) -> bool {
        let (rows, cols) = (self.params.shape.rows(), self.cols());
        let Some(first) = sea.iter().position(|&s| s) else {
            return false;
        };
        if sea.len() > 1 && sea.iter().all(|&s| s) {
            return false;
        }
        let pool = (0..rows.saturating_sub(1)).any(|r| {
            (0..cols.saturating_sub(1)).any(|c| {
                let k = r * cols + c;
                [k, k + 1, k + cols, k + cols + 1].iter().all(|&j| sea[j])
            })
        });
        !pool
            && flood(first, |j| sea[j], &self.neighbours).len()
                == sea.iter().filter(|&&s| s).count()
    }

    /// Give every island exactly one numbered cell, keeping existing ones
    /// where possible.
    fn renumber(&self, c: &mut Candidate, rng: &mut SplitMix64) {
        let land: Vec<bool> = c.sea.iter().map(|s| !s).collect();
        let mut seen = vec![false; land.len()];
        for i in 0..land.len() {
            if c.sea[i] {
                c.numbered[i] = false;
            }
        }
        for i in 0..land.len() {
            if !land[i] || seen[i] {
                continue;
            }
            let island = flood(i, |j| land[j], &self.neighbours);
            for &j in &island {
                seen[j] = true;
            }
            let marked: Vec<_> = island.iter().copied().filter(|&j| c.numbered[j]).collect();
            match marked[..] {
                [] => c.numbered[island[rng.below(island.len() as u64) as usize]] = true,
                [_] => {}
                [_, ref rest @ ..] => {
                    for &j in rest {
                        c.numbered[j] = false;
                    }
                }
            }
        }
    }
}

impl Construct for Grower {
    type Candidate = Candidate;
    type Puzzle = Puzzle;

    /// Grow a connected, pool-free sea from a random cell.
    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        let cells = self.params.shape.cells();
        let mut sea = vec![false; cells];
        sea[rng.below(cells as u64) as usize] = true;
        let target = (cells * 2 / 5).max(1) + rng.below((cells * 2 / 5 + 1) as u64) as usize;
        for _ in 1..target.min(cells) {
            // Each growth step checks every cell with a flood.
            if !budget.spend(cells as u64) {
                return Err(BudgetExceeded);
            }
            let options: Vec<_> = (0..cells)
                .filter(|&i| {
                    !sea[i] && self.neighbours[i].iter().any(|&j| sea[j]) && {
                        sea[i] = true;
                        let valid = self.valid(&sea);
                        sea[i] = false;
                        valid
                    }
                })
                .collect();
            if options.is_empty() {
                break;
            }
            sea[options[rng.below(options.len() as u64) as usize]] = true;
        }
        let mut candidate = Candidate {
            numbered: vec![false; cells],
            sea,
        };
        self.renumber(&mut candidate, rng);
        Ok(candidate)
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        let land: Vec<bool> = c.sea.iter().map(|s| !s).collect();
        let clues = (0..land.len())
            .map(|i| {
                if c.numbered[i] {
                    flood(i, |j| land[j], &self.neighbours).len() as u32
                } else {
                    0
                }
            })
            .collect();
        Puzzle::new(self.params.shape, clues).expect("island sizes are in range")
    }

    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        Ok(analyze_state(p, initial_masks(p), budget, cap, false)?.outcome())
    }

    /// Flip a cell the solver could not work out (or any cell, if the
    /// puzzle was too easy), keeping the sea valid, then renumber islands.
    fn change(
        &self,
        c: &mut Candidate,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Candidate> {
        let all: Vec<usize> = (0..c.sea.len()).collect();
        let pool = match outcome {
            Outcome::Stuck(open) if !open.is_empty() => open,
            _ => &all,
        };
        for _ in 0..FLIP_TRIES {
            let i = pool[rng.below(pool.len() as u64) as usize];
            let mut next = c.clone();
            next.sea[i] = !next.sea[i];
            if self.valid(&next.sea) {
                self.renumber(&mut next, rng);
                return Move::Try(next);
            }
        }
        Move::Restart
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

/// Generate with the shared search loop: grow a random connected sea, number
/// each island, and screen the puzzle with the human techniques up to the
/// requested level. A cell the techniques could not work out (or any cell,
/// if the puzzle was too easy) is flipped while keeping the sea connected and
/// pool-free; the change is kept unless the puzzle got further from the
/// requested level. The returned puzzle is rated in full and checked for
/// uniqueness. Extreme is not supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    if options.difficulty == Some(Difficulty::Extreme) {
        return Err(GenerateError::UnsupportedRequest(
            "Extreme Nurikabe generation is unavailable: no Extreme human technique profile is supported",
        ));
    }
    let shape = options.params.shape;
    let grower = Grower {
        params: options.params,
        neighbours: neighbours(shape.rows(), shape.cols()),
    };
    match search(
        &grower,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => {
            let mut solution = Answer::new(shape);
            for (i, &s) in found.candidate.sea.iter().enumerate() {
                solution.set(
                    i / shape.cols(),
                    i % shape.cols(),
                    if s { Mark::Shaded } else { Mark::Unshaded },
                );
            }
            Ok(Generated {
                puzzle: grower.puzzle(&found.candidate),
                solution,
                rating: found.rating,
                seed: options.seed,
                generator_version: GENERATOR_VERSION,
                attempts: found.attempts,
            })
        }
        Err(SearchError::BudgetExceeded) => Err(GenerateError::BudgetExceeded),
        Err(SearchError::AttemptsExhausted(attempts)) => {
            Err(GenerateError::AttemptsExhausted { attempts })
        }
    }
}
