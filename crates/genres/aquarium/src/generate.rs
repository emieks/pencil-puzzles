//! Reproducible generation: random tanks with random water levels, then
//! changes to the levels and the tank borders until the human techniques
//! solve the puzzle at exactly the requested level.
use crate::human::{Grid, analyze_state, initial_masks};
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::grid::{flood, neighbours};
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
                "no Aquarium of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before an Aquarium was found")
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

/// The fewest cells a grid needs for Expert generation.
const EXPERT_CELLS: usize = 64;

/// Why `generate` turns down a level at a shape, if it does.
pub(crate) fn unsupported(shape: Shape, difficulty: Option<Difficulty>) -> Option<&'static str> {
    match difficulty {
        Some(Difficulty::Extreme) => Some(
            "Extreme Aquarium generation is unavailable: no Extreme human technique profile is supported",
        ),
        Some(Difficulty::Expert) if shape.cells() < EXPERT_CELLS => {
            Some("Expert Aquarium generation needs a grid of at least 64 cells, such as 8x8")
        }
        _ => None,
    }
}

/// Tank sizes the builder aims for, with their weights.
const SIZES: [(usize, u64); 9] = [
    (3, 1),
    (4, 2),
    (5, 4),
    (6, 5),
    (7, 5),
    (8, 4),
    (9, 2),
    (10, 1),
    (11, 1),
];
/// Grids with a smaller mean side scale the tank sizes down in proportion.
const FULL_SIDE: usize = 12;
/// Tanks smaller than this join a neighbouring tank, and border changes
/// never shrink a tank below it.
const SMALLEST: usize = 3;
/// Border changes never grow a tank past this size.
const LARGEST: usize = 12;

/// A random index, each with the chance of its weight. The weights are not
/// all zero.
fn choose(weights: &[u64], rng: &mut SplitMix64) -> usize {
    let mut pick = rng.below(weights.iter().sum());
    for (k, &w) in weights.iter().enumerate() {
        if pick < w {
            return k;
        }
        pick -= w;
    }
    unreachable!("the pick is below the sum of the weights")
}

/// A candidate: each cell's tank and each tank's water line. A cell is
/// water when its row is at or below its tank's line.
#[derive(Clone)]
struct Candidate {
    tank: Vec<u32>,
    line: Vec<usize>,
    /// Changes proposed since the candidate was built.
    edits: u32,
}

struct Builder {
    shape: Shape,
    nb: Vec<Vec<usize>>,
}

impl Builder {
    fn water(&self, c: &Candidate, i: usize) -> bool {
        i / self.shape.cols() >= c.line[c.tank[i] as usize]
    }

    /// The first and last row of a tank.
    fn span(&self, c: &Candidate, tank: u32) -> (usize, usize) {
        let cols = self.shape.cols();
        let mut cells = (0..c.tank.len()).filter(|&i| c.tank[i] == tank);
        let first = cells.next().expect("tanks are never empty");
        (first / cols, cells.next_back().unwrap_or(first) / cols)
    }

    /// Move cell `i` to the tank of a random neighbour, if its own tank
    /// stays in one piece and both tanks stay between [`SMALLEST`] and
    /// [`LARGEST`] cells. Water lines stay within their tanks' rows.
    fn transfer(&self, c: &mut Candidate, i: usize, rng: &mut SplitMix64) -> bool {
        let from = c.tank[i];
        let size = |t: u32| c.tank.iter().filter(|&&u| u == t).count();
        let others: Vec<u32> = (self.nb[i].iter())
            .map(|&j| c.tank[j])
            .filter(|&t| t != from && size(t) < LARGEST)
            .collect();
        let Some(start) = self.nb[i].iter().copied().find(|&j| c.tank[j] == from) else {
            return false;
        };
        if others.is_empty() {
            return false;
        }
        let size = size(from);
        if size <= SMALLEST {
            return false;
        }
        if flood(start, |j| j != i && c.tank[j] == from, &self.nb).len() != size - 1 {
            return false;
        }
        let to = others[rng.below(others.len() as u64) as usize];
        c.tank[i] = to;
        for tank in [from, to] {
            let (top, bottom) = self.span(c, tank);
            c.line[tank as usize] = c.line[tank as usize].clamp(top, bottom + 1);
        }
        true
    }

    /// Move a tank's water line up or down one row, within the tank.
    fn shift(&self, c: &mut Candidate, tank: u32, rng: &mut SplitMix64) {
        let (top, bottom) = self.span(c, tank);
        // Between a third and two thirds of the cells stay water.
        let water = (0..c.tank.len()).filter(|&i| self.water(c, i)).count();
        let up = if 3 * water < c.tank.len() {
            true
        } else if 3 * water > 2 * c.tank.len() {
            false
        } else {
            rng.below(2) == 0
        };
        let line = &mut c.line[tank as usize];
        *line = if (up && *line > top) || *line > bottom {
            *line - 1
        } else {
            *line + 1
        };
    }

    fn solution(&self, c: &Candidate) -> Answer {
        let cols = self.shape.cols();
        let mut a = Answer::new(self.shape);
        for i in 0..c.tank.len() {
            let mark = if self.water(c, i) {
                Mark::Shaded
            } else {
                Mark::Unshaded
            };
            a.set(i / cols, i % cols, mark);
        }
        a
    }
}

impl Construct for Builder {
    type Candidate = Candidate;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        let n = self.shape.cells();
        if !budget.spend(n as u64) {
            return Err(BudgetExceeded);
        }
        // Tanks grow one at a time from the first free cell to a random
        // size. A free cell that touches the tank on one side only is three
        // times as likely to be added, which gives tanks arms and bends.
        let sizes: Vec<u64> = SIZES.iter().map(|s| s.1).collect();
        let side = ((self.shape.rows() + self.shape.cols()) / 2).min(FULL_SIDE);
        let mut tank = vec![u32::MAX; n];
        let mut count = 0;
        for start in 0..n {
            if tank[start] != u32::MAX {
                continue;
            }
            let t = count as u32;
            count += 1;
            let goal = (SIZES[choose(&sizes, rng)].0 * side / FULL_SIDE).max(SMALLEST);
            tank[start] = t;
            let mut frontier: Vec<usize> = (self.nb[start].iter())
                .copied()
                .filter(|&j| tank[j] == u32::MAX)
                .collect();
            for _ in 1..goal {
                if frontier.is_empty() {
                    break;
                }
                let weights: Vec<u64> = (frontier.iter())
                    .map(|&j| {
                        let sides = self.nb[j].iter().filter(|&&k| tank[k] == t).count();
                        if sides == 1 { 3 } else { 1 }
                    })
                    .collect();
                let j = frontier.swap_remove(choose(&weights, rng));
                tank[j] = t;
                for &m in &self.nb[j] {
                    if tank[m] == u32::MAX && !frontier.contains(&m) {
                        frontier.push(m);
                    }
                }
            }
        }
        // A tank smaller than SMALLEST joins its smallest neighbouring tank.
        let mut size = vec![0; count];
        for &t in &tank {
            size[t as usize] += 1;
        }
        loop {
            let small = (0..count as u32).find_map(|t| {
                if size[t as usize] == 0 || size[t as usize] >= SMALLEST {
                    return None;
                }
                (0..n)
                    .filter(|&i| tank[i] == t)
                    .flat_map(|i| self.nb[i].iter().map(|&j| tank[j]))
                    .filter(|&u| u != t)
                    .min_by_key(|&u| size[u as usize])
                    .map(|u| (t, u))
            });
            let Some((from, into)) = small else { break };
            for t in &mut tank {
                if *t == from {
                    *t = into;
                }
            }
            size[into as usize] += size[from as usize];
            size[from as usize] = 0;
        }
        let mut number = vec![u32::MAX; count];
        let mut count = 0;
        for t in &mut tank {
            if number[*t as usize] == u32::MAX {
                number[*t as usize] = count;
                count += 1;
            }
            *t = number[*t as usize];
        }
        let count = count as usize;
        let mut c = Candidate {
            tank,
            line: vec![0; count],
            edits: 0,
        };
        // Random water levels, with between 40% and 60% of the cells water
        // when a few tries allow.
        for _ in 0..8 {
            for t in 0..count {
                let (top, bottom) = self.span(&c, t as u32);
                c.line[t] = top + rng.below((bottom - top + 2) as u64) as usize;
            }
            let water = (0..n).filter(|&i| self.water(&c, i)).count();
            if (2 * n..=3 * n).contains(&(5 * water)) {
                break;
            }
        }
        Ok(c)
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let mut row_clues = vec![0; rows];
        let mut col_clues = vec![0; cols];
        for i in (0..c.tank.len()).filter(|&i| self.water(c, i)) {
            row_clues[i / cols] += 1;
            col_clues[i % cols] += 1;
        }
        Puzzle::new(self.shape, &c.tank, row_clues, col_clues).expect("generated tanks are valid")
    }

    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        Ok(analyze_state(&Grid::new(p), initial_masks(p), budget, cap, false)?.outcome())
    }

    /// Change the water line or the border of a tank at an unresolved
    /// cell, or anywhere when the puzzle solved below the requested level.
    /// A candidate is rebuilt after twice as many changes as it has cells.
    fn change(
        &self,
        c: &mut Candidate,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Candidate> {
        let n = self.shape.cells();
        let focus = match outcome {
            Outcome::Stuck(cells) if !cells.is_empty() => {
                cells[rng.below(cells.len() as u64) as usize]
            }
            _ => rng.below(n as u64) as usize,
        };
        c.edits += 1;
        if c.edits > 2 * n as u32 {
            return Move::Restart;
        }
        let mut next = c.clone();
        if rng.below(2) == 0 || !self.transfer(&mut next, focus, rng) {
            self.shift(&mut next, c.tank[focus], rng);
        }
        Move::Try(next)
    }

    /// Solved candidates climb: a change is kept if the level rises, or
    /// stays and the proof grows no smaller.
    fn keep(&self, before: &Outcome, after: &Outcome) -> bool {
        if let (Outcome::Solved(l1, w1), Outcome::Solved(l2, w2)) = (before, after) {
            return (l2, w2) >= (l1, w1);
        }
        after.rank() >= before.rank()
    }

    fn patience(&self) -> u32 {
        2 * (self.shape.rows() + self.shape.cols()) as u32
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop. Random tanks get random water
/// levels, and the clues count the water. Water lines and tank borders are
/// changed until the techniques up to the requested level solve the puzzle.
/// The returned puzzle is rated in full and checked for uniqueness. Extreme
/// is not supported, and Expert needs a grid of at least 64 cells.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    let shape = options.params.shape;
    if let Some(reason) = unsupported(shape, options.difficulty) {
        return Err(GenerateError::UnsupportedRequest(reason));
    }
    let builder = Builder {
        shape,
        nb: neighbours(shape.rows(), shape.cols()),
    };
    match search(
        &builder,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => Ok(Generated {
            puzzle: builder.puzzle(&found.candidate),
            solution: builder.solution(&found.candidate),
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
