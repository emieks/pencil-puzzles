//! The exact solver and the checker. Independent of the human techniques:
//! the solver tracks each run's used digits, remaining sum and open cells,
//! keeps for every empty cell the digits both its runs can still take, and
//! branches on a cell with the fewest.
use crate::{Answer, BudgetExceeded, Mark, Puzzle, Shape};
use pencil_puzzles_core::{Budget, ParseError, Place, Solutions, Violation};

/// Every digit, as bits: bit `d - 1` stands for digit `d`.
const DIGITS: u16 = 0x1FF;

/// For each set of available digits and each count, the sums that that
/// many different available digits can make, as bits.
pub(crate) struct Reach(Vec<[u64; 10]>);

impl Reach {
    pub(crate) fn new() -> Self {
        let mut table = vec![[0u64; 10]; 512];
        table[0][0] = 1;
        for set in 1..512usize {
            let low = set.trailing_zeros();
            let rest = table[set & (set - 1)];
            let mut row = rest;
            for k in 1..10 {
                row[k] |= rest[k - 1] << (low + 1);
            }
            table[set] = row;
        }
        Self(table)
    }

    /// Whether `count` different digits from `available` add up to `sum`.
    pub(crate) fn can(&self, available: u16, count: usize, sum: u32) -> bool {
        count < 10 && sum < 64 && (self.0[usize::from(available & DIGITS)][count] >> sum) & 1 == 1
    }
}

/// The search state: each white cell's digit and each run's progress.
struct Search<'a> {
    reach: Reach,
    budget: &'a Budget,
    shape: Shape,
    whites: Vec<usize>,
    /// For each cell, the indices of its two runs.
    runs_of: Vec<[usize; 2]>,
    /// The cells of each run.
    runs: Vec<Vec<usize>>,
    digit: Vec<u8>,
    used: Vec<u16>,
    left: Vec<u32>,
    open: Vec<usize>,
    /// The digits each empty cell can still take.
    allowed: Vec<u16>,
}

enum Choice {
    Solved,
    Dead,
    Cell(usize, u16),
}

impl Search<'_> {
    fn new<'a>(p: &Puzzle, budget: &'a Budget) -> Search<'a> {
        let n = p.cells().len();
        let mut runs_of = vec![[usize::MAX; 2]; n];
        for (k, run) in p.runs().iter().enumerate() {
            for &i in &run.cells {
                runs_of[i][usize::from(run.down)] = k;
            }
        }
        let mut s = Search {
            reach: Reach::new(),
            budget,
            shape: p.shape(),
            whites: (0..n).filter(|&i| runs_of[i][0] != usize::MAX).collect(),
            runs_of,
            runs: p.runs().iter().map(|r| r.cells.clone()).collect(),
            digit: vec![0; n],
            used: vec![0; p.runs().len()],
            left: p.runs().iter().map(|r| r.sum).collect(),
            open: p.runs().iter().map(|r| r.cells.len()).collect(),
            allowed: vec![0; n],
        };
        for &x in &s.whites {
            s.allowed[x] = s.compute(x);
        }
        s
    }

    /// Recompute the digits allowed in the empty cells of `x`'s two runs.
    fn refresh(&mut self, x: usize) {
        for r in self.runs_of[x] {
            for k in 0..self.runs[r].len() {
                let y = self.runs[r][k];
                if self.digit[y] == 0 {
                    self.allowed[y] = self.compute(y);
                }
            }
        }
    }

    /// Digits allowed in the empty cell `x`: both its runs can still reach
    /// their sums with different digits.
    fn compute(&self, x: usize) -> u16 {
        let [a, b] = self.runs_of[x];
        let mut out = 0;
        let mut rest = DIGITS & !(self.used[a] | self.used[b]);
        while rest != 0 {
            let bit = rest & rest.wrapping_neg();
            rest &= !bit;
            let d = bit.trailing_zeros() + 1;
            let fits = |r: usize| {
                self.left[r] >= d
                    && self.reach.can(
                        DIGITS & !self.used[r] & !bit,
                        self.open[r] - 1,
                        self.left[r] - d,
                    )
            };
            if fits(a) && fits(b) {
                out |= bit;
            }
        }
        out
    }

    fn set(&mut self, x: usize, d: u8) {
        for r in self.runs_of[x] {
            self.used[r] |= 1 << (d - 1);
            self.left[r] -= u32::from(d);
            self.open[r] -= 1;
        }
        self.digit[x] = d;
        self.refresh(x);
    }

    fn clear(&mut self, x: usize) {
        let d = self.digit[x];
        for r in self.runs_of[x] {
            self.used[r] &= !(1 << (d - 1));
            self.left[r] += u32::from(d);
            self.open[r] += 1;
        }
        self.digit[x] = 0;
        self.refresh(x);
    }

    /// The empty cell with the fewest allowed digits, lowest first.
    fn choose(&self) -> Choice {
        let mut best: Option<(u32, usize, u16)> = None;
        for &x in &self.whites {
            if self.digit[x] != 0 {
                continue;
            }
            let allowed = self.allowed[x];
            if allowed == 0 {
                return Choice::Dead;
            }
            if best.is_none_or(|(count, _, _)| allowed.count_ones() < count) {
                best = Some((allowed.count_ones(), x, allowed));
            }
        }
        best.map_or(Choice::Solved, |(_, x, allowed)| Choice::Cell(x, allowed))
    }

    fn answer(&self) -> Answer {
        let cols = self.shape.cols();
        let mut a = Answer::new(self.shape);
        for &x in &self.whites {
            a.set(x / cols, x % cols, Mark::Digit(self.digit[x]));
        }
        a
    }
}

/// Visit up to `limit` solutions, depth first with an explicit stack.
fn run(
    p: &Puzzle,
    limit: usize,
    budget: &Budget,
    visit: &mut dyn FnMut(Answer),
) -> Result<usize, BudgetExceeded> {
    search(Search::new(p, budget), limit, usize::MAX, visit).map(|(found, _)| found)
}

/// Visit up to `limit` solutions of a search, depth first with an explicit
/// stack, giving up after `max_nodes` cells tried. Returns the number found
/// and whether the search finished.
fn search(
    mut s: Search<'_>,
    limit: usize,
    max_nodes: usize,
    visit: &mut dyn FnMut(Answer),
) -> Result<(usize, bool), BudgetExceeded> {
    let mut found = 0;
    let mut nodes = 0;
    // Each frame: a cell and the digits not tried there yet.
    let mut stack: Vec<(usize, u16)> = Vec::new();
    if limit == 0 {
        return Ok((0, true));
    }
    loop {
        if !s.budget.spend(s.whites.len() as u64 + 1) {
            return Err(BudgetExceeded);
        }
        nodes += 1;
        if nodes > max_nodes {
            return Ok((found, false));
        }
        match s.choose() {
            Choice::Dead => {}
            Choice::Solved => {
                found += 1;
                visit(s.answer());
                if found >= limit {
                    return Ok((found, true));
                }
            }
            Choice::Cell(x, allowed) => stack.push((x, allowed)),
        }
        // Try the next digit of the deepest cell that has one left.
        loop {
            let Some(&(x, rest)) = stack.last() else {
                return Ok((found, true));
            };
            if s.digit[x] != 0 {
                s.clear(x);
            }
            if rest == 0 {
                stack.pop();
                continue;
            }
            let bit = rest & rest.wrapping_neg();
            stack.last_mut().expect("nonempty").1 &= !bit;
            s.set(x, bit.trailing_zeros() as u8 + 1);
            break;
        }
    }
}

/// Prove zero, one or more solutions; two different ones when there are more.
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut answers = Vec::new();
    run(p, 2, budget, &mut |a| answers.push(a))?;
    Ok(match answers.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(answers.remove(0)),
        _ => Solutions::Multiple(answers.remove(0), answers.remove(0)),
    })
}

/// Up to `limit` solutions within the candidates `masks` (one per white
/// cell in row-major order), in search order, giving up after `max_nodes`
/// cells tried. The second value is whether the search finished.
pub(crate) fn solutions_within(
    p: &Puzzle,
    masks: &[u16],
    limit: usize,
    max_nodes: usize,
    budget: &Budget,
) -> Result<(Vec<Answer>, bool), BudgetExceeded> {
    let mut s = Search::new(p, budget);
    for (v, &x) in s.whites.clone().iter().enumerate() {
        s.allowed[x] &= masks[v];
    }
    let mut answers = Vec::new();
    let (_, complete) = search(s, limit, max_nodes, &mut |a| answers.push(a))?;
    Ok((answers, complete))
}

/// Count solutions, stopping at `limit`.
pub fn count_solutions(p: &Puzzle, limit: usize, budget: &Budget) -> Result<usize, BudgetExceeded> {
    run(p, limit, budget, &mut |_| {})
}

pub(crate) fn cell(i: usize, cols: usize) -> Place {
    Place::Cell {
        row: (i / cols) as u32,
        col: (i % cols) as u32,
    }
}

/// Every broken rule of a complete or partial answer (P8). A run is
/// reported when its digits repeat, or when no different digits unused in
/// it can fill its unmarked cells to reach its sum.
pub fn check(
    p: &Puzzle,
    a: &Answer,
    budget: &Budget,
) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
    if !p.fits(a) {
        return Err(ParseError::new("the answer does not fit the puzzle").into());
    }
    let n = p.cells().len();
    if !budget.spend(n as u64 * 10 + 512 * 10) {
        return Err(Box::new(BudgetExceeded));
    }
    let reach = Reach::new();
    let cols = p.shape().cols();
    let marks = a.marks();
    let mut violations = Vec::new();
    for run in p.runs() {
        let (mut used, mut total, mut open, mut repeated) = (0u16, 0u32, 0, None);
        for &i in &run.cells {
            match marks[i] {
                Mark::Digit(d) => {
                    let bit = 1 << (d - 1);
                    if used & bit != 0 && repeated.is_none() {
                        repeated = Some(d);
                    }
                    used |= bit;
                    total += u32::from(d);
                }
                Mark::Unmarked => open += 1,
                Mark::Empty | Mark::Block => {}
            }
        }
        let reachable = total <= run.sum && reach.can(DIGITS & !used, open, run.sum - total);
        if repeated.is_some() || !reachable {
            let mut values = vec![
                ("down", i64::from(run.down)),
                ("sum", i64::from(run.sum)),
                ("total", i64::from(total)),
            ];
            values.extend(repeated.map(|d| ("repeated", i64::from(d))));
            violations.push(Violation::new(1, cell(run.clue, cols), values));
        }
    }
    for (i, &m) in marks.iter().enumerate() {
        if m == Mark::Empty {
            violations.push(Violation::new(2, cell(i, cols), Vec::new()));
        }
    }
    violations.sort_by_key(|v| (v.rule, v.place));
    Ok(violations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reachable_sums() {
        let reach = Reach::new();
        assert!(reach.can(DIGITS, 2, 3));
        assert!(!reach.can(DIGITS, 2, 2));
        assert!(reach.can(DIGITS, 9, 45));
        assert!(!reach.can(DIGITS, 9, 44));
        assert!(reach.can(DIGITS, 0, 0));
        assert!(!reach.can(0b110, 2, 3));
        assert!(reach.can(0b101, 2, 4));
    }
}
