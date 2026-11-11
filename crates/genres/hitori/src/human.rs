//! Hitori's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order. Candidate bit 0 is unshaded
//! (`-`) and bit 1 shaded (`#`). A cell is fixed when one candidate is left.
//! The open area is every cell that may still be unshaded.
use crate::Puzzle;
use crate::board::peers;
use pencil_puzzles_core::grid;
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::search::Outcome;
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};

/// The unshaded candidate.
pub(crate) const WHITE: u16 = 1;
/// The shaded candidate.
pub(crate) const BLACK: u16 = 2;
/// Both candidates.
const BOTH: u16 = WHITE | BLACK;
/// Rounds of routine consequences `forced` looks ahead.
const FORCED_ROUNDS: usize = 2;
/// Longest Hard contradiction chain.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain.
const LONG_CHAIN: usize = 20;

/// The board's neighbours, numbers and same-number cells.
pub(crate) struct Grid {
    rows: usize,
    cols: usize,
    numbers: Vec<u32>,
    nb: Vec<Vec<usize>>,
    /// Other cells in the same row or column with the same number.
    peers: Vec<Vec<usize>>,
}

impl Grid {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        let shape = puzzle.shape();
        Self {
            rows: shape.rows(),
            cols: shape.cols(),
            numbers: puzzle.numbers().to_vec(),
            nb: grid::neighbours(shape.rows(), shape.cols()),
            peers: peers(puzzle),
        }
    }

    /// Whether two cells share a row.
    fn same_row(&self, i: usize, j: usize) -> bool {
        i / self.cols == j / self.cols
    }
}

/// The consequences of the fixed cells: open neighbours of shaded cells
/// lose shading, open peers of unshaded cells lose unshading. Returns the
/// cells used and the changes.
fn consequences(grid: &Grid, masks: &[u16]) -> (Vec<usize>, Vec<(usize, u16)>) {
    let mut support = Vec::new();
    let mut keep = vec![BOTH; masks.len()];
    for (i, &m) in masks.iter().enumerate() {
        let (others, value) = match m {
            BLACK => (&grid.nb[i], WHITE),
            WHITE => (&grid.peers[i], BLACK),
            _ => continue,
        };
        let mut used = false;
        for &j in others {
            if masks[j] == BOTH {
                keep[j] &= value;
                used = true;
            }
        }
        if used {
            support.push(i);
        }
    }
    let changes = (0..masks.len())
        .filter(|&j| keep[j] != BOTH)
        .map(|j| (j, keep[j]))
        .collect();
    (support, changes)
}

/// Whether the candidates already break a rule: a cell with no candidate,
/// two touching shaded cells, two unshaded cells with the same number in a
/// line, unshaded cells that cannot connect through the open area, or no
/// open area at all. Never true while some completion keeps every rule;
/// exact on complete grids.
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    if masks.contains(&0) {
        return true;
    }
    for (i, &m) in masks.iter().enumerate() {
        let clash = match m {
            BLACK => grid.nb[i].iter().any(|&j| masks[j] == BLACK),
            WHITE => grid.peers[i].iter().any(|&j| masks[j] == WHITE),
            _ => false,
        };
        if clash {
            return true;
        }
    }
    if masks.iter().all(|&m| m & WHITE == 0) {
        return true;
    }
    let Some(white) = masks.iter().position(|&m| m == WHITE) else {
        return false;
    };
    let area = grid::flood(white, |j| masks[j] & WHITE != 0, &grid.nb);
    let mut reached = vec![false; masks.len()];
    for j in area {
        reached[j] = true;
    }
    (0..masks.len()).any(|j| masks[j] == WHITE && !reached[j])
}

/// Bookkeeping: shaded cells' neighbours are unshaded, and unshaded cells'
/// same-number cells in their row and column are shaded. One step holds
/// every such consequence.
fn routine(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    c.charge()?;
    let (support, changes) = consequences(c.context, c.masks);
    Ok(c.step("routine", support, Vec::new(), changes)
        .into_iter()
        .collect())
}

/// A cell between two equal numbers in its row or column is unshaded: one
/// of the two is shaded, and shaded cells cannot touch.
fn sandwich(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let mut steps = Vec::new();
    for m in 0..c.masks.len() {
        if c.masks[m] != BOTH {
            continue;
        }
        c.charge()?;
        let (r, col) = (m / g.cols, m % g.cols);
        let mut ends = Vec::new();
        if col > 0 && col + 1 < g.cols && g.numbers[m - 1] == g.numbers[m + 1] {
            ends.push((m - 1, m + 1));
        }
        if r > 0 && r + 1 < g.rows && g.numbers[m - g.cols] == g.numbers[m + g.cols] {
            ends.push((m - g.cols, m + g.cols));
        }
        if let Some(&(a, b)) = ends.first() {
            steps.extend(c.step("sandwich", vec![a, b], Vec::new(), vec![(m, WHITE)]));
        }
    }
    Ok(steps)
}

/// Two touching equal numbers in a line: one of them stays unshaded, so
/// every other cell with that number in the line is shaded.
fn pair(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let mut steps = Vec::new();
    for a in 0..c.masks.len() {
        c.charge()?;
        let right = (a % g.cols + 1 < g.cols).then_some(a + 1);
        let below = (a / g.cols + 1 < g.rows).then_some(a + g.cols);
        for b in [right, below].into_iter().flatten() {
            if g.numbers[a] != g.numbers[b] {
                continue;
            }
            let row = g.same_row(a, b);
            let changes: Vec<(usize, u16)> = g.peers[a]
                .iter()
                .filter(|&&k| k != b && g.same_row(a, k) == row && c.masks[k] == BOTH)
                .map(|&k| (k, BLACK))
                .collect();
            steps.extend(c.step("pair", vec![a, b], Vec::new(), changes));
        }
    }
    Ok(steps)
}

/// A corner cell whose two neighbours hold its number is shaded: unshaded,
/// it would shade both neighbours and be cut off from the cell diagonal to
/// it, which is then unshaded.
fn corner(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let mut steps = Vec::new();
    if g.rows < 2 || g.cols < 2 {
        return Ok(steps);
    }
    let last = g.rows * g.cols - 1;
    for x in [0, g.cols - 1, last + 1 - g.cols, last] {
        c.charge()?;
        if c.masks[x] != BOTH {
            continue;
        }
        let nb = &g.nb[x];
        if nb.iter().all(|&j| g.numbers[j] == g.numbers[x]) {
            steps.extend(c.step("corner", nb.clone(), Vec::new(), vec![(x, BLACK)]));
        }
    }
    Ok(steps)
}

/// The cut cells of the open area: cells whose removal splits their part of
/// the area. Iterative (G7).
fn cut_cells(grid: &Grid, open: &[bool]) -> Vec<bool> {
    let n = open.len();
    let mut disc = vec![u32::MAX; n];
    let mut low = vec![0u32; n];
    let mut parent = vec![usize::MAX; n];
    let mut cut = vec![false; n];
    let mut time = 0u32;
    for root in 0..n {
        if !open[root] || disc[root] != u32::MAX {
            continue;
        }
        disc[root] = time;
        low[root] = time;
        time += 1;
        let mut children = 0;
        let mut stack = vec![(root, 0usize)];
        while let Some(&mut (v, ref mut k)) = stack.last_mut() {
            if let Some(&w) = grid.nb[v].get(*k) {
                *k += 1;
                if !open[w] {
                    continue;
                }
                if disc[w] == u32::MAX {
                    parent[w] = v;
                    disc[w] = time;
                    low[w] = time;
                    time += 1;
                    if v == root {
                        children += 1;
                    }
                    stack.push((w, 0));
                } else if w != parent[v] {
                    low[v] = low[v].min(disc[w]);
                }
            } else {
                stack.pop();
                if let Some(&(u, _)) = stack.last() {
                    low[u] = low[u].min(low[v]);
                    if u != root && low[v] >= disc[u] {
                        cut[u] = true;
                    }
                }
            }
        }
        if children > 1 {
            cut[root] = true;
        }
    }
    cut
}

/// An undecided cell whose shading would split the open area is unshaded:
/// its neighbours would be unshaded on different sides. So is the last
/// cell of the open area.
fn cut(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    c.charge()?;
    let open: Vec<bool> = c.masks.iter().map(|&m| m & WHITE != 0).collect();
    let cells = cut_cells(g, &open);
    let only = open.iter().filter(|&&o| o).count() == 1;
    let mut steps = Vec::new();
    for (x, &is_cut) in cells.iter().enumerate() {
        if c.masks[x] != BOTH || !(is_cut || only) {
            continue;
        }
        c.charge()?;
        // The open area around the cut: the cells it separates.
        let mut support = vec![x];
        for &j in &g.nb[x] {
            if open[j] {
                support.push(j);
            }
        }
        steps.extend(c.step("cut", support, Vec::new(), vec![(x, WHITE)]));
    }
    Ok(steps)
}

/// A value that breaks a rule within `FORCED_ROUNDS` rounds of its
/// routine consequences is removed. Returns one step per cell. Bookkeeping
/// has already applied the consequences of every fixed cell, so each round
/// only follows the cells fixed in the round before.
fn forced(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let mut steps = Vec::new();
    let mut masks = c.masks.to_vec();
    for x in 0..c.masks.len() {
        if c.masks[x] != BOTH {
            continue;
        }
        for value in [BLACK, WHITE] {
            c.charge()?;
            masks[x] = value;
            let mut support = vec![x];
            let mut fresh = vec![x];
            for _ in 0..FORCED_ROUNDS {
                let mut next = Vec::new();
                for &i in &fresh {
                    let (others, keep) = if masks[i] == BLACK {
                        (&g.nb[i], WHITE)
                    } else {
                        (&g.peers[i], BLACK)
                    };
                    for &j in others {
                        if masks[j] == BOTH {
                            masks[j] = keep;
                            next.push(j);
                        }
                    }
                }
                support.extend(&next);
                fresh = next;
            }
            let refuted = broken(g, &masks);
            for &i in &support {
                masks[i] = c.masks[i];
            }
            if refuted {
                steps.extend(c.step("forced", support, Vec::new(), vec![(x, BOTH & !value)]));
                break;
            }
        }
    }
    Ok(steps)
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

static TECHNIQUES: [Technique<Grid>; 8] = [
    entry("routine", Easy, Bookkeeping, false, routine),
    entry("sandwich", Easy, Skill, false, sandwich),
    entry("pair", Easy, Skill, false, pair),
    entry("corner", Easy, Skill, false, corner),
    entry("cut", Easy, Skill, false, cut),
    entry("forced", Medium, Skill, true, forced),
    entry("short_contradiction", Hard, Skill, true, |c| {
        PROFILE.contradictions(c, "short_contradiction", Easy, SHORT_CHAIN)
    }),
    entry("contradiction", Expert, Skill, true, |c| {
        PROFILE.contradictions(c, "contradiction", Easy, LONG_CHAIN)
    }),
];

/// The canonical Hitori human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "hitori-human",
    techniques: &TECHNIQUES,
    broken,
};

/// Every cell starts undecided.
pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    vec![BOTH; puzzle.numbers().len()]
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

/// The generator's view of a trace.
pub(crate) fn outcome(trace: &Trace) -> Outcome {
    trace.outcome()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Shape;
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    /// Whether a full shading (bit set = shaded) keeps every rule, by plain
    /// loops.
    fn valid(p: &Puzzle, shaded: u32) -> bool {
        let s = p.shape();
        let cols = s.cols();
        let n = s.cells();
        let black = |i: usize| shaded >> i & 1 == 1;
        let num = p.numbers();
        for i in 0..n {
            for j in i + 1..n {
                let same_line = i / cols == j / cols || i % cols == j % cols;
                if same_line && !black(i) && !black(j) && num[i] == num[j] {
                    return false;
                }
                let touching = (i / cols).abs_diff(j / cols) + (i % cols).abs_diff(j % cols) == 1;
                if touching && black(i) && black(j) {
                    return false;
                }
            }
        }
        let Some(start) = (0..n).find(|&i| !black(i)) else {
            return false;
        };
        let mut seen = vec![start];
        let mut at = 0;
        while at < seen.len() {
            let i = seen[at];
            at += 1;
            for j in 0..n {
                let touching = (i / cols).abs_diff(j / cols) + (i % cols).abs_diff(j % cols) == 1;
                if touching && !black(j) && !seen.contains(&j) {
                    seen.push(j);
                }
            }
        }
        seen.len() == (0..n).filter(|&i| !black(i)).count()
    }

    fn masks_of(n: usize, shaded: u32) -> Vec<u16> {
        (0..n)
            .map(|i| if shaded >> i & 1 == 1 { BLACK } else { WHITE })
            .collect()
    }

    fn random_puzzle(rng: &mut SplitMix64, shape: Shape) -> Puzzle {
        let m = u64::from(shape.largest());
        // Few distinct numbers make duplicates and patterns common.
        let k = 1 + rng.below(m);
        let numbers = (0..shape.cells())
            .map(|_| 1 + rng.below(k) as u32)
            .collect();
        Puzzle::new(shape, numbers).unwrap()
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
        let shapes: Vec<Shape> = ["2x2", "2x3", "3x3", "3x4", "4x3", "1x5", "4x4"]
            .iter()
            .map(|s| s.parse().unwrap())
            .collect();
        for round in 0..3000 {
            let s = shapes[round % shapes.len()];
            let n = s.cells();
            let p = random_puzzle(&mut rng, s);
            let grid = Grid::new(&p);
            let solutions: Vec<u32> = (0..1u32 << n).filter(|&b| valid(&p, b)).collect();
            if solutions.is_empty() {
                continue;
            }
            let truth = solutions[rng.below(solutions.len() as u64) as usize];
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
            "sandwich",
            "pair",
            "corner",
            "cut",
            "forced",
            "short_contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }

    #[test]
    fn cut_cells_of_a_line_and_a_ring() {
        let p = Puzzle::from_id("hitori:1x4:1234").unwrap();
        let grid = Grid::new(&p);
        assert_eq!(cut_cells(&grid, &[true; 4]), [false, true, true, false]);
        let ring = Puzzle::from_id("hitori:3x3:123231312").unwrap();
        let grid = Grid::new(&ring);
        let mut open = [true; 9];
        open[4] = false;
        assert_eq!(cut_cells(&grid, &open), [false; 9]);
        open[1] = false;
        assert_eq!(
            cut_cells(&grid, &open),
            [false, false, false, true, false, true, true, true, true]
        );
    }
}
