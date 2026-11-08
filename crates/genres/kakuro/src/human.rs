//! Kakuro's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are the white cells in row-major order. Candidate bit `d - 1`
//! stands for digit `d`. A cell is fixed when one candidate is left. The
//! combinations of a run are the sets of different digits with as many
//! digits as the run has cells, adding up to its sum.
use crate::Puzzle;
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::search::Outcome;
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};

/// Every digit.
pub(crate) const DIGITS: u16 = 0x1FF;
/// Longest Hard contradiction chain, using Easy and Medium techniques.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain, using techniques up to Hard.
const LONG_CHAIN: usize = 20;
/// Cells with at most this many candidates are tried in Hard chains.
const SHORT_CANDIDATES: u32 = 3;
/// Cells with at most this many candidates are tried in Expert chains.
const LONG_CANDIDATES: u32 = 4;

fn fixed(mask: u16) -> bool {
    mask.count_ones() == 1
}

/// The digit of a fixed mask.
fn digit(mask: u16) -> u32 {
    mask.trailing_zeros() + 1
}

fn bits(mask: u16) -> impl Iterator<Item = u16> {
    (0..9).map(|k| 1u16 << k).filter(move |b| mask & b != 0)
}

/// The runs and the combinations of every length and sum.
pub(crate) struct Grid {
    /// The cell of each variable.
    pub(crate) whites: Vec<usize>,
    /// The variables of each run, in order.
    runs: Vec<Vec<usize>>,
    sums: Vec<u32>,
    /// The two runs of each variable.
    runs_of: Vec<[usize; 2]>,
    /// The digit sets of each size and sum: `combos[size][sum]`.
    combos: Vec<Vec<Vec<u16>>>,
    /// The sum of each digit set.
    total: Vec<u32>,
}

impl Grid {
    pub(crate) fn new(p: &Puzzle) -> Self {
        let n = p.cells().len();
        let mut var = vec![usize::MAX; n];
        let mut whites = Vec::new();
        for (i, &c) in p.cells().iter().enumerate() {
            if c == crate::Cell::White {
                var[i] = whites.len();
                whites.push(i);
            }
        }
        let mut runs_of = vec![[0; 2]; whites.len()];
        let mut runs = Vec::new();
        let mut sums = Vec::new();
        for (k, run) in p.runs().iter().enumerate() {
            let vars: Vec<usize> = run.cells.iter().map(|&i| var[i]).collect();
            for &v in &vars {
                runs_of[v][usize::from(run.down)] = k;
            }
            runs.push(vars);
            sums.push(run.sum);
        }
        let mut combos = vec![vec![Vec::new(); 46]; 10];
        let mut total = vec![0; 512];
        for set in 0..512u16 {
            let s: u32 = bits(set).map(digit).sum();
            total[usize::from(set)] = s;
            combos[set.count_ones() as usize][s as usize].push(set);
        }
        Self {
            whites,
            runs,
            sums,
            runs_of,
            combos,
            total,
        }
    }

    /// The other variables of `x`'s two runs.
    fn mates(&self, x: usize) -> impl Iterator<Item = usize> + '_ {
        self.runs_of[x]
            .iter()
            .flat_map(move |&k| self.runs[k].iter().copied())
            .filter(move |&j| j != x)
    }

    /// The fixed digits of run `k`, or `None` if one repeats.
    fn fixed_digits(&self, masks: &[u16], k: usize) -> Option<u16> {
        let mut used = 0;
        for &v in &self.runs[k] {
            let m = masks[v];
            if fixed(m) {
                if used & m != 0 {
                    return None;
                }
                used |= m;
            }
        }
        Some(used)
    }

    /// The combinations of run `k` containing the digits `used`.
    fn combinations(&self, k: usize, used: u16) -> impl Iterator<Item = u16> + '_ {
        self.combos[self.runs[k].len()][self.sums[k] as usize]
            .iter()
            .copied()
            .filter(move |&c| c & used == used)
    }
}

/// Whether the candidates already break a rule: a cell has no candidate, a
/// run repeats a fixed digit, or no different digits unused in a run can
/// fill its open cells to reach the rest of its sum. With one candidate
/// everywhere, exactly "some run breaks R1".
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    if masks.contains(&0) {
        return true;
    }
    (0..grid.runs.len()).any(|k| {
        let (mut used, mut left, mut open) = (0u16, grid.sums[k], 0);
        for &v in &grid.runs[k] {
            let m = masks[v];
            if !fixed(m) {
                open += 1;
                continue;
            }
            if used & m != 0 || digit(m) > left {
                return true;
            }
            used |= m;
            left -= digit(m);
        }
        !grid.combos[open][left as usize]
            .iter()
            .any(|&c| c & used == 0)
    })
}

/// Bookkeeping: a written-in digit leaves the other cells of its runs.
fn digit_used(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let mut steps = Vec::new();
    for x in 0..c.masks.len() {
        c.charge()?;
        let m = c.masks[x];
        if !c.placed[x] || !fixed(m) {
            continue;
        }
        let changes = grid.mates(x).map(|j| (j, !m)).collect();
        if let Some(step) = c.step("digit_used", vec![x], grid.runs_of[x].to_vec(), changes) {
            steps.push(step);
            if c.first_only {
                break;
            }
        }
    }
    Ok(steps)
}

/// A cell with one candidate is written in; its digit leaves its runs.
fn single(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let mut steps = Vec::new();
    for x in 0..c.masks.len() {
        c.charge()?;
        let m = c.masks[x];
        if c.placed[x] || !fixed(m) {
            continue;
        }
        let changes = grid.mates(x).map(|j| (j, !m)).collect();
        if let Some(mut step) = c.step("single", vec![x], grid.runs_of[x].to_vec(), changes) {
            step.places = Some((x, m));
            steps.push(step);
            if c.first_only {
                break;
            }
        }
    }
    Ok(steps)
}

/// A run's open cells keep only digits of its combinations that contain
/// its fixed digits, and lose the fixed digits.
fn sum_combinations(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let mut steps = Vec::new();
    for k in 0..grid.runs.len() {
        c.charge()?;
        let Some(used) = grid.fixed_digits(c.masks, k) else {
            continue;
        };
        let union = grid.combinations(k, used).fold(0, |a, b| a | b);
        let vars = &grid.runs[k];
        let support = vars
            .iter()
            .copied()
            .filter(|&v| fixed(c.masks[v]))
            .collect();
        let changes = vars
            .iter()
            .filter(|&&v| !fixed(c.masks[v]))
            .map(|&v| (v, union & !used))
            .collect();
        if let Some(step) = c.step("sum_combinations", support, vec![k], changes) {
            steps.push(step);
            if c.first_only {
                break;
            }
        }
    }
    Ok(steps)
}

/// A digit that every combination of a run needs, with only one open cell
/// of the run allowing it, goes in that cell.
fn only_cell(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let mut steps = Vec::new();
    for k in 0..grid.runs.len() {
        c.charge()?;
        let Some(used) = grid.fixed_digits(c.masks, k) else {
            continue;
        };
        let required = grid.combinations(k, used).fold(DIGITS, |a, b| a & b);
        let vars = &grid.runs[k];
        for bit in bits(required & !used) {
            let mut holders = vars
                .iter()
                .copied()
                .filter(|&v| !fixed(c.masks[v]) && c.masks[v] & bit != 0);
            let (Some(x), None) = (holders.next(), holders.next()) else {
                continue;
            };
            if let Some(mut step) = c.step("only_cell", vars.clone(), vec![k], vec![(x, bit)]) {
                step.places = Some((x, bit));
                steps.push(step);
                if c.first_only {
                    return Ok(steps);
                }
            }
        }
    }
    Ok(steps)
}

/// A combination of a run survives if its digits can go one per cell
/// within the candidates; each cell keeps the digits it gets in some
/// surviving way.
fn combination_fit(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let mut steps = Vec::new();
    let (mut seen, mut good, mut stamp) = (vec![0u32; 512], vec![0u32; 512], 0u32);
    // The digit sets the first i cells of a run can take, reused per run.
    let mut lists: Vec<Vec<u16>> = vec![Vec::new(); crate::MAX_RUN + 1];
    for k in 0..grid.runs.len() {
        c.charge()?;
        let vars = &grid.runs[k];
        let sum = grid.sums[k];
        let len = vars.len();
        if vars.iter().all(|&v| fixed(c.masks[v])) {
            continue;
        }
        // The digit sets the first i cells can take, one digit each. Sets
        // of different sizes never collide, so one mark per run tells
        // which sets were seen, and another which can still be finished.
        stamp += 1;
        lists[0].clear();
        lists[0].push(0);
        for i in 0..len {
            let m = c.masks[vars[i]];
            let (done, rest) = lists.split_at_mut(i + 1);
            let next = &mut rest[0];
            next.clear();
            for &set in &done[i] {
                for bit in bits(m & !set) {
                    let t = usize::from(set | bit);
                    if grid.total[t] <= sum && seen[t] != stamp {
                        seen[t] = stamp;
                        next.push(set | bit);
                    }
                }
            }
        }
        // Back from the full sets with the right sum: which sets can still
        // be finished, and with which digit in each cell.
        for &set in &lists[len] {
            if grid.total[usize::from(set)] == sum {
                good[usize::from(set)] = stamp;
            }
        }
        let mut keep = [0u16; crate::MAX_RUN];
        for i in (0..len).rev() {
            let m = c.masks[vars[i]];
            for &set in &lists[i] {
                for bit in bits(m & !set) {
                    if good[usize::from(set | bit)] == stamp {
                        good[usize::from(set)] = stamp;
                        keep[i] |= bit;
                    }
                }
            }
        }
        let changes = vars.iter().copied().zip(keep).collect();
        if let Some(step) = c.step("combination_fit", vars.clone(), vec![k], changes) {
            steps.push(step);
            if c.first_only {
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

pub(crate) static TECHNIQUES: [Technique<Grid>; 7] = [
    entry("digit_used", Easy, Bookkeeping, false, digit_used),
    entry("single", Easy, Skill, false, single),
    entry("sum_combinations", Easy, Skill, false, sum_combinations),
    entry("only_cell", Easy, Skill, false, only_cell),
    entry("combination_fit", Medium, Skill, false, combination_fit),
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

/// The canonical Kakuro human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "kakuro-human",
    version: 1,
    techniques: &TECHNIQUES,
    broken,
};

/// Every white cell starts with the digits found in some combination of
/// each of its two runs.
pub(crate) fn initial_masks(grid: &Grid) -> Vec<u16> {
    grid.runs_of
        .iter()
        .map(|runs| {
            runs.iter()
                .map(|&k| grid.combinations(k, 0).fold(0, |a, b| a | b))
                .fold(DIGITS, |a, b| a & b)
        })
        .collect()
}

/// Solve from `masks` with techniques up to `cap`.
pub(crate) fn analyze(
    grid: &Grid,
    masks: Vec<u16>,
    placed: Vec<bool>,
    budget: &Budget,
    cap: Difficulty,
) -> Result<Trace, BudgetExceeded> {
    PROFILE.analyze(grid, masks, placed, budget, cap, false)
}

/// The generator's view of a capped solve of a puzzle.
pub(crate) fn screen(
    p: &Puzzle,
    budget: &Budget,
    cap: Difficulty,
) -> Result<Outcome, BudgetExceeded> {
    let grid = Grid::new(p);
    let masks = initial_masks(&grid);
    let placed = vec![false; masks.len()];
    Ok(analyze(&grid, masks, placed, budget, cap)?.outcome())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cell, Shape};
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    /// A shape whose cells are white from row 1 and column 1 on, except
    /// `blocks`.
    fn layout(shape: &str, blocks: &[(usize, usize)]) -> (Shape, Vec<bool>) {
        let s: Shape = shape.parse().unwrap();
        let white = (0..s.cells())
            .map(|i| {
                let (r, c) = (i / s.cols(), i % s.cols());
                r > 0 && c > 0 && !blocks.contains(&(r, c))
            })
            .collect();
        (s, white)
    }

    /// The puzzle with the sums of `digits` (0 on blocks).
    fn puzzle(s: Shape, white: &[bool], digits: &[u8]) -> Puzzle {
        let cols = s.cols();
        let run_sum = |start: usize, step: usize| -> Option<u8> {
            let mut i = start + step;
            let mut sum = 0;
            while i < white.len() && white[i] && (step == cols || i / cols == start / cols) {
                sum += digits[i];
                i += step;
            }
            (sum > 0).then_some(sum)
        };
        let cells = (0..s.cells())
            .map(|i| {
                if white[i] {
                    return Cell::White;
                }
                let down = run_sum(i, cols);
                let across = if i % cols + 1 < cols {
                    run_sum(i, 1)
                } else {
                    None
                };
                Cell::Block { down, across }
            })
            .collect();
        Puzzle::new(s, cells).unwrap()
    }

    /// Every way to put digits in the white cells with no digit repeated
    /// in a row or column segment, by plain loops.
    fn fillings(s: Shape, white: &[bool]) -> Vec<Vec<u8>> {
        let cols = s.cols();
        let cells: Vec<usize> = (0..s.cells()).filter(|&i| white[i]).collect();
        let segment = |i: usize, j: usize| {
            let (a, b) = (i.min(j), i.max(j));
            if a / cols == b / cols {
                (a..=b).all(|k| white[k])
            } else if a % cols == b % cols {
                (a..=b).step_by(cols).all(|k| white[k])
            } else {
                false
            }
        };
        let mut out = Vec::new();
        let mut v = vec![1u8; cells.len()];
        loop {
            let ok = (0..cells.len())
                .all(|x| (0..x).all(|y| v[x] != v[y] || !segment(cells[x], cells[y])));
            if ok {
                let mut digits = vec![0; s.cells()];
                for (k, &i) in cells.iter().enumerate() {
                    digits[i] = v[k];
                }
                out.push(digits);
            }
            let mut k = 0;
            while k < v.len() && v[k] == 9 {
                v[k] = 1;
                k += 1;
            }
            if k == v.len() {
                return out;
            }
            v[k] += 1;
        }
    }

    fn layouts() -> Vec<(Shape, Vec<bool>)> {
        vec![layout("3x3", &[]), layout("3x4", &[]), layout("4x3", &[])]
    }

    /// The fillings of a layout grouped by the puzzle their sums make.
    fn by_puzzle(s: Shape, white: &[bool]) -> Vec<(Puzzle, Vec<Vec<u8>>)> {
        let mut groups: std::collections::BTreeMap<String, (Puzzle, Vec<Vec<u8>>)> =
            Default::default();
        for f in fillings(s, white) {
            let p = puzzle(s, white, &f);
            groups.entry(p.id()).or_insert((p, Vec::new())).1.push(f);
        }
        groups.into_values().collect()
    }

    /// Screening stops each technique at its first step; the applied steps
    /// are the same as when every step is listed for the metrics.
    #[test]
    fn screening_applies_the_same_steps_as_rating() {
        for shape in ["6x6", "8x8"] {
            for level in [Easy, Medium, Hard, Expert] {
                for seed in 0..3 {
                    let mut o = crate::GenerateOptions::new(shape.parse().unwrap(), seed);
                    o.difficulty = Some(level);
                    let p = crate::generate(&o, &Budget::new(100_000_000))
                        .unwrap()
                        .puzzle;
                    let grid = Grid::new(&p);
                    for cap in [Easy, Medium, Hard, Expert] {
                        let budget = Budget::unlimited();
                        let run = |metrics| {
                            PROFILE
                                .analyze(
                                    &grid,
                                    initial_masks(&grid),
                                    vec![false; grid.whites.len()],
                                    &budget,
                                    cap,
                                    metrics,
                                )
                                .unwrap()
                        };
                        let (full, quick) = (run(true), run(false));
                        assert_eq!(full.steps, quick.steps, "{} {cap:?}", p.id());
                        assert_eq!(full.masks, quick.masks);
                        assert_eq!(full.solved, quick.solved);
                    }
                }
            }
        }
    }

    /// `broken` is exact on complete grids: false for every filling of
    /// the sums it was made from, true when a digit changes and breaks it.
    #[test]
    fn broken_is_exact_on_complete_grids() {
        let mut rng = SplitMix64::new(5);
        for (s, white) in layouts() {
            let all = by_puzzle(s, &white);
            for _ in 0..200 {
                let (p, group) = &all[rng.below(all.len() as u64) as usize];
                let f = &group[rng.below(group.len() as u64) as usize];
                let grid = Grid::new(p);
                let solutions: BTreeSet<Vec<u8>> = group.iter().cloned().collect();
                // A solution with one digit changed, possibly to itself.
                let mut digits: Vec<u8> = grid.whites.iter().map(|&i| f[i]).collect();
                let x = rng.below(digits.len() as u64) as usize;
                digits[x] = 1 + rng.below(9) as u8;
                let masks: Vec<u16> = digits.iter().map(|&d| 1 << (d - 1)).collect();
                let mut full = vec![0; s.cells()];
                for (k, &i) in grid.whites.iter().enumerate() {
                    full[i] = digits[k];
                }
                assert_eq!(broken(&grid, &masks), !solutions.contains(&full));
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
        let shapes = layouts();
        let all: Vec<_> = shapes.iter().map(|(s, w)| by_puzzle(*s, w)).collect();
        for round in 0..1200 {
            let k = round % shapes.len();
            let (p, group) = &all[k][rng.below(all[k].len() as u64) as usize];
            let truth = &group[rng.below(group.len() as u64) as usize];
            let grid = Grid::new(p);
            let fits: Vec<Vec<u8>> = group
                .iter()
                .map(|g| grid.whites.iter().map(|&i| g[i]).collect())
                .collect();
            let truth: Vec<u8> = grid.whites.iter().map(|&i| truth[i]).collect();
            // Partial candidates: the truth plus random other digits.
            let masks: Vec<u16> = truth
                .iter()
                .map(|&d| {
                    let bit = 1 << (d - 1);
                    match rng.below(4) {
                        0 => bit,
                        1 => DIGITS,
                        _ => (rng.next_u64() as u16 & DIGITS) | bit,
                    }
                })
                .collect();
            let fits: Vec<&Vec<u8>> = fits
                .iter()
                .filter(|f| (0..masks.len()).all(|i| masks[i] & (1 << (f[i] - 1)) != 0))
                .collect();
            assert!(!broken(&grid, &masks), "broken although a solution fits");
            let placed: Vec<bool> = masks
                .iter()
                .map(|&m| fixed(m) && rng.below(2) == 0)
                .collect();
            let candidates = Candidates::new(&grid, &masks, &placed, &budget);
            for technique in &TECHNIQUES {
                for step in technique.run(&candidates).unwrap() {
                    fired.insert(step.technique);
                    for &(i, keep) in &step.changes {
                        for f in &fits {
                            assert_ne!(
                                keep & (1 << (f[i] - 1)),
                                0,
                                "{} removed a true digit from {} with {masks:?}",
                                step.technique,
                                p.id()
                            );
                        }
                    }
                }
            }
        }
        for technique in [
            "digit_used",
            "single",
            "sum_combinations",
            "only_cell",
            "combination_fit",
            "short_contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }
}
