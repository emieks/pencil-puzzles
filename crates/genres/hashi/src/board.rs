//! Where bridges can go, the exact solver and the checker. The solver is
//! independent of the human techniques: it narrows each slot's possible
//! counts from the island sums, crossings and connectivity, then searches.
use crate::{Answer, BudgetExceeded, Puzzle};
use pencil_puzzles_core::{Budget, ParseError, Place, Solutions, Violation};

/// A place where bridges can go: two islands in the same row or column with
/// only empty cells between them.
#[derive(Clone, Debug)]
pub(crate) struct Slot {
    /// The two islands, by index; the first comes first in row-major order.
    pub ends: [usize; 2],
    /// Whether the islands share a row.
    pub horizontal: bool,
    /// The cells between the islands, in order. Never empty, because
    /// islands are never adjacent.
    pub interior: Vec<usize>,
    /// The most bridges the slot can hold: 2, or less next to a smaller
    /// number.
    pub cap: u8,
}

/// The islands of a puzzle and the slots between them.
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub cols: usize,
    /// Each island's cell, in row-major order.
    pub cells: Vec<usize>,
    /// Each island's number.
    pub numbers: Vec<u8>,
    /// The island in each cell, if any.
    pub island_at: Vec<Option<usize>>,
    /// Slots ordered by their first island, the one to the right before the
    /// one below.
    pub slots: Vec<Slot>,
    /// The slots at each island, in slot order.
    pub at: Vec<Vec<usize>>,
    /// The slots each slot crosses, in slot order.
    pub crosses: Vec<Vec<usize>>,
    /// The horizontal and vertical slot passing through each empty cell.
    pub through: Vec<[Option<usize>; 2]>,
}

impl Layout {
    pub(crate) fn new(p: &Puzzle) -> Self {
        let clues = p.clues();
        let cols = p.shape().cols();
        let n = clues.len();
        let mut island_at = vec![None; n];
        let mut cells = Vec::new();
        let mut numbers = Vec::new();
        for (i, &c) in clues.iter().enumerate() {
            if c > 0 {
                island_at[i] = Some(cells.len());
                cells.push(i);
                numbers.push(c);
            }
        }
        let mut slots = Vec::new();
        let mut at = vec![Vec::new(); cells.len()];
        let mut through = vec![[None, None]; n];
        for (k, &i) in cells.iter().enumerate() {
            for (horizontal, step) in [(true, 1), (false, cols)] {
                let mut interior = Vec::new();
                let mut j = i;
                let end = loop {
                    if (horizontal && j % cols + 1 == cols) || (!horizontal && j + cols >= n) {
                        break None;
                    }
                    j += step;
                    match island_at[j] {
                        Some(other) => break Some(other),
                        None => interior.push(j),
                    }
                };
                if let Some(other) = end {
                    let s = slots.len();
                    for &c in &interior {
                        through[c][usize::from(!horizontal)] = Some(s);
                    }
                    at[k].push(s);
                    at[other].push(s);
                    slots.push(Slot {
                        ends: [k, other],
                        horizontal,
                        interior,
                        cap: numbers[k].min(numbers[other]).min(2),
                    });
                }
            }
        }
        for list in &mut at {
            list.sort_unstable();
        }
        let mut crosses = vec![Vec::new(); slots.len()];
        for (s, slot) in slots.iter().enumerate() {
            for &c in &slot.interior {
                if let [Some(h), Some(v)] = through[c] {
                    crosses[s].push(if h == s { v } else { h });
                }
            }
        }
        for list in &mut crosses {
            list.sort_unstable();
        }
        Self {
            cols,
            cells,
            numbers,
            island_at,
            slots,
            at,
            crosses,
            through,
        }
    }

    /// The place of slot `s`, for steps and violations.
    pub(crate) fn place(&self, s: usize) -> Place {
        let [a, b] = self.slots[s].ends.map(|k| self.cells[k]);
        Place::Link {
            row: (a / self.cols) as u32,
            col: (a % self.cols) as u32,
            row2: (b / self.cols) as u32,
            col2: (b % self.cols) as u32,
        }
    }

    /// Connected groups of islands joined by the slots `joined` accepts.
    pub(crate) fn groups(&self, joined: impl Fn(usize) -> bool) -> usize {
        let mut parent: Vec<usize> = (0..self.cells.len()).collect();
        fn root(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        let mut groups = self.cells.len();
        for (s, slot) in self.slots.iter().enumerate() {
            if joined(s) {
                let (a, b) = (
                    root(&mut parent, slot.ends[0]),
                    root(&mut parent, slot.ends[1]),
                );
                if a != b {
                    parent[a] = b;
                    groups -= 1;
                }
            }
        }
        groups
    }
}

pub(crate) fn cell(i: usize, cols: usize) -> Place {
    Place::Cell {
        row: (i / cols) as u32,
        col: (i % cols) as u32,
    }
}

/// Possible counts of a slot: bit `v` stands for `v` bridges.
type Counts = u8;

fn lowest(d: Counts) -> u8 {
    d.trailing_zeros() as u8
}

fn highest(d: Counts) -> u8 {
    7 - d.leading_zeros() as u8
}

/// Counts from `lo` to `hi`, empty if `lo > hi`.
fn range(lo: i32, hi: i32) -> Counts {
    (lo.max(0)..=hi.min(2)).fold(0, |m, v| m | 1 << v)
}

struct Search<'a> {
    layout: &'a Layout,
    budget: &'a Budget,
    found: Vec<Vec<u8>>,
}

impl Search<'_> {
    fn spend(&self, n: usize) -> Result<(), BudgetExceeded> {
        if self.budget.spend(n as u64 + 1) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    /// Narrow the counts until nothing changes. False if some island's
    /// number can't be met, crossing slots both need bridges, or the slots
    /// left can't join every island.
    fn propagate(&self, d: &mut [Counts]) -> Result<bool, BudgetExceeded> {
        let l = self.layout;
        loop {
            self.spend(d.len())?;
            let before = d.to_vec();
            for (k, slots) in l.at.iter().enumerate() {
                let n = i32::from(l.numbers[k]);
                let lo: i32 = slots.iter().map(|&s| i32::from(lowest(d[s]))).sum();
                let hi: i32 = slots.iter().map(|&s| i32::from(highest(d[s]))).sum();
                if n < lo || n > hi {
                    return Ok(false);
                }
                for &s in slots {
                    let (a, b) = (i32::from(lowest(d[s])), i32::from(highest(d[s])));
                    d[s] &= range(n - (hi - b), n - (lo - a));
                    if d[s] == 0 {
                        return Ok(false);
                    }
                }
            }
            for s in 0..d.len() {
                if d[s] & 1 == 0 {
                    for &c in &l.crosses[s] {
                        d[c] &= 1;
                        if d[c] == 0 {
                            return Ok(false);
                        }
                    }
                }
            }
            if l.groups(|s| d[s] != 1) > 1 {
                return Ok(false);
            }
            if d == before {
                return Ok(true);
            }
        }
    }

    fn run(&mut self, mut d: Vec<Counts>) -> Result<(), BudgetExceeded> {
        if !self.propagate(&mut d)? {
            return Ok(());
        }
        let open = (0..d.len())
            .filter(|&s| d[s].count_ones() > 1)
            .min_by_key(|&s| d[s].count_ones());
        let Some(s) = open else {
            self.found.push(d.iter().map(|&m| lowest(m)).collect());
            return Ok(());
        };
        for v in 0..3 {
            if d[s] & 1 << v != 0 && self.found.len() < 2 {
                let mut next = d.clone();
                next[s] = 1 << v;
                self.run(next)?;
            }
        }
        Ok(())
    }
}

/// Bridges on each slot of up to two solutions.
pub(crate) fn solutions(layout: &Layout, budget: &Budget) -> Result<Vec<Vec<u8>>, BudgetExceeded> {
    let mut search = Search {
        layout,
        budget,
        found: Vec::new(),
    };
    let start = layout
        .slots
        .iter()
        .map(|s| range(0, i32::from(s.cap)))
        .collect();
    search.run(start)?;
    Ok(search.found)
}

/// Find whether the puzzle has no solution, exactly one, or more than one
/// (with two of them), within the work budget (P5).
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let layout = Layout::new(p);
    let mut found = solutions(&layout, budget)?;
    let answer = |counts: &[u8]| Answer::from_counts(p, &layout, counts, true);
    Ok(match found.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(answer(&found.remove(0))),
        _ => Solutions::Multiple(answer(&found[0]), answer(&found[1])),
    })
}

/// The number of solutions, counting at most two.
pub fn count_solutions(p: &Puzzle, budget: &Budget) -> Result<usize, BudgetExceeded> {
    Ok(solutions(&Layout::new(p), budget)?.len())
}

/// Every broken rule instance of an answer (P8). For an unfinished answer,
/// bridges not drawn are undecided, and a rule is reported only if no way of
/// drawing them could keep it.
pub fn check(
    p: &Puzzle,
    a: &Answer,
    budget: &Budget,
) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
    if a.shape() != p.shape() {
        return Err(ParseError::new("the answer has the wrong shape").into());
    }
    let layout = Layout::new(p);
    if !budget.spend((p.shape().cells() + layout.slots.len() * 4) as u64) {
        return Err(Box::new(BudgetExceeded));
    }
    let counts = a.counts(&layout)?;
    let known = |s: usize| counts[s] > 0 || a.is_finished();
    let cols = layout.cols;
    let mut violations = Vec::new();
    for (k, slots) in layout.at.iter().enumerate() {
        let n = layout.numbers[k];
        let drawn: u8 = slots.iter().map(|&s| counts[s]).sum();
        let room: u8 = slots
            .iter()
            .filter(|&&s| !known(s))
            .map(|&s| layout.slots[s].cap)
            .sum();
        if drawn > n || drawn + room < n {
            violations.push(Violation::new(
                1,
                cell(layout.cells[k], cols),
                vec![("number", i64::from(n)), ("bridges", i64::from(drawn))],
            ));
        }
    }
    for (i, through) in layout.through.iter().enumerate() {
        if let [Some(h), Some(v)] = *through
            && counts[h] > 0
            && counts[v] > 0
        {
            violations.push(Violation::new(2, cell(i, cols), Vec::new()));
        }
    }
    let groups = layout.groups(|s| counts[s] > 0 || !known(s));
    if groups > 1 {
        violations.push(Violation::new(
            3,
            Place::Board,
            vec![("groups", groups as i64)],
        ));
    }
    Ok(violations)
}
