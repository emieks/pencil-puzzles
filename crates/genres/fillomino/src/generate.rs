//! Reproducible generation: a random partition into regions, then clues
//! removed while the human techniques can still solve the puzzle at the
//! requested level.
use crate::human::{analyze_state, initial_masks, outcome};
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::grid::neighbours;
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::{error::Error, fmt};

/// Generator version (V3).
pub const GENERATOR_VERSION: u32 = 1;

/// Largest region the generator builds.
const LARGEST: usize = 9;
/// Weights of region sizes 1 to 9 when growing a partition; they add up to
/// 32.
const WEIGHTS: [u64; LARGEST] = [1, 3, 5, 6, 6, 5, 3, 2, 1];
/// Rounds of putting clues back before starting from a new partition.
const ROUNDS: u32 = 24;

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
    /// The generator version.
    pub generator_version: u32,
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
                "no Fillomino of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a Fillomino was found")
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

/// A partition being grown: the region of each cell and each region's cells.
struct Partition<'a> {
    nb: &'a [Vec<usize>],
    owner: Vec<Option<usize>>,
    regions: Vec<Vec<usize>>,
}

impl Partition<'_> {
    /// Unowned cells next to `cells`, in index order.
    fn frontier(&self, cells: &[usize]) -> Vec<usize> {
        let mut out: Vec<usize> = cells
            .iter()
            .flat_map(|&i| self.nb[i].iter().copied())
            .filter(|&j| self.owner[j].is_none() && !cells.contains(&j))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Finished regions next to `cells`, in index order.
    fn touching(&self, cells: &[usize]) -> Vec<usize> {
        let mut out: Vec<usize> = cells
            .iter()
            .flat_map(|&i| self.nb[i].iter().filter_map(|&j| self.owner[j]))
            .filter(|&g| !self.regions[g].iter().any(|c| cells.contains(c)))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `cells` as one region would touch a finished region of its size.
    fn clashes(&self, cells: &[usize]) -> bool {
        self.touching(cells)
            .iter()
            .any(|&g| self.regions[g].len() == cells.len())
    }

    fn claim(&mut self, g: usize, cells: &[usize]) {
        for &i in cells {
            self.owner[i] = Some(g);
        }
    }

    /// Grow a region from `start` towards a random size, then make sure it
    /// touches no finished region of the same size. False if that fails.
    fn grow(&mut self, start: usize, rng: &mut SplitMix64) -> bool {
        let mut pick = rng.below(WEIGHTS.iter().sum());
        let target = 1 + WEIGHTS
            .iter()
            .position(|&w| {
                let hit = pick < w;
                pick = pick.saturating_sub(w);
                hit
            })
            .expect("weights cover the range");
        let mut cells = vec![start];
        while cells.len() < target {
            let frontier = self.frontier(&cells);
            if frontier.is_empty() {
                break;
            }
            cells.push(frontier[rng.below(frontier.len() as u64) as usize]);
        }
        if self.clashes(&cells) && cells.len() < LARGEST {
            // One cell more may avoid the clash.
            for j in self.frontier(&cells) {
                cells.push(j);
                if !self.clashes(&cells) {
                    break;
                }
                cells.pop();
            }
        }
        if !self.clashes(&cells) {
            let g = self.regions.len();
            self.claim(g, &cells);
            self.regions.push(cells);
            return true;
        }
        // Otherwise join a neighbouring region, if the result fits.
        for g in self.touching(&cells) {
            let mut merged = self.regions[g].clone();
            merged.extend(&cells);
            if merged.len() <= LARGEST && !self.clashes(&merged) {
                self.claim(g, &cells);
                self.regions[g] = merged;
                return true;
            }
        }
        false
    }
}

/// A random partition with regions of 1 to 9 cells, no two of the same
/// size sharing an edge, as each cell's region size.
fn partition(
    shape: Shape,
    nb: &[Vec<usize>],
    rng: &mut SplitMix64,
    budget: &Budget,
) -> Result<Vec<u32>, BudgetExceeded> {
    let n = shape.cells();
    loop {
        let mut order: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut order);
        let mut p = Partition {
            nb,
            owner: vec![None; n],
            regions: Vec::new(),
        };
        let mut ok = true;
        for &i in &order {
            // Growing a region looks at its frontier about once per cell.
            if !budget.spend((LARGEST * LARGEST) as u64) {
                return Err(BudgetExceeded);
            }
            if p.owner[i].is_none() && !p.grow(i, rng) {
                ok = false;
                break;
            }
        }
        if ok {
            let mut sizes = vec![0; n];
            for region in &p.regions {
                for &i in region {
                    sizes[i] = region.len() as u32;
                }
            }
            return Ok(sizes);
        }
    }
}

/// A candidate: the solution's sizes, the current clues, and the order in
/// which clues are tried for removal.
#[derive(Clone)]
struct Candidate {
    solution: Vec<u32>,
    clues: Vec<u32>,
    order: Vec<usize>,
    next: usize,
    /// The best clue set seen so far, by level then proof size.
    best: Option<(Vec<u32>, (Difficulty, u64))>,
    rounds: u32,
}

struct Digger {
    shape: Shape,
    nb: Vec<Vec<usize>>,
}

impl Digger {
    fn order(&self, rng: &mut SplitMix64) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.shape.cells()).collect();
        rng.shuffle(&mut order);
        order
    }
}

impl Construct for Digger {
    type Candidate = Candidate;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        let solution = partition(self.shape, &self.nb, rng, budget)?;
        Ok(Candidate {
            clues: solution.clone(),
            solution,
            order: self.order(rng),
            next: 0,
            best: None,
            rounds: 0,
        })
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        Puzzle::new(self.shape, c.clues.clone()).expect("generated clues are in range")
    }

    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        Ok(outcome(&analyze_state(
            p,
            initial_masks(p),
            budget,
            cap,
            false,
        )?))
    }

    /// Remove the next clue in the order. When none is left and the puzzle
    /// is still below the requested level, put a few clues back into the
    /// best clue set so far and start a new order.
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
        while c.next < c.order.len() {
            let i = c.order[c.next];
            c.next += 1;
            if c.clues[i] == 0 {
                continue;
            }
            let mut fewer = c.clone();
            fewer.clues[i] = 0;
            return Move::Try(fewer);
        }
        if c.rounds >= ROUNDS {
            return Move::Restart;
        }
        if c.best
            .as_ref()
            .is_none_or(|(_, best)| (level, work) >= *best)
        {
            c.best = Some((c.clues.clone(), (level, work)));
        }
        let mut more = c.clone();
        more.clues = c.best.as_ref().expect("just set").0.clone();
        let mut missing: Vec<usize> = (0..more.clues.len())
            .filter(|&i| more.clues[i] == 0)
            .collect();
        rng.shuffle(&mut missing);
        for &i in missing.iter().take(2 + rng.below(4) as usize) {
            more.clues[i] = more.solution[i];
        }
        more.order = self.order(rng);
        more.next = 0;
        more.rounds += 1;
        Move::Force(more)
    }

    fn finished(&self, c: &Candidate) -> bool {
        // Done when no clue is left to try.
        c.order[c.next..].iter().all(|&i| c.clues[i] == 0)
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

/// Generate with the shared search loop. A random partition into regions of
/// 1 to 9 cells starts with every cell given; clues are removed in a random
/// order while the techniques up to the requested level still solve the
/// puzzle. A puzzle
/// still too easy at the end gets a few clues back and a new order. The
/// returned puzzle is rated in full and checked for uniqueness. Extreme is
/// not supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    if options.difficulty == Some(Difficulty::Extreme) {
        return Err(GenerateError::UnsupportedRequest(
            "Extreme Fillomino generation is unavailable: no Extreme human technique profile is supported",
        ));
    }
    let shape = options.params.shape;
    let digger = Digger {
        shape,
        nb: neighbours(shape.rows(), shape.cols()),
    };
    match search(
        &digger,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => {
            let mut solution = Answer::new(shape);
            for (i, &v) in found.candidate.solution.iter().enumerate() {
                solution.set(i / shape.cols(), i % shape.cols(), Mark::Number(v));
            }
            Ok(Generated {
                puzzle: digger.puzzle(&found.candidate),
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
