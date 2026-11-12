//! Binairo's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order. Candidate bit 0 is the digit 0
//! and bit 1 the digit 1. Lines are the rows, top to bottom, then the
//! columns, left to right. A line's cells are numbered along it: left to
//! right in a row, top to bottom in a column. A line is finished when every
//! cell in it has a digit.
//!
//! The techniques read the candidates as two bitsets per line, the cells
//! holding 0 and those holding 1, so that a pass over the grid is a few
//! operations per line.
use crate::{MAX_SIZE, Mark, Puzzle};
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};

/// The candidate 0.
pub(crate) const ZERO: u16 = 1;
/// The candidate 1.
pub(crate) const ONE: u16 = 2;
/// Both candidates.
const BOTH: u16 = ZERO | ONE;
/// Longest Hard contradiction chain.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain.
const LONG_CHAIN: usize = 20;

/// The board's size, and the bits of the candidates seen last.
pub(crate) struct Grid {
    rows: usize,
    cols: usize,
    last: std::cell::RefCell<(Vec<u16>, Bits)>,
}

/// The cells holding each digit in every line, as sets: `digit[0]` for 0
/// and `digit[1]` for 1.
#[derive(Clone, Copy)]
struct Bits {
    digit: [[u32; 2 * MAX_SIZE]; 2],
}

impl Bits {
    fn decided(&self, line: usize) -> u32 {
        self.digit[0][line] | self.digit[1][line]
    }
}

/// The candidate mask of a digit.
fn mask(digit: usize) -> u16 {
    1 << digit
}

/// Whether a set has three members in a row.
fn triple(set: u32) -> bool {
    set & set >> 1 & set >> 2 != 0
}

impl Grid {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        Self {
            rows: puzzle.shape().rows(),
            cols: puzzle.shape().cols(),
            last: std::cell::RefCell::new((
                Vec::new(),
                Bits {
                    digit: [[0; 2 * MAX_SIZE]; 2],
                },
            )),
        }
    }

    fn lines(&self) -> usize {
        self.rows + self.cols
    }

    /// The number of cells in a line.
    fn length(&self, line: usize) -> usize {
        if line < self.rows {
            self.cols
        } else {
            self.rows
        }
    }

    /// Every cell of a line, as a set.
    fn full(&self, line: usize) -> u32 {
        (1 << self.length(line)) - 1
    }

    /// The lines parallel to `line`, itself included.
    fn parallel(&self, line: usize) -> std::ops::Range<usize> {
        if line < self.rows {
            0..self.rows
        } else {
            self.rows..self.lines()
        }
    }

    fn bits(&self, masks: &[u16]) -> Bits {
        let mut last = self.last.borrow_mut();
        if last.0 == masks {
            return last.1;
        }
        let bits = self.build(masks);
        last.0.clear();
        last.0.extend_from_slice(masks);
        last.1 = bits;
        bits
    }

    fn build(&self, masks: &[u16]) -> Bits {
        let mut bits = Bits {
            digit: [[0; 2 * MAX_SIZE]; 2],
        };
        let [zeros, ones] = &mut bits.digit;
        let (row_zeros, col_zeros) = zeros.split_at_mut(self.rows);
        let (row_ones, col_ones) = ones.split_at_mut(self.rows);
        for (r, row) in masks.chunks_exact(self.cols).enumerate() {
            let (mut z, mut o) = (0, 0);
            for (c, &m) in row.iter().enumerate() {
                let (is_zero, is_one) = (u32::from(m == ZERO), u32::from(m == ONE));
                z |= is_zero << c;
                o |= is_one << c;
                col_zeros[c] |= is_zero << r;
                col_ones[c] |= is_one << r;
            }
            row_zeros[r] = z;
            row_ones[r] = o;
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

/// Whether the candidates already break a rule: a cell with no candidate,
/// three equal digits in a row, a line with more than half its cells one
/// digit, or two equal finished parallel lines. Never true while some
/// completion keeps every rule; exact on complete grids.
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    if masks.contains(&0) {
        return true;
    }
    let bits = grid.bits(masks);
    let mut finished = [u32::MAX; 2 * MAX_SIZE];
    for line in 0..grid.lines() {
        let half = grid.length(line) as u32 / 2;
        for d in &bits.digit {
            if triple(d[line]) || d[line].count_ones() > half {
                return true;
            }
        }
        if bits.decided(line) == grid.full(line) {
            let ones = bits.digit[1][line];
            if finished[grid.parallel(line)].contains(&ones) {
                return true;
            }
            finished[line] = ones;
        }
    }
    false
}

/// Bookkeeping, in one step: a line with half its cells one digit gets the
/// other digit in its open cells.
fn routine(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    c.charge()?;
    let bits = g.bits(c.masks);
    let mut changes = Vec::new();
    let mut support = Vec::new();
    for line in 0..g.lines() {
        let half = g.length(line) as u32 / 2;
        let open = g.full(line) & !bits.decided(line);
        if open == 0 {
            continue;
        }
        for (d, digit) in bits.digit.iter().enumerate() {
            if digit[line].count_ones() == half {
                changes.extend(g.cells(line, open).map(|i| (i, mask(1 - d))));
                support.extend(g.cells(line, digit[line]));
            }
        }
    }
    Ok(c.step("routine", support, Vec::new(), changes)
        .into_iter()
        .collect())
}

/// What a technique finds in a line: the open cells that hold each digit,
/// the line's cells it used, and other lines all of whose cells it used.
type Found = ([u32; 2], u32, u32);

/// One step per line where `deduce` finds cells to decide.
fn per_line(
    c: &Candidates<'_, Grid>,
    technique: &'static str,
    mut deduce: impl FnMut(&Bits, usize) -> Found,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    let bits = g.bits(c.masks);
    let mut steps = Vec::new();
    for line in 0..g.lines() {
        c.charge()?;
        let open = g.full(line) & !bits.decided(line);
        if open == 0 {
            continue;
        }
        let (found, used, lines) = deduce(&bits, line);
        if (found[0] | found[1]) & open == 0 {
            continue;
        }
        let mut changes = Vec::new();
        for (d, &cells) in found.iter().enumerate() {
            changes.extend(g.cells(line, cells & open).map(|i| (i, mask(d))));
        }
        let mut support: Vec<usize> = g.cells(line, used).collect();
        for other in ones(lines) {
            let other = other as usize;
            support.extend(g.cells(other, g.full(other)));
        }
        steps.extend(c.step(technique, support, vec![line], changes));
        if c.first_only && !steps.is_empty() {
            break;
        }
    }
    Ok(steps)
}

/// Two equal neighbours: the cells on both ends hold the other digit.
fn pair(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    per_line(c, "pair", |bits, line| {
        let (mut found, mut used) = ([0; 2], 0);
        for d in 0..2 {
            let set = bits.digit[d][line];
            let pairs = set & set >> 1;
            found[1 - d] |= pairs >> 1 | pairs << 2;
            used |= pairs | pairs << 1;
        }
        (found, used, 0)
    })
}

/// Two equal digits with one cell between them: that cell holds the other
/// digit.
fn gap(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    per_line(c, "gap", |bits, line| {
        let (mut found, mut used) = ([0; 2], 0);
        for d in 0..2 {
            let set = bits.digit[d][line];
            let gaps = set & set >> 2;
            found[1 - d] |= gaps << 1;
            used |= gaps | gaps << 2;
        }
        (found, used, 0)
    })
}

/// What a line needing one more of some digit says: that digit goes in one
/// open cell and the other digit fills the rest. The digit can only go
/// where no three equal digits end up in a row and, with `distinct`, where
/// the line does not copy a finished parallel line. Open cells where it
/// cannot go hold the other digit; if it can go in one cell only, it is
/// there. If it can go nowhere, every open cell loses both digits.
fn lone(g: &Grid, bits: &Bits, line: usize, distinct: bool) -> Found {
    let half = g.length(line) as u32 / 2;
    let open = g.full(line) & !bits.decided(line);
    let used = bits.decided(line);
    let mut lines = 0;
    for d in 0..2 {
        let (mine, other) = (bits.digit[d][line], bits.digit[1 - d][line]);
        if mine.count_ones() + 1 != half || open.count_ones() < 2 {
            continue;
        }
        let mut places = 0;
        for k in ones(open) {
            let (with, rest) = (mine | 1 << k, other | (open & !(1 << k)));
            if triple(with) || triple(rest) {
                continue;
            }
            if distinct {
                let ones = if d == 1 { with } else { rest };
                let copied = g.parallel(line).find(|&l| {
                    l != line && bits.decided(l) == g.full(l) && bits.digit[1][l] == ones
                });
                if let Some(l) = copied {
                    lines |= 1 << l;
                    continue;
                }
            }
            places |= 1 << k;
        }
        if places == 0 {
            return ([open, open], used, lines);
        }
        let mut found = [0; 2];
        found[1 - d] = open & !places;
        if places.count_ones() == 1 {
            found[d] = places;
        }
        return (found, used, lines);
    }
    ([0; 2], 0, 0)
}

/// A line needing one more of a digit, by R1 and R2.
fn one_left(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    per_line(c, "one_left", |bits, line| lone(g, bits, line, false))
}

/// The same, also ruling out copies of finished parallel lines (R3).
fn duplicate(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let g = c.context;
    per_line(c, "duplicate", |bits, line| lone(g, bits, line, true))
}

fn short_contradiction(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    PROFILE.contradictions(c, "short_contradiction", Medium, SHORT_CHAIN)
}

fn contradiction(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    PROFILE.contradictions(c, "contradiction", Medium, LONG_CHAIN)
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

static TECHNIQUES: [Technique<Grid>; 7] = [
    entry("routine", Easy, Bookkeeping, false, routine),
    entry("pair", Easy, Skill, false, pair),
    entry("gap", Easy, Skill, false, gap),
    entry("one_left", Medium, Skill, false, one_left),
    entry("duplicate", Medium, Skill, false, duplicate),
    entry(
        "short_contradiction",
        Hard,
        Skill,
        true,
        short_contradiction,
    ),
    entry("contradiction", Expert, Skill, true, contradiction),
];

/// The canonical Binairo human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "binairo-human",
    techniques: &TECHNIQUES,
    broken,
};

/// Given cells start with their digit, the rest undecided.
pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    puzzle
        .givens()
        .iter()
        .map(|m| match m {
            Mark::Unmarked => BOTH,
            Mark::Zero => ZERO,
            Mark::One => ONE,
        })
        .collect()
}

/// Solve from `masks` with techniques up to `cap`. Cells with one
/// candidate count as written in.
pub(crate) fn analyze_state(
    grid: &Grid,
    masks: Vec<u16>,
    budget: &Budget,
    cap: Difficulty,
    metrics: bool,
) -> Result<Trace, BudgetExceeded> {
    let placed = masks.iter().map(|m| m.count_ones() == 1).collect();
    PROFILE.analyze(grid, masks, placed, budget, cap, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Answer, Shape};
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    /// Whether a full grid (bit `i` set = cell `i` is 1) keeps R1-R3, by
    /// plain loops.
    fn valid(shape: Shape, grid: u32) -> bool {
        let (rows, cols) = (shape.rows(), shape.cols());
        let at = |r: usize, c: usize| grid >> (r * cols + c) & 1;
        let lines: Vec<Vec<u32>> = (0..rows)
            .map(|r| (0..cols).map(|c| at(r, c)).collect())
            .chain((0..cols).map(|c| (0..rows).map(|r| at(r, c)).collect()))
            .collect();
        let fine = lines.iter().all(|l| {
            2 * l.iter().sum::<u32>() as usize == l.len()
                && l.windows(3).all(|w| !(w[0] == w[1] && w[1] == w[2]))
        });
        let distinct = (0..lines.len()).all(|a| {
            (a + 1..lines.len()).all(|b| (a < rows) != (b < rows) || lines[a] != lines[b])
        });
        fine && distinct
    }

    fn masks_of(n: usize, grid: u32) -> Vec<u16> {
        (0..n)
            .map(|i| if grid >> i & 1 == 1 { ONE } else { ZERO })
            .collect()
    }

    fn grid(shape: Shape) -> Grid {
        let p = Puzzle::new(shape, vec![Mark::Unmarked; shape.cells()]).unwrap();
        Grid::new(&p)
    }

    /// The answer with the decided cells of `masks`.
    fn answer(shape: Shape, masks: &[u16]) -> Answer {
        let mut a = Answer::new(shape);
        for (i, &m) in masks.iter().enumerate() {
            if m == ZERO || m == ONE {
                a.set(
                    i / shape.cols(),
                    i % shape.cols(),
                    Mark::of(u8::from(m == ONE)),
                );
            }
        }
        a
    }

    /// `broken` is exact on every full grid of the small shapes.
    #[test]
    fn broken_is_exact_on_full_grids() {
        for shape in ["2x2", "4x4"] {
            let s: Shape = shape.parse().unwrap();
            let g = grid(s);
            for bits in 0..1u32 << s.cells() {
                assert_eq!(
                    broken(&g, &masks_of(s.cells(), bits)),
                    !valid(s, bits),
                    "{shape} {bits:b}"
                );
            }
        }
    }

    /// Every technique keeps every solution of random partial states,
    /// `broken` never fires while a solution remains, and the checker
    /// reports a broken rule whenever `broken` fires (L23, P8).
    #[test]
    fn techniques_keep_every_solution() {
        let mut rng = SplitMix64::new(17);
        let budget = Budget::unlimited();
        let mut fired = BTreeSet::new();
        for shape in ["4x4", "4x6", "6x4"] {
            let s: Shape = shape.parse().unwrap();
            let n = s.cells();
            let g = grid(s);
            // Grids from rows that each have as many 1s as 0s.
            let rows: Vec<u32> = (0..1u32 << s.cols())
                .filter(|r| 2 * r.count_ones() as usize == s.cols())
                .collect();
            let mut grids = vec![0u32];
            for r in 0..s.rows() {
                grids = grids
                    .iter()
                    .flat_map(|&g| rows.iter().map(move |&row| g | row << (r * s.cols())))
                    .collect();
            }
            let solutions: Vec<u32> = grids.into_iter().filter(|&b| valid(s, b)).collect();
            let blank = Puzzle::new(s, vec![Mark::Unmarked; n]).unwrap();
            for _ in 0..1500 {
                // Some cells of a solution, sometimes with a wrong digit.
                let truth = solutions[rng.below(solutions.len() as u64) as usize];
                let keep = rng.below(3) + 1;
                let mut masks: Vec<u16> = masks_of(n, truth)
                    .into_iter()
                    .map(|m| if rng.below(4) < keep { m } else { BOTH })
                    .collect();
                if rng.below(4) == 0 {
                    let i = rng.below(n as u64) as usize;
                    masks[i] = [ZERO, ONE][rng.below(2) as usize];
                }
                let fits: Vec<u32> = solutions
                    .iter()
                    .copied()
                    .filter(|&b| masks_of(n, b).iter().zip(&masks).all(|(t, m)| t & m != 0))
                    .collect();
                let is_broken = broken(&g, &masks);
                if !fits.is_empty() {
                    assert!(!is_broken, "broken although a solution fits: {masks:?}");
                }
                if is_broken {
                    let a = answer(s, &masks);
                    assert!(!crate::check(&blank, &a).unwrap().is_empty(), "{a}");
                }
                let placed = vec![false; n];
                let candidates = Candidates::new(&g, &masks, &placed, &budget);
                for technique in &TECHNIQUES {
                    for step in technique.run(&candidates).unwrap() {
                        fired.insert(step.technique);
                        for &(i, keep) in &step.changes {
                            for &f in &fits {
                                assert_ne!(
                                    keep & masks_of(n, f)[i],
                                    0,
                                    "{} removed a true value with {masks:?}",
                                    step.technique
                                );
                            }
                        }
                    }
                }
            }
        }
        for technique in [
            "routine",
            "pair",
            "gap",
            "one_left",
            "duplicate",
            "short_contradiction",
            "contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }

    /// The examples on the genre page: what a technique finds in the first
    /// row of a puzzle.
    #[test]
    fn documented_examples() {
        let budget = Budget::unlimited();
        let run = |row: &str, find: Find<Grid>| -> String {
            let blank = ".".repeat(row.len());
            let mut rows = vec![row.to_string()];
            rows.extend((1..row.len()).map(|_| blank.clone()));
            let p = Puzzle::from_rows(&rows).unwrap();
            let g = Grid::new(&p);
            let masks = initial_masks(&p);
            let placed = vec![false; masks.len()];
            let mut after = masks.clone();
            for step in find(&Candidates::new(&g, &masks, &placed, &budget)).unwrap() {
                for (i, keep) in step.changes {
                    after[i] &= keep;
                }
            }
            (0..row.len())
                .map(|i| match after[i] {
                    ZERO => '0',
                    ONE => '1',
                    _ => '.',
                })
                .collect()
        };
        assert_eq!(run(".11...", pair), "0110..");
        assert_eq!(run("0.0...", gap), "010...");
        assert_eq!(run("110...", routine), "110...");
        assert_eq!(run("1101..", routine), "110100");
        // One more 1: in the second cell it would leave three 0s together.
        assert_eq!(run("1...01", one_left), "10..01");
        // Rows 1 and 2 would be the same.
        let p = Puzzle::from_rows(&["0101", "01..", "....", "...."]).unwrap();
        let g = Grid::new(&p);
        let masks = initial_masks(&p);
        let placed = vec![false; 16];
        let steps = duplicate(&Candidates::new(&g, &masks, &placed, &budget)).unwrap();
        assert_eq!(steps[0].changes, [(6, ONE), (7, ZERO)]);
    }
}
