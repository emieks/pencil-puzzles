//! Aquarium's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order. Candidate bit 0 is air (`-`) and
//! bit 1 water (`#`). Lines are the rows, top to bottom, then the columns,
//! left to right. A line's need is its clue minus its water cells.
//!
//! The techniques read the candidates as one bitset per line, so that a
//! pass over the grid is a few operations per line.
use crate::model::Layout;
use crate::{MAX_SIZE, Puzzle};
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};

/// The air candidate.
pub(crate) const AIR: u16 = 1;
/// The water candidate.
pub(crate) const WATER: u16 = 2;
/// Both candidates.
const BOTH: u16 = AIR | WATER;
/// Longest Hard contradiction chain.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain.
const LONG_CHAIN: usize = 20;

/// The board's tanks, lines and clues. A line's cells are numbered along
/// it: left to right in a row, top to bottom in a column.
pub(crate) struct Grid {
    rows: usize,
    cols: usize,
    clues: Vec<u32>,
    /// Each line's tanks, as the set of cells each has in the line.
    parts: Vec<Vec<u32>>,
    /// Each tank's rows, top to bottom, with the set of cells it has there.
    tanks: Vec<Vec<(usize, u32)>>,
    /// For each cell, the set of cells of its tank in its row.
    group: Vec<u32>,
}

/// The water cells and the open cells of every line, as sets.
struct Bits {
    water: [u32; 2 * MAX_SIZE],
    open: [u32; 2 * MAX_SIZE],
}

impl Grid {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        let layout = Layout::new(puzzle);
        let (rows, cols) = (layout.rows, layout.cols);
        let tank = puzzle.tanks();
        let set = |cells: &[usize]| cells.iter().fold(0u32, |set, &i| set | 1 << (i % cols));
        let tanks: Vec<Vec<(usize, u32)>> = (layout.tank_groups.iter())
            .map(|groups| {
                (groups.iter())
                    .map(|&g| (layout.groups[g][0] / cols, set(&layout.groups[g])))
                    .collect()
            })
            .collect();
        let mut parts: Vec<Vec<u32>> = vec![Vec::new(); rows + cols];
        for cells in &layout.groups {
            parts[cells[0] / cols].push(set(cells));
        }
        for c in 0..cols {
            let mut part_of = vec![usize::MAX; puzzle.tank_count()];
            for r in 0..rows {
                let t = tank[r * cols + c] as usize;
                if part_of[t] == usize::MAX {
                    part_of[t] = parts[rows + c].len();
                    parts[rows + c].push(0);
                }
                parts[rows + c][part_of[t]] |= 1 << r;
            }
        }
        let mut clues = puzzle.row_clues().to_vec();
        clues.extend_from_slice(puzzle.col_clues());
        Self {
            rows,
            cols,
            clues,
            parts,
            tanks,
            group: (0..tank.len())
                .map(|i| set(&layout.groups[layout.group_of[i]]))
                .collect(),
        }
    }

    fn bits(&self, masks: &[u16]) -> Bits {
        let mut bits = Bits {
            water: [0; 2 * MAX_SIZE],
            open: [0; 2 * MAX_SIZE],
        };
        for (r, row) in masks.chunks_exact(self.cols).enumerate() {
            let (mut water, mut open) = (0, 0);
            for (c, &m) in row.iter().enumerate() {
                let (w, o) = (u32::from(m == WATER), u32::from(m == BOTH));
                water |= w << c;
                open |= o << c;
                bits.water[self.rows + c] |= w << r;
                bits.open[self.rows + c] |= o << r;
            }
            bits.water[r] = water;
            bits.open[r] = open;
        }
        bits
    }

    /// The cell at place `k` of a line.
    fn cell(&self, line: usize, k: u32) -> usize {
        if line < self.rows {
            line * self.cols + k as usize
        } else {
            k as usize * self.cols + line - self.rows
        }
    }

    /// The cells of a line in `set`, in line order.
    fn cells(&self, line: usize, set: u32) -> impl Iterator<Item = usize> + '_ {
        ones(set).map(move |k| self.cell(line, k))
    }

    /// The index of a tank's highest row with water and of its lowest row
    /// with air, among its rows.
    fn levels(&self, bits: &Bits, tank: usize) -> (Option<usize>, Option<usize>) {
        let rows = &self.tanks[tank];
        let water = rows.iter().position(|&(r, set)| bits.water[r] & set != 0);
        let air = rows
            .iter()
            .rposition(|&(r, set)| set & !bits.water[r] & !bits.open[r] != 0);
        (water, air)
    }
}

/// The members of a set, smallest first.
fn ones(mut set: u32) -> impl Iterator<Item = u32> {
    std::iter::from_fn(move || {
        (set != 0).then(|| {
            let k = set.trailing_zeros();
            set &= set - 1;
            k
        })
    })
}

/// The `count` smallest members of a set.
fn lowest(set: u32, count: u32) -> u32 {
    ones(set)
        .take(count as usize)
        .fold(0, |low, k| low | 1 << k)
}

/// Whether the candidates already break a rule: a cell with no candidate, a
/// line with more water than its clue or too few water and open cells to
/// reach it, or a tank with air level with or below water. Never true while
/// some completion keeps every rule; exact on complete grids.
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    if masks.contains(&0) {
        return true;
    }
    let bits = grid.bits(masks);
    (0..grid.clues.len()).any(|line| {
        let (water, open) = (bits.water[line].count_ones(), bits.open[line].count_ones());
        water > grid.clues[line] || water + open < grid.clues[line]
    }) || (0..grid.tanks.len())
        .any(|t| matches!(grid.levels(&bits, t), (Some(water), Some(air)) if air >= water))
}

/// Bookkeeping, in one step: water spreads sideways and down in its tank
/// and air sideways and up; a line with all its water gets air in its open
/// cells, and a line that needs all its open cells gets water.
fn routine(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    c.charge()?;
    let bits = g.bits(c.masks);
    let mut keep = vec![BOTH; c.masks.len()];
    let mut support = Vec::new();
    for (t, rows) in g.tanks.iter().enumerate() {
        let (water, air) = g.levels(&bits, t);
        let mut changed = false;
        for (k, &(r, set)) in rows.iter().enumerate() {
            let open = set & bits.open[r];
            if open == 0 {
                continue;
            }
            for (level, value) in [
                (water.filter(|&w| k >= w), WATER),
                (air.filter(|&a| k <= a), AIR),
            ] {
                if level.is_some() {
                    changed = true;
                    for i in g.cells(r, open) {
                        keep[i] &= value;
                    }
                }
            }
        }
        if changed {
            // One cell of the tank's highest water and of its lowest air.
            let first = |k: usize, set: u32| g.cell(rows[k].0, (set & rows[k].1).trailing_zeros());
            support.extend(water.map(|k| first(k, bits.water[rows[k].0])));
            support.extend(air.map(|k| first(k, !bits.water[rows[k].0] & !bits.open[rows[k].0])));
        }
    }
    for line in 0..g.clues.len() {
        let (water, open) = (bits.water[line], bits.open[line]);
        let Some(need) = g.clues[line].checked_sub(water.count_ones()) else {
            continue;
        };
        if open != 0 && (need == 0 || need == open.count_ones()) {
            let value = if need == 0 { AIR } else { WATER };
            for i in g.cells(line, open) {
                keep[i] &= value;
            }
            support.extend(g.cells(line, water));
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

/// One step per line where `deduce` finds cells to change. `deduce` gets
/// the line, its need and its open cells, and returns the open cells that
/// are air and those that are water. It is only asked about lines with open
/// cells that can reach the need.
fn per_line(
    c: &Candidates<'_, Grid>,
    technique: &'static str,
    lines: std::ops::Range<usize>,
    mut deduce: impl FnMut(usize, u32, u32) -> (u32, u32),
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let bits = g.bits(c.masks);
    let mut steps = Vec::new();
    for line in lines {
        c.charge()?;
        let open = bits.open[line];
        let Some(need) = g.clues[line].checked_sub(bits.water[line].count_ones()) else {
            continue;
        };
        if open == 0 || open.count_ones() < need {
            continue;
        }
        let (air, water) = deduce(line, need, open);
        if air | water == 0 {
            continue;
        }
        let changes = (g.cells(line, air).map(|i| (i, AIR)))
            .chain(g.cells(line, water).map(|i| (i, WATER)))
            .collect();
        // The line's decided cells; the step adds the changed ones.
        let length = if line < g.rows { g.cols } else { g.rows };
        let decided = g.cells(line, !open & ((1 << length) - 1)).collect();
        steps.extend(c.step(technique, decided, vec![line], changes));
        if c.first_only && !steps.is_empty() {
            break;
        }
    }
    Ok(steps)
}

/// A tank with more open cells in a line than the line needs cannot fill
/// them all. In a row they share one level, so all are air. In a column
/// they fill from the bottom, so the top ones beyond the need are air.
fn too_big(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    per_line(c, "too_big", 0..g.clues.len(), |line, need, open| {
        let mut air = 0;
        for &part in &g.parts[line] {
            let cells = part & open;
            if cells.count_ones() > need {
                air |= if line < g.rows {
                    cells
                } else {
                    lowest(cells, cells.count_ones() - need)
                };
            }
        }
        (air, 0)
    })
}

/// A tank whose open cells a line cannot do without holds water there. In
/// a row all of them are water. In a column the bottom ones are, as many
/// as the other tanks fall short.
fn needed(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    per_line(c, "needed", 0..g.clues.len(), |line, need, open| {
        let mut water = 0;
        for &part in &g.parts[line] {
            let cells = part & open;
            let others = (open & !part).count_ones();
            if cells != 0 && others < need {
                water |= if line < g.rows {
                    cells
                } else {
                    cells & !lowest(cells, cells.count_ones() - (need - others))
                };
            }
        }
        (0, water)
    })
}

/// The ways a row's tanks can make up its need: each tank's open cells in
/// the row are all water or all air. Tanks that are water in every way get
/// water, and tanks that are water in no way get air.
fn combinations(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    per_line(c, "combinations", 0..g.rows, |line, need, open| {
        let parts = &g.parts[line];
        let size = |k: usize| (parts[k] & open).count_ones();
        // Bit `s` of `before[k]` is set when the first k tanks can hold
        // exactly `s` cells of water; `after[k]` is the same for the tanks
        // from the k-th on.
        let fits = (1u32 << (need + 1)) - 1;
        let n = parts.len();
        let mut before = [1u32; MAX_SIZE + 1];
        for k in 0..n {
            before[k + 1] = (before[k] | before[k] << size(k)) & fits;
        }
        let mut after = [1u32; MAX_SIZE + 1];
        for k in (0..n).rev() {
            after[k] = (after[k + 1] | after[k + 1] << size(k)) & fits;
        }
        let (mut air, mut water) = (0, 0);
        for k in (0..n).filter(|&k| size(k) > 0) {
            // Whether the other tanks can hold exactly `rest`.
            let makes = |rest: u32| {
                (0..=rest).any(|s| before[k] >> s & 1 == 1 && after[k + 1] >> (rest - s) & 1 == 1)
            };
            match (size(k) <= need && makes(need - size(k)), makes(need)) {
                (true, false) => water |= parts[k] & open,
                (false, true) => air |= parts[k] & open,
                _ => {}
            }
        }
        (air, water)
    })
}

/// Contradictions are only tried on the first open cell of each tank's
/// row: the cells beside it share its value.
fn contradictions(
    c: &Candidates<'_, Grid>,
    technique: &'static str,
    max_steps: usize,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let bits = g.bits(c.masks);
    PROFILE.contradictions_where(c, technique, Medium, max_steps, |i| {
        let open = g.group[i] & bits.open[i / g.cols];
        open.trailing_zeros() as usize == i % g.cols
    })
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
    entry("too_big", Easy, Skill, false, too_big),
    entry("needed", Easy, Skill, false, needed),
    entry("combinations", Medium, Skill, false, combinations),
    entry("short_contradiction", Hard, Skill, true, |c| {
        contradictions(c, "short_contradiction", SHORT_CHAIN)
    }),
    entry("contradiction", Expert, Skill, true, |c| {
        contradictions(c, "contradiction", LONG_CHAIN)
    }),
];

/// The canonical Aquarium human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "aquarium-human",
    techniques: &TECHNIQUES,
    broken,
};

/// Every cell starts undecided.
pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    vec![BOTH; puzzle.tanks().len()]
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
    use crate::Shape;
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    /// Whether a full shading (bit set = water) keeps every rule, by plain
    /// loops.
    fn valid(p: &Puzzle, water: u32) -> bool {
        let s = p.shape();
        let cols = s.cols();
        let n = s.cells();
        let wet = |i: usize| water >> i & 1 == 1;
        let rows_fit = (0..s.rows())
            .all(|r| (0..cols).filter(|&c| wet(r * cols + c)).count() as u32 == p.row_clues()[r]);
        let cols_fit = (0..cols).all(|c| {
            (0..s.rows()).filter(|&r| wet(r * cols + c)).count() as u32 == p.col_clues()[c]
        });
        let level = (0..n).all(|i| {
            (0..n).all(|j| {
                !(wet(i) && !wet(j) && p.tanks()[i] == p.tanks()[j] && j / cols >= i / cols)
            })
        });
        rows_fit && cols_fit && level
    }

    fn masks_of(n: usize, water: u32) -> Vec<u16> {
        (0..n)
            .map(|i| if water >> i & 1 == 1 { WATER } else { AIR })
            .collect()
    }

    /// Random tanks, each cell joining the tank above or to the left or
    /// starting a new one, with random clues.
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
        let clues = |rng: &mut SplitMix64, count: usize, max: usize| {
            (0..count)
                .map(|_| rng.below(max as u64 + 1) as u32)
                .collect()
        };
        let rows = clues(rng, shape.rows(), cols);
        let columns = clues(rng, cols, shape.rows());
        Puzzle::new(shape, &labels, rows, columns).unwrap()
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
                for water in 0..1u32 << s.cells() {
                    assert_eq!(
                        broken(&grid, &masks_of(s.cells(), water)),
                        !valid(&p, water),
                        "{} {water:b}",
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
            // Clues from a random shading of random tanks that keeps R3, so
            // that there is a solution.
            let tanks = random_puzzle(&mut rng, s);
            let Some(truth) = (0..40).map(|_| rng.below(1 << n) as u32).find(|&w| {
                let cols = s.cols();
                (0..n).all(|i| {
                    (0..n).all(|j| {
                        !(w >> i & 1 == 1
                            && w >> j & 1 == 0
                            && tanks.tanks()[i] == tanks.tanks()[j]
                            && j / cols >= i / cols)
                    })
                })
            }) else {
                continue;
            };
            round += 1;
            let mut solution = crate::Answer::new(s);
            for i in 0..n {
                let mark = if truth >> i & 1 == 1 {
                    crate::Mark::Shaded
                } else {
                    crate::Mark::Unshaded
                };
                solution.set(i / s.cols(), i % s.cols(), mark);
            }
            let p = tanks.with_clues_of(&solution);
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
            "too_big",
            "needed",
            "combinations",
            "short_contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }

    /// The examples on the genre page: what a technique finds in one line
    /// of an untouched grid.
    #[test]
    fn documented_examples() {
        let budget = Budget::unlimited();
        let run = |text: &str, find: Find<Grid>, line: usize| {
            let p: Puzzle = text.parse().unwrap();
            let grid = Grid::new(&p);
            let masks = initial_masks(&p);
            let placed = vec![false; masks.len()];
            let steps = find(&Candidates::new(&grid, &masks, &placed, &budget)).unwrap();
            let step = steps.into_iter().find(|s| s.units == [line]);
            step.map_or(Vec::new(), |s| s.changes)
        };
        // Row 1 needs 2 and tank a has three cells there.
        let text = "rows: 2
cols: 0 0 0 1 1
regions:
aaabc";
        assert_eq!(run(text, too_big, 0), [(0, AIR), (1, AIR), (2, AIR)]);
        // A column needing 1 with a tank three cells tall: the top two are air.
        let text = "rows: 0 0 1
cols: 1
regions:
a
a
a";
        assert_eq!(run(text, too_big, 3), [(0, AIR), (1, AIR)]);
        // Row 1 needs 4; without tank a the others hold 2.
        let text = "rows: 4
cols: 1 1 1 1 0
regions:
aaabc";
        assert_eq!(run(text, needed, 0), [(0, WATER), (1, WATER), (2, WATER)]);
        // A column needing 3 with two tanks of two cells: each needs its
        // bottom cell.
        let text = "rows: 0 1 1 1
cols: 3
regions:
a
a
b
b";
        assert_eq!(run(text, needed, 4), [(1, WATER), (3, WATER)]);
        // Need 4 from tanks of 3, 3 and 1: the single cell is in both ways.
        let text = "rows: 4
cols: 1 1 1 0 0 0 1
regions:
aaabbbc";
        assert_eq!(run(text, combinations, 0), [(6, WATER)]);
        // Need 4 from tanks of 2, 2 and 1: the single cell is in no way.
        let text = "rows: 4
cols: 1 1 1 1 0
regions:
aabbc";
        assert_eq!(run(text, combinations, 0)[4], (4, AIR));
    }
}
