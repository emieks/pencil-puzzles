//! Reproducible generation: a random layout of blocks, random digits, and
//! digits changed until the human techniques solve the puzzle at the
//! requested level.
use crate::human::screen;
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::grid::{flood, neighbours};
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::{error::Error, fmt};

/// Generator version (V3).
pub const GENERATOR_VERSION: u32 = 1;

/// The longest run the generator builds, by level as for [`DENSITY`].
const LONGEST: [usize; 5] = [6, 5, 5, 6, 7];
/// Percent of the inner cells (below the first row, right of the first
/// column) made blocks, by level: any, easy, medium, hard, expert.
const DENSITY: [usize; 5] = [25, 30, 25, 22, 20];
/// How much the digit order varies from each run's preference for low or
/// high digits, by level as for [`DENSITY`]. Small values give sums with
/// few combinations.
const NOISE: [u64; 5] = [8, 3, 6, 10, 16];
/// The most cells one new block may turn into blocks while the layout is
/// built or changed, counting cells left in runs of one cell.
const CASCADE: usize = 3;
/// When stuck, one change in this many makes the chosen cell a block, while
/// the blocks stay within twenty points of the level's density.
const BLOCK_CHANCE: u64 = 4;
/// Cells tried when filling in digits before starting a new layout.
const FILL_NODES: u32 = 20_000;
/// How many times [`Construct::patience`] changes a stuck candidate gets
/// without improving before starting over.
const STUCK_PATIENCE: u32 = 3;
/// How many times longer than [`Construct::patience`] a solved candidate
/// that is too easy is changed before starting over.
const SOLVED_PATIENCE: u32 = 8;
/// Unresolved cells compared when choosing one to change.
const SAMPLE: u32 = 3;
/// One more cell is changed at once for every this many unresolved cells.
const SPREAD: usize = 16;
/// Cells tried for one change before starting over.
const TRIES: u32 = 8;

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
                "no Kakuro of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a Kakuro was found")
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

/// A candidate: which cells are white, the solution's digits (0 on
/// blocks), the best outcome rank so far and the changes since it.
#[derive(Clone)]
struct Candidate {
    white: Vec<bool>,
    digits: Vec<u8>,
    best: (bool, i64),
    stale: u32,
}

struct Builder {
    shape: Shape,
    density: usize,
    noise: u64,
    longest: usize,
    nb: Vec<Vec<usize>>,
    /// The number of digit sets of each size and sum: `ways[size][sum]`.
    ways: [[u32; 46]; 10],
}

impl Builder {
    fn spend(budget: &Budget, n: usize) -> Result<(), BudgetExceeded> {
        if budget.spend(n as u64) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    /// The cells of the run through white cell `i`, in order.
    fn run(&self, white: &[bool], i: usize, down: bool) -> Vec<usize> {
        let cols = self.shape.cols();
        let step = if down { cols } else { 1 };
        let same_line = |a: usize, b: usize| down || a / cols == b / cols;
        let mut start = i;
        while start >= step && same_line(start - step, i) && white[start - step] {
            start -= step;
        }
        let mut out = Vec::new();
        let mut j = start;
        while j < white.len() && same_line(j, i) && white[j] {
            out.push(j);
            j += step;
        }
        out
    }

    /// Make `i` a block, then every white cell left in a run of one cell,
    /// and so on; returns the cells made blocks. Undone, returning `None`,
    /// if fewer than four white cells would remain or, when `small`, if
    /// more than [`CASCADE`] cells change or the white cells would no
    /// longer be connected.
    fn block(
        &self,
        white: &mut [bool],
        i: usize,
        whites: &mut usize,
        small: bool,
    ) -> Option<Vec<usize>> {
        let mut changed = Vec::new();
        let mut stack = vec![i];
        while let Some(j) = stack.pop() {
            if !white[j] {
                continue;
            }
            white[j] = false;
            changed.push(j);
            for &k in &self.nb[j] {
                if white[k]
                    && (self.run(white, k, false).len() == 1 || self.run(white, k, true).len() == 1)
                {
                    stack.push(k);
                }
            }
        }
        let left = whites.saturating_sub(changed.len());
        let fails = left < 4
            || (small
                && (changed.len() > CASCADE || {
                    let start = (0..white.len()).find(|&j| white[j]);
                    start.is_none_or(|s| flood(s, |j| white[j], &self.nb).len() != left)
                }));
        if fails {
            for j in changed {
                white[j] = true;
            }
            return None;
        }
        *whites = left;
        Some(changed)
    }

    /// A random layout: inner cells start white and some become blocks,
    /// then long runs are broken. `None` if a long run cannot be broken.
    fn layout(
        &self,
        rng: &mut SplitMix64,
        budget: &Budget,
    ) -> Result<Option<Vec<bool>>, BudgetExceeded> {
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let n = self.shape.cells();
        if rows < 3 || cols < 3 {
            // Too small for a run of two cells.
            return Ok(Some(vec![false; n]));
        }
        let mut white: Vec<bool> = (0..n).map(|i| i / cols > 0 && i % cols > 0).collect();
        let inner = (rows - 1) * (cols - 1);
        let mut whites = inner;
        let target = inner * self.density / 100;
        let mut order: Vec<usize> = (0..n).filter(|&i| white[i]).collect();
        rng.shuffle(&mut order);
        for &i in &order {
            if inner - whites >= target {
                break;
            }
            Self::spend(budget, 4 * rows.max(cols))?;
            if white[i] {
                self.block(&mut white, i, &mut whites, true);
            }
        }
        loop {
            let long = (0..n).find_map(|i| {
                let first = |down: bool| {
                    let step = if down { cols } else { 1 };
                    i < step || !white[i - step] || (!down && i % cols == 0)
                };
                [false, true].into_iter().find_map(|down| {
                    (white[i] && first(down))
                        .then(|| self.run(&white, i, down))
                        .filter(|r| r.len() > self.longest)
                })
            });
            Self::spend(budget, n)?;
            let Some(mut long) = long else {
                return Ok(Some(white));
            };
            rng.shuffle(&mut long);
            // A small change if possible, any change otherwise.
            if ![true, false].iter().any(|&small| {
                long.iter()
                    .any(|&i| self.block(&mut white, i, &mut whites, small).is_some())
            }) {
                return Ok(None);
            }
        }
    }

    /// Random digits with no repeat in a run, row-major by backtracking
    /// with an explicit stack. Each run prefers low or high digits at
    /// random, and each cell tries digits in the order its two runs prefer,
    /// varied by [`NOISE`]. `None` after [`FILL_NODES`] cells.
    fn fill(
        &self,
        white: &[bool],
        rng: &mut SplitMix64,
        budget: &Budget,
    ) -> Result<Option<Vec<u8>>, BudgetExceeded> {
        let cols = self.shape.cols();
        let cells: Vec<usize> = (0..white.len()).filter(|&i| white[i]).collect();
        if cells.is_empty() {
            return Ok(Some(vec![0; white.len()]));
        }
        let mut digits = vec![0u8; white.len()];
        // Whether the run starting at each cell prefers low digits, for
        // across and down runs.
        let low: Vec<[bool; 2]> = (0..white.len())
            .map(|_| [rng.below(2) == 0, rng.below(2) == 0])
            .collect();
        let start = |i: usize, down: bool| {
            let step = if down { cols } else { 1 };
            let mut j = i;
            while (if down {
                j >= cols
            } else {
                !j.is_multiple_of(cols)
            }) && white[j - step]
            {
                j -= step;
            }
            j
        };
        let order = |rng: &mut SplitMix64, i: usize| {
            let lows = [low[start(i, false)][0], low[start(i, true)][1]];
            let mut o = [1u8, 2, 3, 4, 5, 6, 7, 8, 9].map(|d| {
                let pull: u64 = lows
                    .iter()
                    .map(|&l| u64::from(if l { d } else { 10 - d }))
                    .sum();
                (pull + rng.below(self.noise + 1), d)
            });
            o.sort_unstable();
            o.map(|(_, d)| d)
        };
        // Digits already in the cells before `i` in its two runs.
        let used = |digits: &[u8], i: usize| {
            let mut used = 0u16;
            let mut j = i;
            while !j.is_multiple_of(cols) && white[j - 1] {
                j -= 1;
                used |= 1 << digits[j];
            }
            let mut j = i;
            while j >= cols && white[j - cols] {
                j -= cols;
                used |= 1 << digits[j];
            }
            used
        };
        let mut stack = vec![(0usize, order(rng, cells[0]), 0usize)];
        let mut nodes = 0;
        while let Some(top) = stack.last_mut() {
            let i = cells[top.0];
            digits[i] = 0;
            let used = used(&digits, i);
            let Some(k) = (top.2..9).find(|&k| used & (1 << top.1[k]) == 0) else {
                stack.pop();
                continue;
            };
            digits[i] = top.1[k];
            top.2 = k + 1;
            let next = top.0 + 1;
            if next == cells.len() {
                return Ok(Some(digits));
            }
            nodes += 1;
            Self::spend(budget, 16)?;
            if nodes > FILL_NODES {
                return Ok(None);
            }
            stack.push((next, order(rng, cells[next]), 0));
        }
        Ok(None)
    }

    /// The other cells of `i`'s across and down runs.
    fn mates(&self, c: &Candidate, i: usize) -> [Vec<usize>; 2] {
        [false, true].map(|down| {
            let mut run = self.run(&c.white, i, down);
            run.retain(|&j| j != i);
            run
        })
    }

    /// One change to cell `i`: make it a block (sometimes, when stuck),
    /// give it a digit neither of its runs has, or swap its digit with
    /// another cell of one of its runs where neither digit repeats.
    fn alter(
        &self,
        c: &Candidate,
        i: usize,
        stuck: bool,
        rng: &mut SplitMix64,
    ) -> Option<Candidate> {
        if stuck && rng.below(BLOCK_CHANCE) == 0 {
            let inner = (self.shape.rows() - 1) * (self.shape.cols() - 1);
            let mut whites = c.white.iter().filter(|&&w| w).count();
            let mut next = c.clone();
            if (inner - whites) * 100 < inner * (self.density + 20)
                && let Some(changed) = self.block(&mut next.white, i, &mut whites, true)
            {
                for j in changed {
                    next.digits[j] = 0;
                }
                return Some(next);
            }
        }
        let mates = self.mates(c, i);
        let taken =
            Self::digits_of(c, &mates[0]) | Self::digits_of(c, &mates[1]) | 1 << c.digits[i];
        let free: Vec<u8> = (1..=9).filter(|&d| taken & (1 << d) == 0).collect();
        if !free.is_empty() {
            let mut next = c.clone();
            next.digits[i] = if stuck {
                // The digit leaving the two runs fewest combinations.
                let ways = |d: u8| -> u32 {
                    mates
                        .iter()
                        .map(|run| {
                            let sum: u32 = run.iter().map(|&j| u32::from(c.digits[j])).sum();
                            self.ways[run.len() + 1][(sum + u32::from(d)) as usize]
                        })
                        .sum()
                };
                let start = rng.below(free.len() as u64) as usize;
                let mut best = free[start];
                for k in 1..free.len() {
                    let d = free[(start + k) % free.len()];
                    if ways(d) < ways(best) {
                        best = d;
                    }
                }
                best
            } else {
                free[rng.below(free.len() as u64) as usize]
            };
            return Some(next);
        }
        let mut others: Vec<(usize, usize)> = (0..2)
            .flat_map(|shared| mates[shared].iter().map(move |&j| (shared, j)))
            .collect();
        rng.shuffle(&mut others);
        for (shared, j) in others {
            let mine = &mates[1 - shared];
            let theirs: Vec<usize> = self
                .run(&c.white, j, shared == 0)
                .into_iter()
                .filter(|&k| k != j)
                .collect();
            let (di, dj) = (c.digits[i], c.digits[j]);
            if Self::digits_of(c, mine) & (1 << dj) == 0
                && Self::digits_of(c, &theirs) & (1 << di) == 0
            {
                let mut next = c.clone();
                next.digits.swap(i, j);
                return Some(next);
            }
        }
        None
    }

    fn digits_of(c: &Candidate, cells: &[usize]) -> u16 {
        cells.iter().fold(0, |m, &j| m | 1 << c.digits[j])
    }
}

impl Construct for Builder {
    type Candidate = Candidate;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        loop {
            let Some(white) = self.layout(rng, budget)? else {
                continue;
            };
            if let Some(digits) = self.fill(&white, rng, budget)? {
                return Ok(Candidate {
                    white,
                    digits,
                    best: (false, i64::MIN),
                    stale: 0,
                });
            }
        }
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        let cols = self.shape.cols();
        let n = c.white.len();
        let sum = |i: usize, down: bool| -> Option<u8> {
            let next = if down { i + cols } else { i + 1 };
            let fits = if down { next < n } else { next % cols != 0 };
            (fits && c.white[next]).then(|| {
                self.run(&c.white, next, down)
                    .iter()
                    .map(|&j| c.digits[j])
                    .sum()
            })
        };
        let cells = (0..n)
            .map(|i| {
                if c.white[i] {
                    Cell::White
                } else {
                    Cell::Block {
                        down: sum(i, true),
                        across: sum(i, false),
                    }
                }
            })
            .collect();
        Puzzle::new(self.shape, cells).expect("generated layouts are valid")
    }

    /// Above Medium, a puzzle that Medium techniques do not solve is tried
    /// with the chains only if it has one solution; otherwise it is stuck
    /// with the cells Medium techniques left.
    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        if cap <= Difficulty::Medium {
            return screen(p, budget, cap);
        }
        let medium = screen(p, budget, Difficulty::Medium)?;
        if matches!(medium, Outcome::Solved(..)) || !crate::solve(p, budget)?.is_unique() {
            return Ok(medium);
        }
        screen(p, budget, cap)
    }

    /// Change a digit: of a cell left unresolved, or of any cell when the
    /// puzzle is solved below the requested level. The new digit is one
    /// that neither of the cell's runs has; if there is none, the digit is
    /// swapped with another cell of one of its runs where neither digit
    /// repeats.
    fn change(
        &self,
        c: &mut Candidate,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Candidate> {
        if outcome.rank() > c.best {
            c.best = outcome.rank();
            c.stale = 0;
        } else {
            // A solved candidate is kept longer, since only some changes
            // make it harder.
            let limit = match outcome {
                Outcome::Solved(..) => SOLVED_PATIENCE * self.patience(),
                Outcome::Stuck(_) => STUCK_PATIENCE * self.patience(),
            };
            c.stale += 1;
            if c.stale > limit {
                return Move::Restart;
            }
        }
        let whites: Vec<usize> = (0..c.white.len()).filter(|&i| c.white[i]).collect();
        let pool: Vec<usize> = match outcome {
            Outcome::Stuck(vars) if !vars.is_empty() => vars.iter().map(|&v| whites[v]).collect(),
            _ => whites,
        };
        if pool.is_empty() {
            return Move::Restart;
        }
        // The combinations of the two runs through cell `i`.
        let open = |i: usize| -> u32 {
            [false, true]
                .iter()
                .map(|&down| {
                    let run = self.run(&c.white, i, down);
                    let sum: u32 = run.iter().map(|&j| u32::from(c.digits[j])).sum();
                    self.ways[run.len()][sum as usize]
                })
                .sum()
        };
        let stuck = matches!(outcome, Outcome::Stuck(_));
        // Several far-apart cells may need changes at once.
        let edits = if stuck { 1 + pool.len() / SPREAD } else { 1 };
        let mut next: Option<Candidate> = None;
        let mut done = 0;
        for _ in 0..TRIES * edits as u32 {
            // When stuck, the cell of [`SAMPLE`] random ones whose runs
            // have the most combinations.
            let mut i = pool[rng.below(pool.len() as u64) as usize];
            if stuck {
                for _ in 1..SAMPLE {
                    let j = pool[rng.below(pool.len() as u64) as usize];
                    if open(j) > open(i) {
                        i = j;
                    }
                }
            }
            let base = next.as_ref().unwrap_or(c);
            if !base.white[i] {
                continue;
            }
            if let Some(changed) = self.alter(base, i, stuck, rng) {
                next = Some(changed);
                done += 1;
                if done == edits {
                    break;
                }
            }
        }
        next.map_or(Move::Restart, Move::Try)
    }

    fn patience(&self) -> u32 {
        ((self.shape.rows() - 1) * (self.shape.cols() - 1)) as u32
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop. A random layout of blocks gets
/// random digits; while the techniques up to the requested level cannot
/// solve the puzzle, digits of unresolved cells change, and while it is
/// too easy, digits of any cells change. The returned puzzle is rated in
/// full and checked for uniqueness. Extreme is not supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    let level = match options.difficulty {
        None => 0,
        Some(Difficulty::Extreme) => {
            return Err(GenerateError::UnsupportedRequest(
                "Extreme Kakuro generation is unavailable: no Extreme human technique profile is supported",
            ));
        }
        Some(level) => 1 + level as usize,
    };
    let shape = options.params.shape;
    let mut ways = [[0u32; 46]; 10];
    for set in 0..512u32 {
        let sum: u32 = (0..9).filter(|k| set & (1 << k) != 0).map(|k| k + 1).sum();
        ways[set.count_ones() as usize][sum as usize] += 1;
    }
    let builder = Builder {
        shape,
        density: DENSITY[level],
        noise: NOISE[level],
        longest: LONGEST[level],
        nb: neighbours(shape.rows(), shape.cols()),
        ways,
    };
    match search(
        &builder,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => {
            let mut solution = Answer::new(shape);
            for (i, &d) in found.candidate.digits.iter().enumerate() {
                if found.candidate.white[i] {
                    solution.set(i / shape.cols(), i % shape.cols(), Mark::Digit(d));
                }
            }
            Ok(Generated {
                puzzle: builder.puzzle(&found.candidate),
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
