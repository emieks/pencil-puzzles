//! Chocona's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order. Candidate bit 0 is unshaded
//! (`-`) and bit 1 shaded (`#`). A region's need is its clue minus its
//! shaded cells. A group is a set of shaded cells joined by their sides,
//! and its box is the smallest rectangle around it.
use crate::{MAX_SIZE, Puzzle};
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty, grid};

/// The unshaded candidate.
pub(crate) const UNSHADED: u16 = 1;
/// The shaded candidate.
pub(crate) const SHADED: u16 = 2;
/// Both candidates.
const BOTH: u16 = UNSHADED | SHADED;
/// Most ways to fill a region that `combinations` compares.
const MAX_WAYS: usize = 6;
/// Most subsets of a region's open cells that `combinations` looks at.
const MAX_SUBSETS: u64 = 70;
/// Longest Hard contradiction chain.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain.
const LONG_CHAIN: usize = 20;

/// The board's regions and clues.
pub(crate) struct Grid {
    rows: usize,
    cols: usize,
    nb: Vec<Vec<usize>>,
    /// Each cell's region.
    region: Vec<u32>,
    clues: Vec<Option<u32>>,
    /// Each region's cells, in reading order.
    cells: Vec<Vec<usize>>,
    /// Each region's cells as (row, set) pairs, top to bottom.
    sets: Vec<Vec<(usize, u32)>>,
    /// The solution, when the generator knows it is the only one. The
    /// contradiction techniques then skip assuming its values, which never
    /// break a rule; their steps are the same.
    pub(crate) solution: Option<Vec<bool>>,
}

/// The shaded and unshaded cells of every row, as sets.
struct Bits {
    shaded: [u32; MAX_SIZE],
    unshaded: [u32; MAX_SIZE],
}

impl Bits {
    fn open(&self, r: usize, full: u32) -> u32 {
        full & !self.shaded[r] & !self.unshaded[r]
    }
}

/// An inclusive range of rows and columns.
#[derive(Clone, Copy)]
struct Rect {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

impl Rect {
    fn cell(i: usize, cols: usize) -> Self {
        let (r, c) = (i / cols, i % cols);
        Self {
            top: r,
            bottom: r,
            left: c,
            right: c,
        }
    }

    fn join(self, o: Self) -> Self {
        Self {
            top: self.top.min(o.top),
            bottom: self.bottom.max(o.bottom),
            left: self.left.min(o.left),
            right: self.right.max(o.right),
        }
    }

    /// The columns as a set.
    fn columns(self) -> u32 {
        ((1u32 << (self.right + 1)) - 1) & !((1u32 << self.left) - 1)
    }

    fn cells(self, cols: usize) -> impl Iterator<Item = usize> {
        (self.top..=self.bottom)
            .flat_map(move |r| (self.left..=self.right).map(move |c| r * cols + c))
    }
}

/// The shaded cells split into rectangles, read row by row: each starts at
/// its first cell, runs right as far as the row is shaded and down as far
/// as all of those columns are. When every group is a rectangle, as after
/// `routine`, these are the groups. Each rectangle's cells are shaded.
struct Blocks {
    /// Each cell's rectangle, or `u8::MAX`.
    of: [u8; MAX_SIZE * MAX_SIZE],
    rects: Vec<Rect>,
}

impl Grid {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        let (rows, cols) = (puzzle.shape().rows(), puzzle.shape().cols());
        let mut cells = vec![Vec::new(); puzzle.region_count()];
        for (i, &t) in puzzle.regions().iter().enumerate() {
            cells[t as usize].push(i);
        }
        let sets = (cells.iter())
            .map(|cells| {
                let mut sets: Vec<(usize, u32)> = Vec::new();
                for &i in cells {
                    match sets.last_mut() {
                        Some((r, set)) if *r == i / cols => *set |= 1 << (i % cols),
                        _ => sets.push((i / cols, 1 << (i % cols))),
                    }
                }
                sets
            })
            .collect();
        Self {
            rows,
            cols,
            nb: grid::neighbours(rows, cols),
            region: puzzle.regions().to_vec(),
            clues: puzzle.clues().to_vec(),
            cells,
            sets,
            solution: None,
        }
    }

    fn full(&self) -> u32 {
        (1 << self.cols) - 1
    }

    fn bits(&self, masks: &[u16]) -> Bits {
        let mut bits = Bits {
            shaded: [0; MAX_SIZE],
            unshaded: [0; MAX_SIZE],
        };
        for (r, row) in masks.chunks_exact(self.cols).enumerate() {
            for (c, &m) in row.iter().enumerate() {
                bits.shaded[r] |= u32::from(m == SHADED) << c;
                bits.unshaded[r] |= u32::from(m == UNSHADED) << c;
            }
        }
        bits
    }

    /// A region's shaded and open cells.
    fn counts(&self, bits: &Bits, t: usize) -> (u32, u32) {
        let full = self.full();
        self.sets[t].iter().fold((0, 0), |(s, o), &(r, set)| {
            (
                s + (bits.shaded[r] & set).count_ones(),
                o + (bits.open(r, full) & set).count_ones(),
            )
        })
    }

    fn blocks(&self, shaded: &[u32; MAX_SIZE]) -> Blocks {
        let mut blocks = Blocks {
            of: [u8::MAX; MAX_SIZE * MAX_SIZE],
            rects: Vec::new(),
        };
        let mut seen = [0u32; MAX_SIZE];
        for r in 0..self.rows {
            let mut todo = shaded[r] & !seen[r];
            while todo != 0 {
                let left = todo.trailing_zeros() as usize;
                let width = (!(todo >> left)).trailing_zeros() as usize;
                let set = ((1u32 << width) - 1) << left;
                let mut bottom = r;
                while bottom + 1 < self.rows && shaded[bottom + 1] & !seen[bottom + 1] & set == set
                {
                    bottom += 1;
                }
                let rect = Rect {
                    top: r,
                    bottom,
                    left,
                    right: left + width - 1,
                };
                for row in &mut seen[r..=bottom] {
                    *row |= set;
                }
                for i in rect.cells(self.cols) {
                    blocks.of[i] = blocks.rects.len() as u8;
                }
                blocks.rects.push(rect);
                todo &= !set;
            }
        }
        blocks
    }

    /// The cells of a rectangle that are unshaded.
    fn unshaded_in(&self, bits: &Bits, rect: Rect) -> Vec<usize> {
        (rect.top..=rect.bottom)
            .filter(|&r| bits.unshaded[r] & rect.columns() != 0)
            .flat_map(|r| {
                (rect.left..=rect.right)
                    .filter(move |&c| bits.unshaded[r] >> c & 1 == 1)
                    .map(move |c| r * self.cols + c)
            })
            .collect()
    }
}

/// The 2x2 blocks of rows `r` and `r + 1` with three shaded cells and one
/// unshaded, as a set of their left columns.
fn broken_blocks(bits: &Bits, r: usize, cols: usize) -> u32 {
    let (s0, s1) = (bits.shaded[r], bits.shaded[r + 1]);
    let (u0, u1) = (bits.unshaded[r], bits.unshaded[r + 1]);
    let (a, b, c, d) = (s0, s0 >> 1, s1, s1 >> 1);
    let (ua, ub, uc, ud) = (u0, u0 >> 1, u1, u1 >> 1);
    let broken = (ua & b & c & d) | (a & ub & c & d) | (a & b & uc & d) | (a & b & c & ud);
    broken & ((1 << (cols - 1)) - 1)
}

/// Whether the candidates already break a rule: a cell with no candidate,
/// a numbered region with more shaded cells than its clue or too few shaded
/// and open cells to reach it, or a 2x2 block with three shaded cells and
/// one unshaded. Never true while some completion keeps every rule; exact
/// on complete grids.
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    if masks.contains(&0) {
        return true;
    }
    let bits = grid.bits(masks);
    (0..grid.clues.len()).any(|t| {
        grid.clues[t].is_some_and(|clue| {
            let (shaded, open) = grid.counts(&bits, t);
            shaded > clue || shaded + open < clue
        })
    }) || (0..grid.rows.saturating_sub(1)).any(|r| broken_blocks(&bits, r, grid.cols) != 0)
}

/// Bookkeeping, in one step: a numbered region with all its shaded cells
/// gets its open cells unshaded, and one that needs all of them gets them
/// shaded. Every group's box is shaded.
fn routine(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    c.charge()?;
    let bits = g.bits(c.masks);
    let mut keep = vec![BOTH; c.masks.len()];
    let mut support = Vec::new();
    for t in 0..g.clues.len() {
        let Some(clue) = g.clues[t] else { continue };
        let (shaded, open) = g.counts(&bits, t);
        if open == 0 || shaded > clue || (shaded < clue && shaded + open > clue) {
            continue;
        }
        let value = if shaded == clue { UNSHADED } else { SHADED };
        for &i in &g.cells[t] {
            if c.masks[i] == BOTH {
                keep[i] &= value;
            } else {
                support.push(i);
            }
        }
    }
    // Fill each 2x2 block with three shaded cells until none is left:
    // every group then fills its box.
    let mut closed = bits.shaded;
    let width = g.full() >> 1;
    loop {
        let mut changed = false;
        for r in 0..g.rows.saturating_sub(1) {
            let (a, b, x, y) = (closed[r], closed[r] >> 1, closed[r + 1], closed[r + 1] >> 1);
            let top = (b & x & y & !a & width) | (a & x & y & !b & width) << 1;
            let bottom = (a & b & y & !x & width) | (a & b & x & !y & width) << 1;
            if top & !closed[r] != 0 || bottom & !closed[r + 1] != 0 {
                changed = true;
                closed[r] |= top;
                closed[r + 1] |= bottom;
            }
        }
        if !changed {
            break;
        }
    }
    let blocks = g.blocks(&closed);
    for rect in &blocks.rects {
        let rows = rect.top..=rect.bottom;
        if rows
            .clone()
            .any(|r| closed[r] & !bits.shaded[r] & rect.columns() != 0)
        {
            for i in rect.cells(g.cols) {
                if c.masks[i] == SHADED {
                    support.push(i);
                } else {
                    keep[i] &= SHADED;
                }
            }
        }
    }
    let changes: Vec<(usize, u16)> = (0..keep.len())
        .filter(|&i| keep[i] != BOTH)
        .map(|i| (i, keep[i]))
        .collect();
    Ok(c.step("routine", support, Vec::new(), changes)
        .into_iter()
        .collect())
}

/// One step for each open cell next to a group where `refute` finds a
/// reason it cannot be shaded. `refute` gets the box the cell's group
/// would have, joined with the groups beside it, and returns the cells that
/// show the problem.
fn per_open_cell(
    c: &Candidates<'_, Grid>,
    technique: &'static str,
    mut refute: impl FnMut(&Bits, Rect) -> Option<Vec<usize>>,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let bits = g.bits(c.masks);
    let blocks = g.blocks(&bits.shaded);
    let mut steps = Vec::new();
    for i in 0..c.masks.len() {
        if c.masks[i] != BOTH {
            continue;
        }
        let mut beside = [u8::MAX; 4];
        for (k, &j) in g.nb[i].iter().enumerate() {
            beside[k] = blocks.of[j];
        }
        if beside == [u8::MAX; 4] {
            continue;
        }
        c.charge()?;
        let joined = beside.iter().filter(|&&k| k != u8::MAX);
        let rect = joined.clone().fold(Rect::cell(i, g.cols), |r, &k| {
            r.join(blocks.rects[k as usize])
        });
        let Some(mut support) = refute(&bits, rect) else {
            continue;
        };
        for &k in joined {
            support.extend(blocks.rects[k as usize].cells(g.cols));
        }
        steps.extend(c.step(technique, support, Vec::new(), vec![(i, UNSHADED)]));
        if c.first_only && !steps.is_empty() {
            break;
        }
    }
    Ok(steps)
}

/// An open cell is unshaded if shading it would join it to groups whose
/// box, with the cell, covers an unshaded cell: they could not form one
/// rectangle.
fn bend(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    per_open_cell(c, "bend", |bits, rect| {
        let gaps = g.unshaded_in(bits, rect);
        (!gaps.is_empty()).then_some(gaps)
    })
}

/// An open cell is unshaded if shading it would fill a box, as for
/// `bend`, that puts more shaded cells in a numbered region than its clue.
fn overflow(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    per_open_cell(c, "overflow", |bits, rect| {
        // The cells the box would add to each region.
        let mut added: Vec<(u32, u32)> = Vec::new();
        for i in rect.cells(g.cols) {
            if bits.shaded[i / g.cols] >> (i % g.cols) & 1 == 1 {
                continue;
            }
            let t = g.region[i];
            match added.iter_mut().find(|(u, _)| *u == t) {
                Some((_, n)) => *n += 1,
                None => added.push((t, 1)),
            }
        }
        let (t, _) = added.into_iter().find(|&(t, n)| {
            g.clues[t as usize].is_some_and(|clue| g.counts(bits, t as usize).0 + n > clue)
        })?;
        // The region's shaded cells show that it would be over its clue.
        Some(
            (g.cells[t as usize].iter().copied())
                .filter(|&i| bits.shaded[i / g.cols] >> (i % g.cols) & 1 == 1)
                .collect(),
        )
    })
}

/// The number of ways to choose `k` of `n` things, capped at `cap + 1`.
fn choose(n: u64, k: u64, cap: u64) -> u64 {
    let k = k.min(n - k);
    let mut ways = 1u64;
    for j in 0..k {
        ways = ways * (n - j) / (j + 1);
        if ways > cap {
            return cap + 1;
        }
    }
    ways
}

/// The ways to shade a numbered region's open cells with exactly its need,
/// keeping the 2x2 rule among the decided cells. Cells shaded in every way
/// are shaded and cells shaded in no way are unshaded. Only regions with at
/// most [`MAX_SUBSETS`] subsets of the right size and [`MAX_WAYS`] ways are
/// compared.
fn combinations(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let bits = g.bits(c.masks);
    let mut steps = Vec::new();
    for t in 0..g.clues.len() {
        let Some(clue) = g.clues[t] else { continue };
        let (shaded, open) = g.counts(&bits, t);
        let Some(need) = clue.checked_sub(shaded) else {
            continue;
        };
        let (n, k) = (u64::from(open), u64::from(need));
        if k == 0 || k >= n || choose(n, k, MAX_SUBSETS) > MAX_SUBSETS {
            continue;
        }
        let open: Vec<usize> = (g.cells[t].iter().copied())
            .filter(|&i| c.masks[i] == BOTH)
            .collect();
        c.charge()?;
        // The rows whose 2x2 blocks touch the region's open cells.
        let (first, last) = (open[0] / g.cols, open[open.len() - 1] / g.cols);
        let (top, bottom) = (first.saturating_sub(1), (last + 1).min(g.rows - 1));
        let mut every = u64::MAX;
        let mut some = 0u64;
        let mut ways = 0;
        // Subsets of `k` open cells in increasing order, as bit sets over
        // `open`, by Gosper's hack.
        let mut subset: u64 = (1 << k) - 1;
        while subset < 1 << n {
            c.charge()?;
            let mut trial = Bits {
                shaded: bits.shaded,
                unshaded: bits.unshaded,
            };
            for (j, &i) in open.iter().enumerate() {
                let (r, col) = (i / g.cols, i % g.cols);
                if subset >> j & 1 == 1 {
                    trial.shaded[r] |= 1 << col;
                } else {
                    trial.unshaded[r] |= 1 << col;
                }
            }
            if (top..bottom).all(|r| broken_blocks(&trial, r, g.cols) == 0) {
                ways += 1;
                if ways > MAX_WAYS {
                    break;
                }
                every &= subset;
                some |= subset;
            }
            let low = subset & subset.wrapping_neg();
            let next = subset + low;
            subset = (((next ^ subset) >> 2) / low) | next;
        }
        if ways > MAX_WAYS {
            continue;
        }
        let changes: Vec<(usize, u16)> = if ways == 0 {
            // No way at all: the candidates are contradictory.
            vec![(open[0], 0)]
        } else {
            (open.iter().enumerate())
                .filter_map(|(j, &i)| {
                    if every >> j & 1 == 1 {
                        Some((i, SHADED))
                    } else if some >> j & 1 == 0 {
                        Some((i, UNSHADED))
                    } else {
                        None
                    }
                })
                .collect()
        };
        // The decided cells of the region and of the rows around it.
        let support = (top * g.cols..(bottom + 1) * g.cols)
            .filter(|&i| c.masks[i] != BOTH)
            .collect();
        steps.extend(c.step("combinations", support, vec![t], changes));
        if c.first_only && !steps.is_empty() {
            break;
        }
    }
    Ok(steps)
}

fn contradictions(
    c: &Candidates<'_, Grid>,
    technique: &'static str,
    max_steps: usize,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    // As `Profile::contradictions_where`, for open cells next to a decided
    // cell. A known solution's value is never refuted, so it is not tried.
    let g = c.context;
    let mut best = max_steps;
    let mut found = Vec::new();
    for i in 0..c.masks.len() {
        if c.masks[i] != BOTH || g.nb[i].iter().all(|&j| c.masks[j] == BOTH) {
            continue;
        }
        let (mut removed, mut longest, mut nodes) = (0, 0, 0);
        let mut support = vec![i];
        let mut units = Vec::new();
        for bit in [UNSHADED, SHADED] {
            if g.solution.as_ref().is_some_and(|s| (bit == SHADED) == s[i]) {
                continue;
            }
            c.charge()?;
            let Some(chain) = PROFILE.refute(c, i, bit, Medium, best)? else {
                continue;
            };
            removed |= bit;
            longest = longest.max(chain.len());
            for step in &chain {
                support.extend(&step.support);
                units.extend(&step.units);
                nodes += step.proof_nodes;
            }
        }
        if removed == 0 {
            continue;
        }
        units.sort_unstable();
        units.dedup();
        if let Some(mut step) = c.step(technique, support, units, vec![(i, !removed)]) {
            step.proof_nodes += nodes;
            best = best.min(longest);
            found.push((longest, step));
        }
    }
    Ok(found
        .into_iter()
        .filter(|&(length, _)| length == best)
        .map(|(_, step)| step)
        .collect())
}

const fn entry(
    id: &'static str,
    level: Difficulty,
    kind: Kind,
    probes: bool,
    find: Find<Grid>,
) -> Technique<Grid> {
    Technique {
        id,
        level,
        kind,
        probes,
        find,
    }
}

use Difficulty::{Easy, Expert, Hard, Medium};
use Kind::{Bookkeeping, Skill};

static TECHNIQUES: [Technique<Grid>; 6] = [
    entry("routine", Easy, Bookkeeping, false, routine),
    entry("bend", Easy, Skill, false, bend),
    entry("overflow", Easy, Skill, false, overflow),
    entry("combinations", Medium, Skill, false, combinations),
    entry("short_contradiction", Hard, Skill, true, |c| {
        contradictions(c, "short_contradiction", SHORT_CHAIN)
    }),
    entry("contradiction", Expert, Skill, true, |c| {
        contradictions(c, "contradiction", LONG_CHAIN)
    }),
];

/// The canonical Chocona human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "chocona-human",
    techniques: &TECHNIQUES,
    broken,
};

/// Every cell starts undecided.
pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    vec![BOTH; puzzle.regions().len()]
}

/// Solve from `masks` with techniques up to `cap`.
pub(crate) fn analyze_state(
    grid: &Grid,
    masks: Vec<u16>,
    budget: &Budget,
    cap: Difficulty,
    metrics: bool,
) -> Result<Trace, BudgetExceeded> {
    let placed = vec![false; masks.len()];
    PROFILE.analyze(grid, masks, placed, budget, cap, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Answer, Mark, Shape};
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    /// Whether a full shading (bit set = shaded) keeps every rule, by plain
    /// loops: region counts, and every group of shaded cells fills the
    /// rectangle around it.
    fn valid(p: &Puzzle, shaded: u32) -> bool {
        let s = p.shape();
        let (rows, cols) = (s.rows(), s.cols());
        let on = |i: usize| shaded >> i & 1 == 1;
        let counts = (0..p.region_count()).all(|t| {
            p.clues()[t].is_none_or(|clue| {
                (0..s.cells())
                    .filter(|&i| p.regions()[i] == t as u32 && on(i))
                    .count() as u32
                    == clue
            })
        });
        // Each group of shaded cells fills the rectangle around it.
        if !counts {
            return false;
        }
        let nb = grid::neighbours(rows, cols);
        let mut seen = 0u32;
        (0..s.cells()).filter(|&i| on(i)).all(|i| {
            if seen >> i & 1 == 1 {
                return true;
            }
            let group = grid::flood(i, on, &nb);
            for &j in &group {
                seen |= 1 << j;
            }
            let rs = group.iter().map(|&j| j / cols);
            let cs = group.iter().map(|&j| j % cols);
            let h = rs.clone().max().unwrap() - rs.min().unwrap() + 1;
            let w = cs.clone().max().unwrap() - cs.min().unwrap() + 1;
            h * w == group.len()
        })
    }

    fn masks_of(n: usize, shaded: u32) -> Vec<u16> {
        (0..n)
            .map(|i| {
                if shaded >> i & 1 == 1 {
                    SHADED
                } else {
                    UNSHADED
                }
            })
            .collect()
    }

    /// Random regions, each cell joining the region above or to the left or
    /// starting a new one, with random clues, some missing.
    fn random_puzzle(rng: &mut SplitMix64, shape: Shape) -> Puzzle {
        let cols = shape.cols();
        let mut labels: Vec<u32> = Vec::new();
        for i in 0..shape.cells() {
            let mut options = vec![i as u32];
            if i % cols > 0 {
                options.push(labels[i - 1]);
            }
            if i >= cols {
                options.push(labels[i - cols]);
            }
            labels.push(options[rng.below(options.len() as u64) as usize]);
        }
        let mut distinct = labels.clone();
        distinct.sort_unstable();
        distinct.dedup();
        let blank = Puzzle::new(shape, &labels, vec![Some(0); distinct.len()]).unwrap();
        let clues = (0..blank.region_count())
            .map(|t| {
                let size = blank.regions().iter().filter(|&&u| u == t as u32).count();
                (rng.below(4) > 0).then(|| rng.below(size as u64 + 1) as u32)
            })
            .collect();
        Puzzle::new(shape, &labels, clues).unwrap()
    }

    /// `broken` is exact on every full shading of a few small puzzles.
    #[test]
    fn broken_is_exact_on_full_grids() {
        let mut rng = SplitMix64::new(5);
        for shape in ["1x1", "1x3", "2x2", "2x3", "3x3", "3x4"] {
            let s: Shape = shape.parse().unwrap();
            for _ in 0..20 {
                let p = random_puzzle(&mut rng, s);
                let grid = Grid::new(&p);
                for shaded in 0..1u32 << s.cells() {
                    assert_eq!(
                        broken(&grid, &masks_of(s.cells(), shaded)),
                        !valid(&p, shaded),
                        "{} {shaded:b}",
                        p.id()
                    );
                }
            }
        }
    }

    /// Every technique keeps every solution of random partial states, and
    /// `broken` never fires while a solution remains (L23).
    #[test]
    fn techniques_keep_every_solution() {
        let mut rng = SplitMix64::new(17);
        let budget = Budget::unlimited();
        let mut fired = BTreeSet::new();
        let shapes: Vec<Shape> = ["2x2", "2x3", "3x3", "3x4", "4x3", "1x5", "4x4", "5x3"]
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
        let mut round = 0;
        while round < 3000 {
            let s = shapes[round % shapes.len()];
            let n = s.cells();
            let regions = random_puzzle(&mut rng, s);
            // Clues from a random shading that forms rectangles, so that
            // there is a solution.
            let Some(truth) = (0..60)
                .map(|_| {
                    let density = 1 + rng.below(3);
                    (0..n).fold(0u32, |b, i| b | u32::from(rng.below(4) < density) << i)
                })
                .find(|&b| valid(&regions.with_clues_of(&answer(s, b)), b))
            else {
                continue;
            };
            round += 1;
            let p = regions.with_clues_of(&answer(s, truth));
            let grid = Grid::new(&p);
            let solutions: Vec<u32> = (0..1u32 << n).filter(|&b| valid(&p, b)).collect();
            assert!(solutions.contains(&truth));
            // Partial candidates: some cells fixed to the truth.
            let masks: Vec<u16> = masks_of(n, truth)
                .into_iter()
                .map(|m| if rng.below(3) == 0 { m } else { BOTH })
                .collect();
            let fits: Vec<u32> = solutions
                .iter()
                .copied()
                .filter(|&b| masks_of(n, b).iter().zip(&masks).all(|(t, m)| t & m != 0))
                .collect();
            assert!(!broken(&grid, &masks), "broken although a solution fits");
            let placed = vec![false; n];
            let candidates = Candidates::new(&grid, &masks, &placed, &budget);
            for technique in &TECHNIQUES {
                for step in technique.run(&candidates).unwrap() {
                    fired.insert(step.technique);
                    for &(i, keep) in &step.changes {
                        for &f in &fits {
                            assert_ne!(
                                keep & masks_of(n, f)[i],
                                0,
                                "{} removed a true value from {} with {masks:?}",
                                step.technique,
                                p.id()
                            );
                        }
                    }
                }
            }
        }
        for technique in [
            "routine",
            "bend",
            "overflow",
            "combinations",
            "short_contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }

    /// The examples on the genre page: what a technique changes in a
    /// small state, given as region rows, clues and marks.
    #[test]
    fn documented_examples() {
        let budget = Budget::unlimited();
        let run = |text: &str, marks: &str, find: Find<Grid>| {
            let p: Puzzle = text.parse().unwrap();
            let grid = Grid::new(&p);
            let masks: Vec<u16> = (marks.chars())
                .filter(|c| !c.is_whitespace())
                .map(|c| match c {
                    '#' => SHADED,
                    '-' => UNSHADED,
                    _ => BOTH,
                })
                .collect();
            let placed = vec![false; masks.len()];
            let steps = find(&Candidates::new(&grid, &masks, &placed, &budget)).unwrap();
            let mut changes: Vec<_> = steps.into_iter().flat_map(|s| s.changes).collect();
            changes.sort_unstable();
            changes
        };
        let open = "regions:
ab
cd
clues: . . . .";
        assert_eq!(run(open, "## -?", bend), [(3, UNSHADED)]);
        assert_eq!(run(open, "#? -#", bend), [(1, UNSHADED)]);
        let text = "regions:
abb
abb
clues: 2 1";
        assert_eq!(
            run(text, "#?? #??", overflow),
            [(1, UNSHADED), (4, UNSHADED)]
        );
        let text = "regions:
aaa
aaa
clues: 3";
        assert_eq!(run(text, "??? --?", combinations), [(0, SHADED)]);
    }

    fn answer(s: Shape, shaded: u32) -> Answer {
        let mut a = Answer::new(s);
        for i in 0..s.cells() {
            let mark = if shaded >> i & 1 == 1 {
                Mark::Shaded
            } else {
                Mark::Unshaded
            };
            a.set(i / s.cols(), i % s.cols(), mark);
        }
        a
    }
}
