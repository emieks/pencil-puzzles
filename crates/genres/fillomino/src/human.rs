//! Fillomino's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order. Candidate bits 0 to 14 stand for
//! sizes 1 to 15, and bit 15 for "16 or more" (written `16+`). A cell is
//! fixed when one candidate is left. A group is a maximal connected set of
//! fixed cells with the same candidate; every group is part of one region,
//! except that a group of `16+` cells may hold several large regions.
use crate::Puzzle;
use pencil_puzzles_core::grid;
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::search::Outcome;
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};
use std::collections::BTreeSet;

/// The candidate standing for every size from 16 up.
pub(crate) const BIG: u16 = 1 << 15;
/// The size a `16+` region has at least.
const BIG_SIZE: usize = 16;
/// Longest Hard contradiction chain, using Easy and Medium techniques.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain, using techniques up to Hard.
const LONG_CHAIN: usize = 20;
/// Cells with at most this many candidates are tried in Hard chains.
const SHORT_CANDIDATES: u32 = 2;
/// Cells with at most this many candidates are tried in Expert chains.
const LONG_CANDIDATES: u32 = 3;
/// Largest leftover area whose fillings `pocket` tries, and the most
/// fillings it tries.
const POCKET: usize = 5;
const FILLINGS: u64 = 256;

/// The candidate for size `v`, with every size from 16 up sharing one.
pub(crate) fn bit(v: u32) -> u16 {
    if v as usize >= BIG_SIZE {
        BIG
    } else {
        1 << (v - 1)
    }
}

/// The least number of cells a region with candidate `b` has; exact below 16.
fn size_of(b: u16) -> usize {
    b.trailing_zeros() as usize + 1
}

fn bits(mask: u16) -> impl Iterator<Item = u16> {
    (0..16).map(|k| 1u16 << k).filter(move |b| mask & b != 0)
}

fn fixed(mask: u16) -> bool {
    mask.count_ones() == 1
}

/// The board's neighbours.
pub(crate) struct Grid {
    nb: Vec<Vec<usize>>,
}

impl Grid {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        Self {
            nb: grid::neighbours(puzzle.shape().rows(), puzzle.shape().cols()),
        }
    }
}

/// Groups of fixed cells with equal candidates.
struct Groups {
    of: Vec<Option<usize>>,
    cells: Vec<Vec<usize>>,
    value: Vec<u16>,
}

fn groups(grid: &Grid, masks: &[u16]) -> Groups {
    let n = masks.len();
    let mut of = vec![None; n];
    let mut cells = Vec::new();
    let mut value = Vec::new();
    for start in 0..n {
        if of[start].is_some() || !fixed(masks[start]) {
            continue;
        }
        let b = masks[start];
        let id = cells.len();
        of[start] = Some(id);
        let mut g = vec![start];
        let mut at = 0;
        while at < g.len() {
            for &j in &grid.nb[g[at]] {
                if of[j].is_none() && masks[j] == b {
                    of[j] = Some(id);
                    g.push(j);
                }
            }
            at += 1;
        }
        cells.push(g);
        value.push(b);
    }
    Groups { of, cells, value }
}

impl Groups {
    /// Whether group `g` has all its cells: exact sizes only.
    fn complete(&self, g: usize) -> bool {
        self.value[g] != BIG && self.cells[g].len() == size_of(self.value[g])
    }

    /// Whether group `g` must still grow.
    fn growing(&self, g: usize) -> bool {
        self.cells[g].len() < size_of(self.value[g])
    }
}

/// Open cells next to `cells` that still allow `b`, in order.
fn exits(grid: &Grid, masks: &[u16], cells: &[usize], b: u16) -> Vec<usize> {
    let mut out: Vec<usize> = cells
        .iter()
        .flat_map(|&i| grid.nb[i].iter().copied())
        .filter(|&j| !fixed(masks[j]) && masks[j] & b != 0)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Cells connected to `cells` through cells allowing `b`, skipping `avoid`,
/// stopping once `limit` cells are found.
fn room(
    grid: &Grid,
    masks: &[u16],
    cells: &[usize],
    b: u16,
    avoid: Option<usize>,
    limit: usize,
) -> Vec<usize> {
    grid::flood_from(
        cells,
        |j| Some(j) != avoid && masks[j] & b != 0,
        &grid.nb,
        limit,
    )
}

/// Whether the candidates already break a rule. Never true while some
/// completion keeps every rule; on grids with one candidate everywhere and
/// no `16+` cells, exactly "some group's size is not its number".
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    if masks.contains(&0) {
        return true;
    }
    let groups = groups(grid, masks);
    (0..groups.cells.len()).any(|g| {
        let (cells, b) = (&groups.cells[g], groups.value[g]);
        let need = size_of(b);
        (b != BIG && cells.len() > need)
            || (cells.len() < need && room(grid, masks, cells, b, None, need).len() < need)
    })
}

/// Bookkeeping: a complete group walls itself off; open neighbours lose its
/// size.
fn region_complete(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let g = groups(grid, c.masks);
    let mut steps = Vec::new();
    for k in 0..g.cells.len() {
        c.charge()?;
        if !g.complete(k) {
            continue;
        }
        let b = g.value[k];
        let changes = exits(grid, c.masks, &g.cells[k], b)
            .into_iter()
            .map(|j| (j, !b))
            .collect();
        steps.extend(c.step("region_complete", g.cells[k].clone(), Vec::new(), changes));
    }
    Ok(steps)
}

/// A size leaves a cell if writing it there would join groups of that size
/// into one with too many cells.
fn too_big(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let g = groups(grid, c.masks);
    let mut steps = Vec::new();
    for x in 0..c.masks.len() {
        if fixed(c.masks[x]) {
            continue;
        }
        let mut touching: Vec<usize> = grid.nb[x].iter().filter_map(|&j| g.of[j]).collect();
        if touching.is_empty() {
            continue;
        }
        c.charge()?;
        touching.sort_unstable();
        touching.dedup();
        let mut remove = 0;
        let mut support = vec![x];
        for &k in &touching {
            let b = g.value[k];
            if b == BIG || c.masks[x] & b == 0 || remove & b != 0 {
                continue;
            }
            let same = touching.iter().filter(|&&h| g.value[h] == b);
            let merged: usize = 1 + same.clone().map(|&h| g.cells[h].len()).sum::<usize>();
            if merged > size_of(b) {
                remove |= b;
                for &h in same {
                    support.extend(&g.cells[h]);
                }
            }
        }
        if remove != 0 {
            steps.extend(c.step("too_big", support, Vec::new(), vec![(x, !remove)]));
        }
    }
    Ok(steps)
}

/// A group that must grow, with one open neighbour that allows its size,
/// grows there.
fn only_exit(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let g = groups(grid, c.masks);
    let mut steps = Vec::new();
    for k in 0..g.cells.len() {
        c.charge()?;
        if !g.growing(k) {
            continue;
        }
        if let [y] = exits(grid, c.masks, &g.cells[k], g.value[k])[..] {
            let changes = vec![(y, g.value[k])];
            steps.extend(c.step("only_exit", g.cells[k].clone(), Vec::new(), changes));
        }
    }
    Ok(steps)
}

/// A group whose room (the cells it could spread to) has exactly as many
/// cells as its size fills all of it.
fn exact_room(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let g = groups(grid, c.masks);
    let mut steps = Vec::new();
    for k in 0..g.cells.len() {
        c.charge()?;
        let b = g.value[k];
        if b == BIG || !g.growing(k) {
            continue;
        }
        let need = size_of(b);
        let area = room(grid, c.masks, &g.cells[k], b, None, need + 1);
        if area.len() == need {
            let changes = area
                .iter()
                .filter(|&&j| !fixed(c.masks[j]))
                .map(|&j| (j, b))
                .collect();
            steps.extend(c.step("exact_room", area, Vec::new(), changes));
        }
    }
    Ok(steps)
}

/// A size leaves every area of cells allowing it that has too few cells
/// for a region that big.
fn no_room(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let n = c.masks.len();
    let open = c
        .masks
        .iter()
        .filter(|&&m| !fixed(m))
        .fold(0, |all, &m| all | m);
    let mut steps = Vec::new();
    let mut seen = vec![u16::MAX; n];
    let mut area = Vec::with_capacity(n);
    for (k, b) in bits(open).enumerate() {
        c.charge()?;
        let need = size_of(b);
        let mut support = Vec::new();
        let mut changes = Vec::new();
        for i in 0..n {
            if seen[i] == k as u16 || c.masks[i] & b == 0 {
                continue;
            }
            area.clear();
            area.push(i);
            seen[i] = k as u16;
            let mut at = 0;
            while at < area.len() {
                for &j in &grid.nb[area[at]] {
                    if seen[j] != k as u16 && c.masks[j] & b != 0 {
                        seen[j] = k as u16;
                        area.push(j);
                    }
                }
                at += 1;
            }
            if area.len() < need {
                support.extend(&area);
                changes.extend(
                    area.iter()
                        .filter(|&&j| !fixed(c.masks[j]))
                        .map(|&j| (j, !b)),
                );
            }
        }
        steps.extend(c.step("no_room", support, Vec::new(), changes));
    }
    Ok(steps)
}

/// A group that cannot reach its size without some open cell takes it.
fn must_include(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let g = groups(grid, c.masks);
    let mut steps = Vec::new();
    for k in 0..g.cells.len() {
        if !g.growing(k) {
            continue;
        }
        let (cells, b) = (&g.cells[k], g.value[k]);
        let need = size_of(b);
        // Only cells within reach of the missing cells can be needed.
        let area = room(grid, c.masks, cells, b, None, c.masks.len());
        let mut changes = Vec::new();
        let mut support = cells.clone();
        for &y in &area {
            if fixed(c.masks[y]) {
                continue;
            }
            c.charge()?;
            let without = room(grid, c.masks, cells, b, Some(y), need);
            if without.len() < need {
                changes.push((y, b));
                support.extend(&without);
            }
        }
        steps.extend(c.step("must_include", support, Vec::new(), changes));
    }
    Ok(steps)
}

/// A group missing one or two cells: try every way to finish it, joining
/// other groups of its size where they touch. Cells used by every way take
/// its size; cells next to the region in every way, and in none of them,
/// lose it.
fn completions(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let g = groups(grid, c.masks);
    let mut steps = Vec::new();
    for k in 0..g.cells.len() {
        c.charge()?;
        let (cells, b) = (&g.cells[k], g.value[k]);
        if b == BIG || !g.growing(k) || size_of(b) - cells.len() > 2 {
            continue;
        }
        let need = size_of(b);
        // The region after adding open cell `y`: with every group of the
        // same size that `y` touches.
        let join = |region: &[usize], y: usize| {
            let mut out = region.to_vec();
            out.push(y);
            for &j in &grid.nb[y] {
                if let Some(h) = g.of[j]
                    && g.value[h] == b
                    && !out.contains(&j)
                {
                    out.extend(&g.cells[h]);
                }
            }
            out
        };
        // Each way: the open cells added and the finished region.
        let mut ways: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
        for y in exits(grid, c.masks, cells, b) {
            let one = join(cells, y);
            if one.len() == need {
                ways.push((vec![y], one));
                continue;
            }
            if one.len() > need {
                continue;
            }
            for z in exits(grid, c.masks, &one, b) {
                if z == y || ways.iter().any(|(added, _)| added == &[z.min(y), z.max(y)]) {
                    continue;
                }
                let two = join(&one, z);
                if two.len() == need {
                    ways.push((vec![z.min(y), z.max(y)], two));
                }
            }
        }
        if ways.is_empty() {
            continue;
        }
        let mut in_all: Option<BTreeSet<usize>> = None;
        let mut beside_all: Option<BTreeSet<usize>> = None;
        let mut used = BTreeSet::new();
        let mut support = cells.clone();
        for (added, region) in &ways {
            used.extend(added.iter().copied());
            support.extend(region);
            let set: BTreeSet<usize> = added.iter().copied().collect();
            let beside: BTreeSet<usize> = exits(grid, c.masks, region, b).into_iter().collect();
            in_all = Some(match in_all {
                None => set,
                Some(s) => s.intersection(&set).copied().collect(),
            });
            beside_all = Some(match beside_all {
                None => beside,
                Some(s) => s.intersection(&beside).copied().collect(),
            });
        }
        let mut changes: Vec<(usize, u16)> = in_all
            .unwrap_or_default()
            .into_iter()
            .map(|y| (y, b))
            .collect();
        changes.extend(
            beside_all
                .unwrap_or_default()
                .into_iter()
                .filter(|j| !used.contains(j))
                .map(|j| (j, !b)),
        );
        steps.extend(c.step("completions", support, Vec::new(), changes));
    }
    Ok(steps)
}

/// Leftover areas of open cells, surrounded by fixed cells and the edge,
/// with at most `largest` cells: try every way to fill one, and keep the
/// sizes some way allows without breaking a rule.
fn fill_areas(
    c: &Candidates<'_, Grid>,
    technique: &'static str,
    largest: usize,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let n = c.masks.len();
    let mut seen = vec![false; n];
    let mut steps = Vec::new();
    for start in 0..n {
        if seen[start] || fixed(c.masks[start]) {
            continue;
        }
        let area = grid::flood(start, |j| !fixed(c.masks[j]), &grid.nb);
        for &j in &area {
            seen[j] = true;
        }
        let fillings: Option<u64> = area
            .iter()
            .map(|&j| u64::from(c.masks[j].count_ones()))
            .try_fold(1u64, |acc, k| acc.checked_mul(k).filter(|&p| p <= FILLINGS));
        if area.len() > largest || fillings.is_none() {
            continue;
        }
        let options: Vec<Vec<u16>> = area.iter().map(|&j| bits(c.masks[j]).collect()).collect();
        let mut kept = vec![0u16; area.len()];
        let mut at = vec![0usize; area.len()];
        let mut masks = c.masks.to_vec();
        loop {
            c.charge()?;
            for (k, &j) in area.iter().enumerate() {
                masks[j] = options[k][at[k]];
            }
            if !broken(grid, &masks) {
                for k in 0..area.len() {
                    kept[k] |= options[k][at[k]];
                }
            }
            let mut k = 0;
            while k < at.len() && at[k] + 1 == options[k].len() {
                at[k] = 0;
                k += 1;
            }
            if k == at.len() {
                break;
            }
            at[k] += 1;
        }
        let mut support = area.clone();
        for &j in &area {
            support.extend(grid.nb[j].iter().filter(|&&i| fixed(c.masks[i])));
        }
        let changes = area
            .iter()
            .zip(&kept)
            .map(|(&j, &keep)| (j, keep))
            .collect();
        steps.extend(c.step(technique, support, Vec::new(), changes));
    }
    Ok(steps)
}

/// Whether an open cell touches a fixed one.
fn frontier(c: &Candidates<'_, Grid>, x: usize) -> bool {
    c.context.nb[x].iter().any(|&j| fixed(c.masks[j]))
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

static TECHNIQUES: [Technique<Grid>; 12] = [
    entry("region_complete", Easy, Bookkeeping, false, region_complete),
    entry("too_big", Easy, Skill, false, too_big),
    entry("only_exit", Easy, Skill, false, only_exit),
    entry("exact_room", Easy, Skill, false, exact_room),
    entry("no_room", Easy, Skill, false, no_room),
    entry("pair", Easy, Skill, false, |c| fill_areas(c, "pair", 2)),
    entry("must_include", Medium, Skill, false, must_include),
    entry("completions", Medium, Skill, false, completions),
    entry("pocket", Medium, Skill, false, |c| {
        fill_areas(c, "pocket", POCKET)
    }),
    // A size that breaks a rule as soon as it is written in: no chain.
    entry("forced", Medium, Skill, false, |c| {
        PROFILE.contradictions_where(c, "forced", Easy, 0, |x| frontier(c, x))
    }),
    entry("short_contradiction", Hard, Skill, true, |c| {
        PROFILE.contradictions_where(c, "short_contradiction", Medium, SHORT_CHAIN, |x| {
            c.masks[x].count_ones() <= SHORT_CANDIDATES
        })
    }),
    entry("contradiction", Expert, Skill, true, |c| {
        PROFILE.contradictions_where(c, "contradiction", Hard, LONG_CHAIN, |x| {
            c.masks[x].count_ones() <= LONG_CANDIDATES
        })
    }),
];

/// The canonical Fillomino human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "fillomino-human",
    techniques: &TECHNIQUES,
    broken,
};

/// Printed sizes are fixed; every other cell starts with every size the
/// grid has room for.
pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    let cells = puzzle.clues().len() as u32;
    let all = (1..=cells.min(BIG_SIZE as u32)).fold(0, |m, v| m | bit(v));
    puzzle
        .clues()
        .iter()
        .map(|&n| if n > 0 { bit(n) } else { all })
        .collect()
}

/// Solve from `masks` with techniques up to `cap`.
pub(crate) fn analyze_state(
    puzzle: &Puzzle,
    masks: Vec<u16>,
    budget: &Budget,
    cap: Difficulty,
    metrics: bool,
) -> Result<Trace, BudgetExceeded> {
    let placed = vec![false; masks.len()];
    PROFILE.analyze(&Grid::new(puzzle), masks, placed, budget, cap, metrics)
}

/// Whether a trace solved the puzzle with an exact size in every cell.
pub(crate) fn solved(trace: &Trace) -> bool {
    trace.solved && !trace.masks.contains(&BIG)
}

/// The generator's view of a trace: cells left at `16+` count as unresolved.
pub(crate) fn outcome(trace: &Trace) -> Outcome {
    if trace.solved && trace.masks.contains(&BIG) {
        return Outcome::Stuck(
            (0..trace.masks.len())
                .filter(|&i| trace.masks[i] == BIG)
                .collect(),
        );
    }
    trace.outcome()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Shape;
    use pencil_puzzles_core::SplitMix64;

    /// Every full assignment of sizes 1 to the cell count that keeps R1,
    /// by brute force with plain loops.
    fn fillings(shape: Shape) -> Vec<Vec<u32>> {
        let n = shape.cells();
        let cols = shape.cols();
        let nb = grid::neighbours(shape.rows(), cols);
        let mut out = Vec::new();
        let mut v = vec![1u32; n];
        loop {
            let ok = (0..n).all(|i| grid::flood(i, |j| v[j] == v[i], &nb).len() as u32 == v[i]);
            if ok {
                out.push(v.clone());
            }
            let mut k = 0;
            while k < n && v[k] == n as u32 {
                v[k] = 1;
                k += 1;
            }
            if k == n {
                return out;
            }
            v[k] += 1;
        }
    }

    /// `broken` is exact on every full grid of a few small shapes.
    #[test]
    fn broken_is_exact_on_full_grids() {
        for shape in ["1x3", "2x2", "2x3"] {
            let s: Shape = shape.parse().unwrap();
            let p = Puzzle::new(s, vec![0; s.cells()]).unwrap();
            let grid = Grid::new(&p);
            let valid = fillings(s);
            let n = s.cells();
            let mut v = vec![1u32; n];
            loop {
                let masks: Vec<u16> = v.iter().map(|&x| bit(x)).collect();
                assert_eq!(broken(&grid, &masks), !valid.contains(&v), "{v:?}");
                let mut k = 0;
                while k < n && v[k] == n as u32 {
                    v[k] = 1;
                    k += 1;
                }
                if k == n {
                    break;
                }
                v[k] += 1;
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
        let shapes: Vec<Shape> = ["2x3", "3x2", "1x6", "2x4", "1x7"]
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
        let all: Vec<Vec<Vec<u32>>> = shapes.iter().map(|&s| fillings(s)).collect();
        for round in 0..1500 {
            let k = round % shapes.len();
            let s = shapes[k];
            let truth = &all[k][rng.below(all[k].len() as u64) as usize];
            let clues: Vec<u32> = truth
                .iter()
                .map(|&v| if rng.below(3) == 0 { v } else { 0 })
                .collect();
            let p = Puzzle::new(s, clues).unwrap();
            let grid = Grid::new(&p);
            let fits_clues: Vec<&Vec<u32>> = all[k]
                .iter()
                .filter(|f| {
                    p.clues()
                        .iter()
                        .zip(f.iter())
                        .all(|(&c, &v)| c == 0 || c == v)
                })
                .collect();
            // Partial candidates: the truth plus random other sizes.
            let masks: Vec<u16> = initial_masks(&p)
                .into_iter()
                .zip(truth)
                .map(|(m, &v)| {
                    if fixed(m) || rng.below(3) == 0 {
                        m
                    } else {
                        (m & (rng.next_u64() as u16)) | bit(v)
                    }
                })
                .collect();
            let fits: Vec<&&Vec<u32>> = fits_clues
                .iter()
                .filter(|f| (0..masks.len()).all(|i| masks[i] & bit(f[i]) != 0))
                .collect();
            assert!(!broken(&grid, &masks), "broken although a solution fits");
            let placed = vec![false; masks.len()];
            let candidates = Candidates::new(&grid, &masks, &placed, &budget);
            for technique in &TECHNIQUES {
                for step in technique.run(&candidates).unwrap() {
                    fired.insert(step.technique);
                    for &(i, keep) in &step.changes {
                        for f in &fits {
                            assert_ne!(
                                keep & bit(f[i]),
                                0,
                                "{} removed a true size from {} with {masks:?}",
                                step.technique,
                                p.id()
                            );
                        }
                    }
                }
            }
        }
        for technique in [
            "region_complete",
            "too_big",
            "only_exit",
            "exact_room",
            "pair",
            "no_room",
            "must_include",
            "completions",
            "pocket",
            "forced",
            "short_contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }

    /// Time each technique along one canonical solve:
    /// `cargo test --release -p pencil-puzzles-fillomino timing -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn timing() {
        use std::time::Instant;
        let mut o = crate::GenerateOptions::new("10x10".parse().unwrap(), 0);
        o.difficulty = Some(Difficulty::Medium);
        let p = crate::generate(&o, &Budget::unlimited()).unwrap().puzzle;
        let grid = Grid::new(&p);
        let budget = Budget::unlimited();
        let mut masks = initial_masks(&p);
        let placed = vec![false; masks.len()];
        let mut time = vec![0u128; TECHNIQUES.len()];
        let mut runs = vec![0u32; TECHNIQUES.len()];
        let mut broken_time = 0;
        loop {
            let t = Instant::now();
            let stop = masks.iter().all(|&m| fixed(m)) || broken(&grid, &masks);
            broken_time += t.elapsed().as_micros();
            if stop {
                break;
            }
            let c = Candidates::new(&grid, &masks, &placed, &budget);
            let mut step = None;
            for (k, t) in TECHNIQUES.iter().enumerate() {
                let start = Instant::now();
                let found = t.run(&c).unwrap();
                time[k] += start.elapsed().as_micros();
                runs[k] += 1;
                if let Some(s) = found.into_iter().next() {
                    step = Some(s);
                    break;
                }
            }
            let Some(step) = step else { break };
            for &(i, keep) in &step.changes {
                masks[i] &= keep;
            }
        }
        println!("broken {broken_time} us");
        for (k, t) in TECHNIQUES.iter().enumerate() {
            println!("{:20} {:5} runs {:8} us", t.id, runs[k], time[k]);
        }
    }
}
