//! Reproducible generation: random rectangles and small regions of one
//! shade each, which are joined while the human techniques still solve the
//! puzzle, then changes to the region borders and the rectangles until they
//! solve it at exactly the requested level.
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
                "no Chocona of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a Chocona was found")
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

/// Why `generate` turns down a level at a shape, if it does.
pub(crate) fn unsupported(shape: Shape, difficulty: Option<Difficulty>) -> Option<&'static str> {
    let (side, cells) = (shape.rows().min(shape.cols()), shape.cells());
    match difficulty {
        Some(Difficulty::Extreme) => Some(
            "Extreme Chocona generation is unavailable: no Extreme human technique profile is supported",
        ),
        Some(Difficulty::Expert) if side < 3 || cells < 15 => Some(
            "Expert Chocona generation needs at least 3 rows, 3 columns and 15 cells, such as 3x5",
        ),
        Some(Difficulty::Hard) if side < 2 || cells < 9 => Some(
            "Hard Chocona generation needs at least 2 rows, 2 columns and 9 cells, such as 3x3",
        ),
        Some(Difficulty::Medium) if side < 2 || cells < 6 => Some(
            "Medium Chocona generation needs at least 2 rows, 2 columns and 6 cells, such as 2x3",
        ),
        _ => None,
    }
}

/// The sizes of the regions a candidate starts with, with their weights.
/// Each starts with cells of one shade only, so that its number settles it.
const PIECES: [(usize, u64); 3] = [(2, 1), (3, 1), (4, 1)];
/// The mean region size of a finished puzzle whose rows and columns
/// average at least [`FULL_SIDE`] cells, in tenths of a cell. Smaller grids
/// aim lower, as do grids less than a quarter as wide as that.
const MEAN: usize = 45;
/// See [`MEAN`].
const FULL_SIDE: usize = 10;
/// Regions smaller than this are rare in a finished puzzle (see
/// [`SINGLES`]). Border changes never shrink a region below it.
const SMALLEST: usize = 2;
/// The most work the uniqueness check in `screen` may do before the full
/// screen runs instead.
const PRECHECK: u64 = 50_000;
/// A finished puzzle has at most one region smaller than [`SMALLEST`] in
/// this many cells.
const SINGLES: usize = 50;
/// A finished puzzle has at most one region without shaded cells in this
/// many regions.
const EMPTY: usize = 5;
/// The builder moves one cell across a region border for every this many
/// cells.
const MOVES: usize = 4;
/// Merges and border changes never grow a region past this size.
const LARGEST: usize = 12;
/// Rectangle heights and widths the builder tries first, with their
/// weights.
const RECTANGLES: [((usize, usize), u64); 15] = [
    ((1, 1), 1),
    ((1, 2), 2),
    ((2, 1), 2),
    ((2, 2), 3),
    ((1, 3), 1),
    ((3, 1), 1),
    ((2, 3), 3),
    ((3, 2), 3),
    ((3, 3), 2),
    ((2, 4), 2),
    ((4, 2), 2),
    ((1, 4), 1),
    ((4, 1), 1),
    ((3, 4), 1),
    ((4, 3), 1),
];

/// The builder skips one place in this many where a rectangle fits.
const SKIP: u64 = 8;

/// The number of regions a finished puzzle of this shape has at most.
fn goal_regions(shape: Shape) -> usize {
    let narrow = 4 * shape.rows().min(shape.cols());
    let side = ((shape.rows() + shape.cols()) / 2)
        .min(narrow)
        .min(FULL_SIDE);
    let mean = (MEAN * side / FULL_SIDE).max(10 * SMALLEST);
    (10 * shape.cells() / mean).max(1)
}

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

/// A candidate: each cell's region and whether it is shaded. Every region
/// shows its number. Regions keep the numbers they were built with, which
/// are below the number of cells.
#[derive(Clone)]
struct Candidate {
    region: Vec<u32>,
    shaded: Vec<bool>,
    /// Changes proposed since the candidate was built.
    edits: u32,
}

struct Builder {
    shape: Shape,
    nb: Vec<Vec<usize>>,
    /// The number of regions of a finished puzzle.
    regions: usize,
    /// The requested level, or Expert for any level.
    cap: Difficulty,
}

impl Builder {
    fn cols(&self) -> usize {
        self.shape.cols()
    }

    /// Whether every 2x2 block has other than three shaded cells, so that
    /// the shaded cells form rectangles.
    fn rectangles(&self, shaded: &[bool]) -> bool {
        let (rows, cols) = (self.shape.rows(), self.cols());
        (0..rows.saturating_sub(1)).all(|r| {
            (0..cols.saturating_sub(1)).all(|c| {
                let i = r * cols + c;
                [i, i + 1, i + cols, i + cols + 1]
                    .iter()
                    .filter(|&&j| shaded[j])
                    .count()
                    != 3
            })
        })
    }

    /// Place rectangles that do not touch by their sides, reading the cells
    /// row by row. At each cell where one fits (but one in [`SKIP`]), a
    /// rectangle of a random size starts, made shorter or narrower until it
    /// fits. About half of the cells end up shaded.
    fn shade(&self, rng: &mut SplitMix64) -> Vec<bool> {
        let (rows, cols) = (self.shape.rows(), self.cols());
        let n = self.shape.cells();
        let mut shaded = vec![false; n];
        let weights: Vec<u64> = RECTANGLES.iter().map(|r| r.1).collect();
        let clear =
            |i: usize, shaded: &[bool]| !shaded[i] && self.nb[i].iter().all(|&j| !shaded[j]);
        for start in 0..n {
            if !clear(start, &shaded) || rng.below(SKIP) == 0 {
                continue;
            }
            let (top, left) = (start / cols, start % cols);
            let (mut h, mut w) = RECTANGLES[choose(&weights, rng)].0;
            let single = h * w == 1;
            h = h.min(rows - top);
            w = w.min(cols - left);
            let fits = |h: usize, w: usize| {
                (top..top + h).all(|r| (left..left + w).all(|c| clear(r * cols + c, &shaded)))
            };
            while h > 0 && w > 0 && !fits(h, w) {
                if h >= w {
                    h -= 1;
                } else {
                    w -= 1;
                }
            }
            // Only a rectangle chosen as one cell may end up as one.
            if h > 0 && w > 0 && (single || h * w > 1) {
                for r in top..top + h {
                    for c in left..left + w {
                        shaded[r * cols + c] = true;
                    }
                }
            }
        }
        shaded
    }

    /// Change the rectangles at `i`: a shaded cell's rectangle loses its row
    /// or column on a random side, and an unshaded cell joins the rectangle
    /// beside it as a new row or column of it, or becomes a rectangle of its
    /// own. False if the result breaks the rectangles or would leave fewer
    /// than a fifth or more than half of the cells shaded.
    fn reshape(&self, c: &mut Candidate, i: usize, rng: &mut SplitMix64) -> bool {
        let cols = self.cols();
        let n = self.shape.cells();
        let count = c.shaded.iter().filter(|&&s| s).count();
        let mut next = c.shaded.clone();
        if c.shaded[i] {
            if 5 * count <= n {
                return false;
            }
            let cells = flood(i, |j| c.shaded[j], &self.nb);
            let rows: Vec<usize> = cells.iter().map(|&j| j / cols).collect();
            let columns: Vec<usize> = cells.iter().map(|&j| j % cols).collect();
            let (top, bottom) = (*rows.iter().min().unwrap(), *rows.iter().max().unwrap());
            let (left, right) = (
                *columns.iter().min().unwrap(),
                *columns.iter().max().unwrap(),
            );
            // Remove the edge row or column at a random side.
            let side = rng.below(4);
            for &j in &cells {
                let drop = match side {
                    0 => j / cols == top,
                    1 => j / cols == bottom,
                    2 => j % cols == left,
                    _ => j % cols == right,
                };
                if drop {
                    next[j] = false;
                }
            }
        } else {
            if 2 * count >= n {
                return false;
            }
            let beside: Vec<usize> = self.nb[i]
                .iter()
                .copied()
                .filter(|&j| c.shaded[j])
                .collect();
            if beside.is_empty() {
                next[i] = true;
            } else {
                // Extend the rectangle beside the cell by the strip through it.
                let j = beside[rng.below(beside.len() as u64) as usize];
                let cells = flood(j, |k| c.shaded[k], &self.nb);
                let along_row = j / cols == i / cols;
                for &k in &cells {
                    let strip = if along_row {
                        (k / cols) * cols + i % cols
                    } else {
                        (i / cols) * cols + k % cols
                    };
                    next[strip] = true;
                }
            }
        }
        if !self.rectangles(&next) {
            return false;
        }
        c.shaded = next;
        true
    }

    /// Move cell `i` to the region of a random neighbour, if its own region
    /// stays in one piece and both regions stay between [`SMALLEST`] and
    /// [`LARGEST`] cells.
    fn transfer(&self, c: &mut Candidate, i: usize, rng: &mut SplitMix64) -> bool {
        let others: Vec<u32> = (self.nb[i].iter())
            .map(|&j| c.region[j])
            .filter(|&t| t != c.region[i])
            .collect();
        if others.is_empty() {
            return false;
        }
        let to = others[rng.below(others.len() as u64) as usize];
        self.move_cell(c, i, to)
    }

    /// Move cell `i` to region `to`, a neighbour of it, if its own region
    /// stays in one piece and both regions stay between [`SMALLEST`] and
    /// [`LARGEST`] cells.
    fn move_cell(&self, c: &mut Candidate, i: usize, to: u32) -> bool {
        let from = c.region[i];
        let size = |t: u32| c.region.iter().filter(|&&u| u == t).count();
        let Some(start) = self.nb[i].iter().copied().find(|&j| c.region[j] == from) else {
            return false;
        };
        let size_from = size(from);
        if size_from <= SMALLEST || size(to) >= LARGEST {
            return false;
        }
        if flood(start, |j| j != i && c.region[j] == from, &self.nb).len() != size_from - 1 {
            return false;
        }
        c.region[i] = to;
        true
    }

    /// Whether there are few enough regions, and at most one region in
    /// [`SINGLES`] cells is smaller than [`SMALLEST`].
    fn built(&self, c: &Candidate) -> bool {
        let size = self.sizes(c);
        let present = size.iter().filter(|&&s| s > 0);
        let small = present.clone().filter(|&&s| s < SMALLEST).count();
        let regions = present.count();
        let empty = self.empty(c).iter().filter(|&&e| e).count();
        regions <= self.regions
            && small <= self.shape.cells().div_ceil(SINGLES)
            && empty <= regions.div_ceil(EMPTY)
    }

    /// Whether each region (by region number) has cells but none shaded.
    fn empty(&self, c: &Candidate) -> Vec<bool> {
        let mut has = vec![(false, false); c.region.len()];
        for (i, &t) in c.region.iter().enumerate() {
            has[t as usize].0 = true;
            has[t as usize].1 |= c.shaded[i];
        }
        has.into_iter()
            .map(|(cells, shaded)| cells && !shaded)
            .collect()
    }

    /// Each region's size, by region number.
    fn sizes(&self, c: &Candidate) -> Vec<usize> {
        let mut size = vec![0; c.region.len()];
        for &t in &c.region {
            size[t as usize] += 1;
        }
        size
    }

    /// Join the region at `i` with a random neighbouring region, if the two
    /// together have at most [`LARGEST`] cells.
    fn merge(&self, c: &mut Candidate, i: usize, rng: &mut SplitMix64) -> bool {
        let size = self.sizes(c);
        let t = c.region[i];
        let mut others: Vec<u32> = (0..c.region.len())
            .filter(|&j| c.region[j] == t)
            .flat_map(|j| self.nb[j].iter().map(|&k| c.region[k]))
            .filter(|&u| u != t && size[t as usize] + size[u as usize] <= LARGEST)
            .collect();
        others.sort_unstable();
        others.dedup();
        if others.is_empty() {
            return false;
        }
        // A region with shaded cells prefers to join one without, and the
        // other way round, ten times to one.
        let any_shaded = |t: u32| (0..c.region.len()).any(|j| c.region[j] == t && c.shaded[j]);
        let mine = any_shaded(t);
        let weights: Vec<u64> = (others.iter())
            .map(|&u| if any_shaded(u) == mine { 1 } else { 10 })
            .collect();
        let u = others[choose(&weights, rng)];
        for r in &mut c.region {
            if *r == t {
                *r = u;
            }
        }
        true
    }

    fn solution(&self, c: &Candidate) -> Answer {
        let cols = self.cols();
        let mut a = Answer::new(self.shape);
        for (i, &s) in c.shaded.iter().enumerate() {
            let mark = if s { Mark::Shaded } else { Mark::Unshaded };
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
        // Regions grow one at a time from the first free cell to a small
        // random size, through cells of the same shade.
        let shaded = self.shade(rng);
        let sizes: Vec<u64> = PIECES.iter().map(|s| s.1).collect();
        let mut region = vec![u32::MAX; n];
        let mut count = 0;
        for start in 0..n {
            if region[start] != u32::MAX {
                continue;
            }
            let t = count as u32;
            count += 1;
            region[start] = t;
            let free =
                |j: usize, region: &[u32]| region[j] == u32::MAX && shaded[j] == shaded[start];
            let mut frontier: Vec<usize> = (self.nb[start].iter())
                .copied()
                .filter(|&j| free(j, &region))
                .collect();
            for _ in 1..PIECES[choose(&sizes, rng)].0 {
                if frontier.is_empty() {
                    break;
                }
                let j = frontier.swap_remove(rng.below(frontier.len() as u64) as usize);
                region[j] = t;
                for &m in &self.nb[j] {
                    if free(m, &region) && !frontier.contains(&m) {
                        frontier.push(m);
                    }
                }
            }
        }
        // A piece of one cell joins a neighbouring piece of its shade, if
        // there is one.
        let mut size = vec![0; n];
        for &t in &region {
            size[t as usize] += 1;
        }
        for i in 0..n {
            if size[region[i] as usize] != 1 {
                continue;
            }
            let into = (self.nb[i].iter())
                .find(|&&j| shaded[j] == shaded[i])
                .map(|&j| region[j]);
            if let Some(into) = into {
                size[region[i] as usize] = 0;
                size[into as usize] += 1;
                region[i] = into;
            }
        }
        let mut c = Candidate {
            region,
            shaded,
            edits: 0,
        };
        // Join random neighbouring regions while the techniques up to
        // Medium (or the requested level, if lower) still solve the
        // puzzle, until the candidate is finished or as many joins in a row
        // as there are cells have failed. Regions smaller than SMALLEST are
        // joined first.
        let cap = self.cap.min(Difficulty::Medium);
        let mut regions = count;
        let mut failed = 0;
        while failed < n && !self.built(&c) {
            let mut next = c.clone();
            let size = self.sizes(&c);
            let empty = self.empty(&c);
            let small: Vec<usize> = (0..n)
                .filter(|&i| {
                    let t = c.region[i] as usize;
                    size[t] < SMALLEST || empty[t]
                })
                .collect();
            let at = if small.is_empty() || (regions > self.regions && failed >= small.len()) {
                rng.below(n as u64) as usize
            } else {
                small[rng.below(small.len() as u64) as usize]
            };
            failed += 1;
            // A one-cell rectangle in a region of its own leaves its new
            // region with no way to tell which cell is shaded, so it is
            // unshaded first.
            if size[c.region[at] as usize] == 1 && c.shaded[at] {
                next.shaded[at] = false;
            }
            if !self.merge(&mut next, at, rng) {
                continue;
            }
            if let Outcome::Solved(..) = self.screen(&self.puzzle(&next), cap, budget)? {
                c = next;
                regions -= 1;
                failed = 0;
            }
        }
        // Move cells across region borders, which gives the regions
        // irregular shapes, while the same techniques solve the puzzle.
        let (mut kept, mut failed) = (0, 0);
        while (MOVES * kept) < n && failed < n {
            let mut next = c.clone();
            failed += 1;
            if !self.transfer(&mut next, rng.below(n as u64) as usize, rng) {
                continue;
            }
            if let Outcome::Solved(..) = self.screen(&self.puzzle(&next), cap, budget)? {
                c = next;
                kept += 1;
                failed = 0;
            }
        }
        Ok(c)
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        // Regions are numbered in the order their first cells appear.
        let mut order = vec![usize::MAX; c.region.len()];
        let mut clues = Vec::new();
        for (i, &t) in c.region.iter().enumerate() {
            let t = t as usize;
            if order[t] == usize::MAX {
                order[t] = clues.len();
                clues.push(0);
            }
            clues[order[t]] += u32::from(c.shaded[i]);
        }
        let clues = clues.into_iter().map(Some).collect();
        Puzzle::new(self.shape, &c.region, clues).expect("generated regions are valid")
    }

    /// Above Medium, a puzzle the techniques up to Medium cannot finish is
    /// first checked with the exact solver, from where they stopped, which
    /// keeps every solution. One without a unique solution is stuck at any
    /// level, so the costly contradiction techniques are skipped: a stuck
    /// candidate is never kept by a solved one. With a unique solution, the
    /// contradiction techniques know it (see `Grid::solution`). Neither
    /// changes the outcome.
    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        let mut grid = Grid::new(p);
        if cap > Difficulty::Medium {
            let trace = analyze_state(&grid, initial_masks(p), budget, Difficulty::Medium, false)?;
            if trace.solved {
                return Ok(trace.outcome());
            }
            let local = Budget::new(PRECHECK);
            let unique = crate::board::unique_from(p, &trace.masks, &local);
            if !budget.spend(local.spent()) {
                return Err(BudgetExceeded);
            }
            match unique {
                Ok(None) => return Ok(trace.outcome()),
                Ok(solution) => grid.solution = solution,
                Err(BudgetExceeded) => {}
            }
        }
        Ok(analyze_state(&grid, initial_masks(p), budget, cap, false)?.outcome())
    }

    /// At a random cell, move a cell to a neighbouring region (17 times in
    /// 20), join two regions (twice in 20, while there are more than three
    /// quarters of the regions a finished puzzle has) or change a rectangle.
    /// Candidates are always solved, since a stuck one is never kept. A
    /// candidate is rebuilt after four times as many changes as it has
    /// cells, or if its build did not finish.
    fn change(
        &self,
        c: &mut Candidate,
        _: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Candidate> {
        let n = self.shape.cells();
        c.edits += 1;
        if c.edits > 4 * n as u32 || !self.built(c) {
            return Move::Restart;
        }
        let regions = self.sizes(c).iter().filter(|&&s| s > 0).count();
        // A few tries at other cells if the first change is impossible.
        for _ in 0..8 {
            let focus = rng.below(n as u64) as usize;
            let mut next = c.clone();
            let changed = match rng.below(20) {
                0..2 if 4 * regions > 3 * self.regions => self.merge(&mut next, focus, rng),
                2 => self.reshape(&mut next, focus, rng),
                _ => self.transfer(&mut next, focus, rng),
            };
            if changed {
                return Move::Try(next);
            }
        }
        Move::Restart
    }

    fn finished(&self, c: &Candidate) -> bool {
        self.built(c)
    }

    /// A change is kept unless the level drops or the candidate gets stuck.
    fn keep(&self, before: &Outcome, after: &Outcome) -> bool {
        if let (Outcome::Solved(l1, _), Outcome::Solved(l2, _)) = (before, after) {
            return l2 >= l1;
        }
        after.rank() >= before.rank()
    }

    fn patience(&self) -> u32 {
        self.shape.cells() as u32
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop. Random rectangles get small
/// regions of one shade each, which are joined while the techniques up to
/// Medium still solve the puzzle; each region's number counts its shaded
/// cells. Region borders and rectangles are then changed until the
/// techniques up to the requested level solve the puzzle. The returned
/// puzzle is rated in full and checked for uniqueness. Extreme is not
/// supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    let shape = options.params.shape;
    if let Some(reason) = unsupported(shape, options.difficulty) {
        return Err(GenerateError::UnsupportedRequest(reason));
    }
    let builder = Builder {
        shape,
        nb: neighbours(shape.rows(), shape.cols()),
        regions: goal_regions(shape),
        cap: options.difficulty.unwrap_or(Difficulty::Expert),
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
