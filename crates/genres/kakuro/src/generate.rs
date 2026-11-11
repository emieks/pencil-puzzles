//! Reproducible generation: a symmetric layout of blocks, random digits,
//! and digits changed until the human techniques solve the puzzle at the
//! requested level.
use crate::human::screen;
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::grid::{flood, neighbours};
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::cell::RefCell;
use std::{error::Error, fmt};

/// The longest run the generator builds, by level: any, easy, medium,
/// hard, expert.
const LONGEST: [usize; 5] = [4, 4, 4, 5, 5];
/// Percent of the inner cells (below the first row, right of the first
/// column) made blocks, by level as for [`LONGEST`]. Breaking long runs
/// may need more.
const DENSITY: [usize; 5] = [30, 32, 30, 28, 26];
/// How much the digit order varies from each run's preference for low or
/// high digits, by level as for [`LONGEST`]. Small values give sums with
/// few combinations.
const NOISE: [u64; 5] = [8, 3, 6, 10, 16];
/// The most cells one placement may turn into blocks, counting the mirror
/// cell and cells left in runs of one cell.
const CASCADE: usize = 6;
/// Placements tried for the random blocks of a layout, per inner cell.
const PLACEMENTS: usize = 2;
/// Layouts tried before runs one cell longer are allowed.
const LAYOUT_TRIES: u32 = 256;
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
/// Solutions enumerated to find the cells that vary between them.
const AMBIGUITY: usize = 4;
/// Cells tried by the brute-force solver when screening before it gives up.
const SEARCH_NODES: usize = 2048;

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
    /// For the last stuck outcomes of puzzles with several solutions: the
    /// unresolved variables and how many of them, listed first, vary
    /// between the solutions.
    ambiguous: RefCell<Vec<(Vec<usize>, usize)>>,
}

/// A layout of blocks while it is built.
struct Layout {
    white: Vec<bool>,
    whites: usize,
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

    /// The cell opposite `i` under a half turn of the inner area.
    fn mirror(&self, i: usize) -> usize {
        let cols = self.shape.cols();
        let (r, c) = (i / cols, i % cols);
        (self.shape.rows() - r) * cols + (cols - c)
    }

    /// Whether white cell `i` has a white neighbour along the axis.
    fn joined(&self, white: &[bool], i: usize, down: bool) -> bool {
        let cols = self.shape.cols();
        let step = if down { cols } else { 1 };
        let before = i >= step && (down || !i.is_multiple_of(cols)) && white[i - step];
        let after =
            i + step < white.len() && (down || !(i + step).is_multiple_of(cols)) && white[i + step];
        before || after
    }

    /// Make `i` and its mirror blocks, then every white cell left in a run
    /// of one cell, and so on. Undone, returning `None`, if more than
    /// `limit` cells change, fewer than four white cells remain, an inner
    /// row or column loses its last white cell, or the white cells are no
    /// longer connected.
    fn block(&self, layout: &mut Layout, i: usize, limit: usize) -> Option<Vec<usize>> {
        let cols = self.shape.cols();
        let white = &mut layout.white;
        let mut changed = Vec::new();
        let mut stack = vec![i, self.mirror(i)];
        while let Some(j) = stack.pop() {
            if !white[j] {
                continue;
            }
            white[j] = false;
            changed.push(j);
            for &k in &self.nb[j] {
                if white[k] && !(self.joined(white, k, false) && self.joined(white, k, true)) {
                    stack.push(k);
                    stack.push(self.mirror(k));
                }
            }
        }
        let left = layout.whites - changed.len();
        let line_emptied = changed.iter().any(|&j| {
            let (r, c) = (j / cols, j % cols);
            (1..cols).all(|k| !white[r * cols + k])
                || (1..self.shape.rows()).all(|k| !white[k * cols + c])
        });
        let disconnected = (0..white.len())
            .find(|&j| white[j])
            .is_none_or(|s| flood(s, |j| white[j], &self.nb).len() != left);
        let fails = changed.len() > limit || left < 4 || line_emptied || disconnected;
        if fails {
            for j in changed {
                white[j] = true;
            }
            return None;
        }
        layout.whites = left;
        Some(changed)
    }

    /// A run longer than `longest`, if any: its cells in order.
    fn long_run(&self, white: &[bool], longest: usize) -> Option<Vec<usize>> {
        let cols = self.shape.cols();
        (0..white.len()).find_map(|i| {
            [false, true].into_iter().find_map(|down| {
                let step = if down { cols } else { 1 };
                let first = i < step || (!down && i % cols == 0) || !white[i - step];
                (white[i] && first)
                    .then(|| self.run(white, i, down))
                    .filter(|r| r.len() > longest)
            })
        })
    }

    /// A random layout with a half turn symmetry: inner cells start white;
    /// every run longer than the limit is broken by a block inside it,
    /// then random cells become blocks up to the density. Every placement
    /// keeps every run at least two cells long, every inner row and column
    /// partly white, and the white cells connected. `None` if a long run
    /// cannot be broken.
    fn layout(
        &self,
        rng: &mut SplitMix64,
        budget: &Budget,
        longest: usize,
    ) -> Result<Option<Vec<bool>>, BudgetExceeded> {
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let n = self.shape.cells();
        if rows < 3 || cols < 3 {
            // Too small for a run of two cells.
            return Ok(Some(vec![false; n]));
        }
        let inner = (rows - 1) * (cols - 1);
        let mut layout = Layout {
            white: (0..n).map(|i| i / cols > 0 && i % cols > 0).collect(),
            whites: inner,
        };
        while let Some(long) = self.long_run(&layout.white, longest) {
            Self::spend(budget, n)?;
            // A block inside the run leaves at least two cells on each
            // side; one that blocks only the pair is preferred.
            let mut inside: Vec<usize> = long[2..long.len() - 2].to_vec();
            rng.shuffle(&mut inside);
            if ![2, CASCADE].iter().any(|&limit| {
                inside
                    .iter()
                    .any(|&i| self.block(&mut layout, i, limit).is_some())
            }) {
                return Ok(None);
            }
        }
        let target = inner * self.density / 100;
        for _ in 0..inner * PLACEMENTS {
            if inner - layout.whites >= target {
                break;
            }
            Self::spend(budget, n)?;
            let i = (1 + rng.below((rows - 1) as u64) as usize) * cols
                + 1
                + rng.below((cols - 1) as u64) as usize;
            if layout.white[i] && self.block(&mut layout, i, 2).is_none() {
                self.block(&mut layout, i, CASCADE);
            }
        }
        Ok(Some(layout.white))
    }

    /// Random digits with no repeat in a run, row-major by backtracking
    /// with an explicit stack. Each run prefers low or high digits at
    /// random, and each cell tries digits in the order its two runs prefer,
    /// varied by [`NOISE`]. A digit that completes a 2x2 block of white
    /// cells holding two digits crosswise is skipped, since swapping them
    /// would give a second solution. `None` after [`FILL_NODES`] cells.
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
        // Digits already in the cells before `i` in its two runs, and the
        // digit that would complete a crosswise 2x2 block.
        let taken = |digits: &[u8], i: usize| {
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
            if !i.is_multiple_of(cols)
                && i >= cols
                && white[i - 1]
                && white[i - cols]
                && white[i - cols - 1]
                && digits[i - 1] == digits[i - cols]
            {
                used |= 1 << digits[i - cols - 1];
            }
            used
        };
        let mut stack = vec![(0usize, order(rng, cells[0]), 0usize)];
        let mut nodes = 0;
        while let Some(top) = stack.last_mut() {
            let i = cells[top.0];
            digits[i] = 0;
            let taken = taken(&digits, i);
            let Some(k) = (top.2..9).find(|&k| taken & (1 << top.1[k]) == 0) else {
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

    /// One change to cell `i`: give it a digit neither of its runs has, or
    /// swap its digit with another cell of one of its runs where neither
    /// digit repeats.
    fn alter(
        &self,
        c: &Candidate,
        i: usize,
        stuck: bool,
        rng: &mut SplitMix64,
    ) -> Option<Candidate> {
        let mates = self.mates(c, i);
        let taken =
            Self::digits_of(c, &mates[0]) | Self::digits_of(c, &mates[1]) | 1 << c.digits[i];
        let free: Vec<u8> = (1..=9).filter(|&d| taken & (1 << d) == 0).collect();
        if !free.is_empty() {
            let mut next = c.clone();
            // The combinations the two runs would have with digit `d`.
            let ways = |d: u8| -> u32 {
                mates
                    .iter()
                    .map(|run| {
                        let sum: u32 = run.iter().map(|&j| u32::from(c.digits[j])).sum();
                        self.ways[run.len() + 1][(sum + u32::from(d)) as usize]
                    })
                    .sum()
            };
            next.digits[i] = if stuck {
                // The digit leaving the two runs the fewest combinations.
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

    /// A layout and digits. When layouts keep failing, runs one cell
    /// longer are allowed, up to [`MAX_RUN`].
    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        let mut failures = 0;
        loop {
            let longest = (self.longest + (failures / LAYOUT_TRIES) as usize).min(MAX_RUN);
            let Some(white) = self.layout(rng, budget, longest)? else {
                failures += 1;
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

    /// Techniques up to Medium (or the cap) run first. If they do not
    /// solve the puzzle, the brute-force solver looks for solutions within
    /// the candidates they left, up to [`SEARCH_NODES`] cells tried. With
    /// several solutions the puzzle is stuck with the cells the techniques
    /// left, those varying between the solutions listed first. A unique
    /// puzzle is tried with the chains when the cap allows.
    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        let grid = crate::human::Grid::new(p);
        let start = crate::human::initial_masks(&grid);
        let low = cap.min(Difficulty::Medium);
        let trace =
            crate::human::analyze(&grid, start, vec![false; grid.whites.len()], budget, low)?;
        let outcome = trace.outcome();
        if trace.solved {
            return Ok(outcome);
        }
        let Outcome::Stuck(mut vars) = outcome else {
            return Ok(outcome);
        };
        let (found, complete) =
            crate::board::solutions_within(p, &trace.masks, AMBIGUITY, SEARCH_NODES, budget)?;
        if found.len() > 1 || (complete && found.is_empty()) {
            let varies = |v: usize| {
                let i = grid.whites[v];
                found.iter().any(|a| a.marks()[i] != found[0].marks()[i])
            };
            let mut varying: Vec<usize> = vars.iter().copied().filter(|&v| varies(v)).collect();
            vars.retain(|v| !varying.contains(v));
            let count = varying.len();
            varying.extend(vars);
            let mut ambiguous = self.ambiguous.borrow_mut();
            if ambiguous.len() == 2 {
                ambiguous.remove(0);
            }
            ambiguous.push((varying.clone(), count));
            return Ok(Outcome::Stuck(varying));
        }
        if cap > low && complete {
            return screen(p, budget, cap);
        }
        Ok(Outcome::Stuck(vars))
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
            Outcome::Stuck(vars) if !vars.is_empty() => {
                // Cells varying between solutions come first; every other
                // change is to one of those.
                let varying = self
                    .ambiguous
                    .borrow()
                    .iter()
                    .find(|(v, _)| v == vars)
                    .map_or(vars.len(), |&(_, count)| count);
                let take = if rng.below(2) == 0 {
                    varying
                } else {
                    vars.len()
                };
                vars[..take].iter().map(|&v| whites[v]).collect()
            }
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

/// Generate with the shared search loop. A symmetric layout of blocks gets
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
        ambiguous: RefCell::new(Vec::new()),
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
                attempts: found.attempts,
            })
        }
        Err(SearchError::BudgetExceeded) => Err(GenerateError::BudgetExceeded),
        Err(SearchError::AttemptsExhausted(attempts)) => {
            Err(GenerateError::AttemptsExhausted { attempts })
        }
    }
}
