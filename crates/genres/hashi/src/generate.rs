//! Reproducible generation: a random network of islands and bridges, then
//! small changes where the techniques get stuck, until the puzzle has one
//! solution at the requested level.
use crate::board::{Layout, solutions};
use crate::human::{analyze_state, initial_masks};
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::{error::Error, fmt};

/// Generator version (V3).
pub const GENERATOR_VERSION: u32 = 1;

/// Rejected changes before starting from a new network.
const PATIENCE: u32 = 30;
/// Changes without getting closer to the requested level before starting
/// from a new network.
const STALE: u32 = 40;

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
                "no Hashi of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => f.write_str("the work budget ran out before a Hashi was found"),
        }
    }
}

impl Error for GenerateError {}

impl From<BudgetExceeded> for GenerateError {
    fn from(_: BudgetExceeded) -> Self {
        Self::BudgetExceeded
    }
}

/// A network: the island cells and the bridges between them, as (first
/// cell, second cell, count) with the first cell first in row-major order,
/// and the search's memory: the best outcome rank so far and the changes
/// since it last improved.
#[derive(Clone, Debug)]
struct Network {
    islands: Vec<usize>,
    bridges: Vec<(usize, usize, u8)>,
    best: (bool, i64),
    stale: u32,
}

impl Network {
    fn new(islands: Vec<usize>) -> Self {
        Self {
            islands,
            bridges: Vec::new(),
            best: (false, i64::MIN),
            stale: 0,
        }
    }
}

struct Builder {
    shape: Shape,
    /// One bridge in this many free slots is added to close loops.
    loops: u64,
}

impl Builder {
    fn puzzle(&self, n: &Network) -> Puzzle {
        let mut clues = vec![0u8; self.shape.cells()];
        for &(a, b, k) in &n.bridges {
            clues[a] += k;
            clues[b] += k;
        }
        for &i in &n.islands {
            debug_assert!(clues[i] > 0, "every island has a bridge");
        }
        Puzzle::new(self.shape, clues).expect("networks keep islands apart")
    }

    /// The cells covered by islands or bridges.
    fn used(&self, n: &Network) -> Vec<bool> {
        let cols = self.shape.cols();
        let mut used = vec![false; self.shape.cells()];
        for &i in &n.islands {
            used[i] = true;
        }
        for &(a, b, _) in &n.bridges {
            let step = if a / cols == b / cols { 1 } else { cols };
            for i in (a..=b).step_by(step) {
                used[i] = true;
            }
        }
        used
    }

    /// Whether an island at `i` would touch another island.
    fn crowded(&self, n: &Network, i: usize) -> bool {
        let cols = self.shape.cols();
        let (r, c) = (i / cols, i % cols);
        n.islands.iter().any(|&j| {
            let (jr, jc) = (j / cols, j % cols);
            r.abs_diff(jr) + c.abs_diff(jc) == 1
        })
    }

    /// Grow a network from `start`: repeatedly run a bridge from a random
    /// island to a new island at a free cell in line with it.
    fn grow(&self, start: usize, rng: &mut SplitMix64) -> Network {
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let target = (self.shape.cells() / 3).max(2);
        let mut n = Network::new(vec![start]);
        let mut used = self.used(&n);
        for _ in 0..20 * target {
            if n.islands.len() >= target {
                break;
            }
            let from = n.islands[rng.below(n.islands.len() as u64) as usize];
            let (dr, dc) = [(0i64, 1i64), (1, 0), (0, -1), (-1, 0)][rng.below(4) as usize];
            let mut ends = Vec::new();
            let (mut r, mut c) = ((from / cols) as i64, (from % cols) as i64);
            for distance in 1.. {
                r += dr;
                c += dc;
                if r < 0 || c < 0 || r >= rows as i64 || c >= cols as i64 {
                    break;
                }
                let i = r as usize * cols + c as usize;
                if used[i] {
                    break;
                }
                if distance >= 2 && !self.crowded(&n, i) {
                    ends.push(i);
                }
            }
            if ends.is_empty() {
                continue;
            }
            let to = ends[rng.below(ends.len() as u64) as usize];
            let count = 1 + rng.below(2) as u8;
            n.islands.push(to);
            n.bridges.push((from.min(to), from.max(to), count));
            used = self.used(&n);
        }
        n.islands.sort_unstable();
        n
    }

    /// Add single bridges between islands that see each other, where no
    /// bridge crosses, to close loops.
    fn close_loops(&self, n: &mut Network, rng: &mut SplitMix64) {
        let layout = Layout::new(&self.placeholder(n));
        let mut counts = self.counts(n, &layout);
        for s in 0..layout.slots.len() {
            let free = counts[s] == 0 && layout.crosses[s].iter().all(|&x| counts[x] == 0);
            if free && rng.below(self.loops) == 0 {
                counts[s] = 1;
                let [a, b] = layout.slots[s].ends.map(|k| layout.cells[k]);
                n.bridges.push((a, b, 1));
            }
        }
    }

    /// A puzzle with the network's islands, for finding slots.
    fn placeholder(&self, n: &Network) -> Puzzle {
        let mut clues = vec![0u8; self.shape.cells()];
        for &i in &n.islands {
            clues[i] = 8;
        }
        Puzzle::new(self.shape, clues).expect("networks keep islands apart")
    }

    /// The bridges on each slot of `layout`.
    fn counts(&self, n: &Network, layout: &Layout) -> Vec<u8> {
        layout
            .slots
            .iter()
            .map(|slot| {
                let [a, b] = slot.ends.map(|k| layout.cells[k]);
                n.bridges
                    .iter()
                    .find(|&&(x, y, _)| (x, y) == (a, b))
                    .map_or(0, |&(_, _, k)| k)
            })
            .collect()
    }

    /// One change at slot `s`, if one fits: add a bridge where none is;
    /// or change one bridge to two (or two to one), put a new island in the
    /// middle of the bridges, or remove them if the network stays connected
    /// and both islands keep a bridge.
    fn edit(&self, n: &Network, s: usize, rng: &mut SplitMix64) -> Option<Network> {
        let layout = Layout::new(&self.placeholder(n));
        let counts = self.counts(n, &layout);
        let slot = &layout.slots[s];
        let [a, b] = slot.ends.map(|k| layout.cells[k]);
        let mut next = n.clone();
        if counts[s] == 0 {
            if layout.crosses[s].iter().any(|&x| counts[x] > 0) {
                return None;
            }
            next.bridges.push((a, b, 1));
            return Some(next);
        }
        let k = next
            .bridges
            .iter()
            .position(|&(x, y, _)| (x, y) == (a, b))
            .expect("the slot has a bridge");
        // Cells for a new island: not next to an island.
        let inner = slot.interior.get(1..slot.interior.len().saturating_sub(1));
        let room: Vec<usize> = inner
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|&i| !self.crowded(n, i))
            .collect();
        let count = next.bridges[k].2;
        let mut without = n.clone();
        without.bridges.remove(k);
        let left = self.counts(&without, &layout);
        let removable = [a, b]
            .iter()
            .all(|&i| without.bridges.iter().any(|&(x, y, _)| x == i || y == i))
            && layout.groups(|t| left[t] > 0) == 1;
        let choice = rng.below(3);
        if choice == 0 && removable {
            return Some(without);
        }
        if room.is_empty() || choice == 1 {
            next.bridges[k].2 = 3 - count;
        } else {
            let i = room[rng.below(room.len() as u64) as usize];
            next.bridges[k] = (a, i, count);
            next.bridges.push((i, b, count));
            next.islands.push(i);
            next.islands.sort_unstable();
        }
        Some(next)
    }
}

impl Construct for Builder {
    type Candidate = Network;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Network, BudgetExceeded> {
        let cells = self.shape.cells();
        // Growing looks at a row or column for each try.
        if !budget.spend((cells * cells) as u64) {
            return Err(BudgetExceeded);
        }
        // A grid too small for any bridge gets no islands.
        for _ in 0..cells {
            let mut n = self.grow(rng.below(cells as u64) as usize, rng);
            if !n.bridges.is_empty() {
                self.close_loops(&mut n, rng);
                return Ok(n);
            }
        }
        Ok(Network::new(Vec::new()))
    }

    fn puzzle(&self, n: &Network) -> Puzzle {
        Builder::puzzle(self, n)
    }

    /// Several solutions leave the slots where two of them differ
    /// unresolved; otherwise the techniques up to `cap` decide.
    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        let layout = Layout::new(p);
        let found = solutions(&layout, budget)?;
        if let [first, second] = &found[..] {
            return Ok(Outcome::Stuck(
                (0..first.len())
                    .filter(|&s| first[s] != second[s])
                    .collect(),
            ));
        }
        let start = initial_masks(&layout);
        Ok(analyze_state(&layout, start, budget, cap, false)?.outcome())
    }

    /// Change the network at a slot where solving got stuck, or anywhere
    /// when the puzzle is too easy.
    fn change(
        &self,
        n: &mut Network,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Network> {
        if outcome.rank() > n.best {
            n.best = outcome.rank();
            n.stale = 0;
        } else {
            n.stale += 1;
            if n.stale > STALE {
                return Move::Restart;
            }
        }
        let slots = Layout::new(&self.placeholder(n)).slots.len();
        let targets: Vec<usize> = match outcome {
            Outcome::Stuck(unresolved) if !unresolved.is_empty() => unresolved.clone(),
            _ => (0..slots).collect(),
        };
        for _ in 0..8 {
            if targets.is_empty() {
                break;
            }
            let s = targets[rng.below(targets.len() as u64) as usize];
            if let Some(next) = self.edit(n, s, rng) {
                return Move::Try(next);
            }
        }
        Move::Restart
    }

    fn patience(&self) -> u32 {
        PATIENCE
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop. A random network of islands and
/// bridges is changed where the techniques up to the requested level get
/// stuck, or where two solutions differ, until it has exactly one solution
/// at that level. The returned puzzle is rated in full and checked for
/// uniqueness. Extreme is not supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    if options.difficulty == Some(Difficulty::Extreme) {
        return Err(GenerateError::UnsupportedRequest(
            "Extreme Hashi generation is unavailable: no Extreme human technique profile is supported",
        ));
    }
    let shape = options.params.shape;
    let hard = options.difficulty.is_some_and(|d| d >= Difficulty::Hard);
    let builder = Builder {
        shape,
        loops: if hard { 3 } else { 8 },
    };
    match search(
        &builder,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => {
            let puzzle = builder.puzzle(&found.candidate);
            let layout = Layout::new(&puzzle);
            let counts = builder.counts(&found.candidate, &layout);
            let solution = Answer::from_counts(&puzzle, &layout, &counts, true);
            Ok(Generated {
                puzzle,
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
