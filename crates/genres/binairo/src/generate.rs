//! Reproducible generation: a random complete grid, then givens removed one
//! at a time while the human techniques up to the requested level still
//! solve the puzzle.
use crate::human::{Grid, analyze_state, initial_masks};
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::{error::Error, fmt};

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

/// A generated puzzle and its solution.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Generated {
    /// The puzzle, with exactly one solution.
    pub puzzle: Puzzle,
    /// Its solution.
    pub solution: Answer,
    /// Its full rating.
    pub rating: Rating,
    /// The seed it was generated from.
    pub seed: u64,
    /// Candidates screened.
    pub attempts: u32,
}

/// Why no puzzle was generated.
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
    /// The work budget ran out.
    BudgetExceeded,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRequest(reason) => f.write_str(reason),
            Self::AttemptsExhausted { attempts } => write!(
                f,
                "no Binairo of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before an Binairo was found")
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

/// The fewest cells a grid needs for Medium generation, and for Hard and
/// Expert: 2x2 grids are always easy, and 4x4 grids at most medium.
const MEDIUM_CELLS: usize = 16;
const HARD_CELLS: usize = 24;

/// Why `generate` turns down a level at a shape, if it does.
pub(crate) fn unsupported(shape: Shape, difficulty: Option<Difficulty>) -> Option<&'static str> {
    match difficulty {
        Some(Difficulty::Extreme) => Some(
            "Extreme Binairo generation is unavailable: no Extreme human technique profile is supported",
        ),
        Some(Difficulty::Hard | Difficulty::Expert) if shape.cells() < HARD_CELLS => {
            Some("Hard and Expert Binairo generation need a grid of at least 24 cells, such as 4x6")
        }
        Some(Difficulty::Medium) if shape.cells() < MEDIUM_CELLS => {
            Some("Medium Binairo generation needs a grid of at least 16 cells, such as 4x4")
        }
        _ => None,
    }
}

/// The most givens removed in one change.
const MAX_BATCH: usize = 32;

/// A candidate: a complete grid, the cells still given, and the order in
/// which givens are tried for removal.
#[derive(Clone)]
struct Candidate {
    solution: Answer,
    givens: Vec<Mark>,
    order: Vec<usize>,
    /// The next place in the order to try.
    next: usize,
    /// How many givens the next change removes.
    batch: usize,
    /// The places in the order of the last change's givens.
    tried: Option<(usize, usize)>,
}

impl Candidate {
    /// Where the order continues: at the last change's givens if they were
    /// removed together and the change was rejected, since each is then
    /// tried alone.
    fn resume(&self) -> usize {
        match self.tried {
            Some((start, end))
                if end - start > 1
                    && self.order[start..end]
                        .iter()
                        .any(|&i| self.givens[i] != Mark::Unmarked) =>
            {
                start
            }
            _ => self.next,
        }
    }
}

struct Digger {
    shape: Shape,
}

impl Construct for Digger {
    type Candidate = Candidate;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        let solution = crate::board::random_grid(self.shape, rng, budget)?
            .expect("every allowed shape has a solution");
        let mut order: Vec<usize> = (0..self.shape.cells()).collect();
        rng.shuffle(&mut order);
        Ok(Candidate {
            givens: solution.marks().to_vec(),
            solution,
            order,
            next: 0,
            batch: 1,
            tried: None,
        })
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        Puzzle::new(self.shape, c.givens.clone()).expect("generated givens fit the shape")
    }

    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        // Most rejected removals leave several solutions, which the exact
        // solver finds faster than the techniques get stuck.
        if crate::board::count_solutions(p, 2, budget)? != 1 {
            let open = (0..p.givens().len())
                .filter(|&i| p.givens()[i] == Mark::Unmarked)
                .collect();
            return Ok(Outcome::Stuck(open));
        }
        Ok(analyze_state(&Grid::new(p), initial_masks(p), budget, cap, false)?.outcome())
    }

    /// Remove the next givens in the candidate's order: one at first, twice
    /// as many after each kept change, up to [`MAX_BATCH`]. After a rejected
    /// change of several givens, each of them is tried alone.
    fn change(
        &self,
        c: &mut Candidate,
        _: &Outcome,
        _: Difficulty,
        _: &mut SplitMix64,
    ) -> Move<Candidate> {
        let resume = c.resume();
        if let Some((start, _)) = c.tried {
            c.batch = if resume == start {
                1
            } else if c.order[start..c.next]
                .iter()
                .all(|&i| c.givens[i] == Mark::Unmarked)
            {
                (2 * c.batch).min(MAX_BATCH)
            } else {
                c.batch
            };
        }
        c.next = resume;
        while c.next < c.order.len() && c.givens[c.order[c.next]] == Mark::Unmarked {
            c.next += 1;
        }
        let start = c.next;
        let mut taken = 0;
        while c.next < c.order.len() && taken < c.batch {
            if c.givens[c.order[c.next]] != Mark::Unmarked {
                taken += 1;
            }
            c.next += 1;
        }
        if taken == 0 {
            return Move::Restart;
        }
        c.tried = Some((start, c.next));
        let mut fewer = c.clone();
        for &i in &c.order[start..c.next] {
            fewer.givens[i] = Mark::Unmarked;
        }
        Move::Try(fewer)
    }

    fn finished(&self, c: &Candidate) -> bool {
        c.order[c.resume()..]
            .iter()
            .all(|&i| c.givens[i] == Mark::Unmarked)
    }

    fn patience(&self) -> u32 {
        // Rejected removals are normal; a candidate ends when its order does.
        u32::MAX
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop. A random complete grid starts with
/// every cell given; givens are removed in a random order, each removal kept
/// if the puzzle keeps one solution and the techniques up to the requested
/// level solve it. When every given has been tried, a puzzle at the
/// requested level is rated in full and returned; one below it starts over
/// from a new grid. Extreme is not supported, Medium needs a grid of at
/// least 16 cells, and Hard and Expert at least 24.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    let shape = options.params.shape;
    if let Some(reason) = unsupported(shape, options.difficulty) {
        return Err(GenerateError::UnsupportedRequest(reason));
    }
    let digger = Digger { shape };
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
            attempts: found.attempts,
        }),
        Err(SearchError::BudgetExceeded) => Err(GenerateError::BudgetExceeded),
        Err(SearchError::AttemptsExhausted(attempts)) => {
            Err(GenerateError::AttemptsExhausted { attempts })
        }
    }
}
