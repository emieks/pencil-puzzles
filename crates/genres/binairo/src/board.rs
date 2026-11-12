//! The exact solver and the checker. Independent of the human techniques:
//! the solver filters each line's list of every valid line and keeps its
//! own propagation.
use crate::{Answer, BudgetExceeded, Mark, Puzzle, Shape};
use pencil_puzzles_core::{Budget, ParseError, Place, Solutions, SplitMix64, Violation};

/// Every line of `length` cells with as many 1s as 0s and no three equal
/// digits in a row, as sets of its 1s.
pub(crate) fn valid_lines(length: usize) -> Vec<u32> {
    fn extend(length: usize, line: u32, at: usize, ones: usize, out: &mut Vec<u32>) {
        if at == length {
            out.push(line);
            return;
        }
        let zeros = at - ones;
        for digit in [0, 1] {
            let count = if digit == 1 { ones } else { zeros };
            // The two cells before are this digit too.
            let run = at >= 2 && (0..2).all(|k| (line >> (at - 1 - k) & 1) as usize == digit);
            if count < length / 2 && !run {
                extend(
                    length,
                    line | (digit as u32) << at,
                    at + 1,
                    ones + digit,
                    out,
                );
            }
        }
    }
    let mut out = Vec::new();
    extend(length, 0, 0, 0, &mut out);
    out
}

/// The valid lines of one length, grouped by their first half: `start[b]`
/// is where the lines whose first half is `b` begin.
struct Table {
    half: usize,
    lines: Vec<u32>,
    start: Vec<usize>,
}

impl Table {
    fn new(length: usize) -> Self {
        let half = length / 2;
        let low = (1u32 << half) - 1;
        let mut lines = valid_lines(length);
        lines.sort_by_key(|&l| (l & low, l));
        let mut start = vec![0; (1 << half) + 1];
        for &l in &lines {
            start[(l & low) as usize + 1] += 1;
        }
        for b in 0..1 << half {
            start[b + 1] += start[b];
        }
        Self { half, lines, start }
    }
}

/// The search state: the 1s and the decided cells of every line, and the
/// number of options each line had when it was last filtered.
#[derive(Clone)]
struct State {
    ones: Vec<u32>,
    decided: Vec<u32>,
    options: Vec<usize>,
}

struct Solver<'a> {
    rows: usize,
    cols: usize,
    /// The valid lines of a row and of a column.
    row_lines: Table,
    col_lines: Table,
    budget: &'a Budget,
}

impl Solver<'_> {
    fn lines(&self) -> usize {
        self.rows + self.cols
    }

    fn table(&self, line: usize) -> &Table {
        if line < self.rows {
            &self.row_lines
        } else {
            &self.col_lines
        }
    }

    fn full(&self, line: usize) -> u32 {
        let length = if line < self.rows {
            self.cols
        } else {
            self.rows
        };
        (1 << length) - 1
    }

    fn parallel(&self, line: usize) -> std::ops::Range<usize> {
        if line < self.rows {
            0..self.rows
        } else {
            self.rows..self.lines()
        }
    }

    /// Write `digit` at place `k` of `line`, and in the crossing line.
    fn set(&self, s: &mut State, line: usize, k: usize, digit: bool) {
        let (cross, at) = if line < self.rows {
            (self.rows + k, line)
        } else {
            (k, line - self.rows)
        };
        for (l, bit) in [(line, k), (cross, at)] {
            s.decided[l] |= 1 << bit;
            if digit {
                s.ones[l] |= 1 << bit;
            }
        }
    }

    /// The valid lines that agree with a line's decided cells and differ
    /// from every other finished parallel line: their number, and the cells
    /// that are 1 in all of them and 0 in all of them.
    fn options(&self, s: &State, line: usize) -> Result<(usize, u32, u32), BudgetExceeded> {
        let table = self.table(line);
        let (decided, ones) = (s.decided[line], s.ones[line]);
        let mut finished: [u32; 2 * crate::MAX_SIZE] = [0; 2 * crate::MAX_SIZE];
        let mut count_finished = 0;
        for l in self.parallel(line) {
            if l != line && s.decided[l] == self.full(l) {
                finished[count_finished] = s.ones[l];
                count_finished += 1;
            }
        }
        let others = &finished[..count_finished];
        let (mut count, mut all, mut any) = (0, u32::MAX, 0);
        // The first halves that agree with the decided cells there.
        let low = (1 << table.half) - 1;
        let free = low & !decided;
        let mut sub = free;
        let mut examined = 0;
        loop {
            let b = ((ones & decided & low) | sub) as usize;
            let group = &table.lines[table.start[b]..table.start[b + 1]];
            examined += 1 + group.len();
            for &option in group {
                if (option ^ ones) & decided != 0 || others.contains(&option) {
                    continue;
                }
                count += 1;
                all &= option;
                any |= option;
            }
            if sub == 0 {
                break;
            }
            sub = (sub - 1) & free;
        }
        if !self.budget.spend(1 + examined as u64 / 32) {
            return Err(BudgetExceeded);
        }
        Ok((count, all, !any & self.full(line)))
    }

    /// Decide every cell that one line forces, until nothing changes. False
    /// if some line has no option left.
    fn propagate(&self, s: &mut State, mut todo: Vec<usize>) -> Result<bool, BudgetExceeded> {
        let mut queued = vec![false; self.lines()];
        for &l in &todo {
            queued[l] = true;
        }
        while let Some(line) = todo.pop() {
            queued[line] = false;
            let (count, all, none) = self.options(s, line)?;
            if count == 0 {
                return Ok(false);
            }
            s.options[line] = count;
            let new = (all | none) & !s.decided[line];
            if new == 0 {
                continue;
            }
            for k in 0..32 {
                if new >> k & 1 == 1 {
                    self.set(s, line, k, all >> k & 1 == 1);
                    let cross = if line < self.rows { self.rows + k } else { k };
                    if !queued[cross] {
                        queued[cross] = true;
                        todo.push(cross);
                    }
                }
            }
            if s.decided[line] == self.full(line) {
                // A finished line rules itself out of its parallel lines.
                for l in self.parallel(line) {
                    if l != line && !queued[l] && s.decided[l] != self.full(l) {
                        queued[l] = true;
                        todo.push(l);
                    }
                }
            }
        }
        Ok(true)
    }

    fn answer(&self, s: &State) -> Answer {
        let mut a = Answer::new(Shape::new(self.rows, self.cols).expect("valid shape"));
        for (r, ones) in s.ones[..self.rows].iter().enumerate() {
            for c in 0..self.cols {
                a.set(r, c, Mark::of((ones >> c & 1) as u8));
            }
        }
        a
    }

    /// The unfinished line with the fewest options when last filtered, with
    /// its count and open cells.
    fn branch(&self, s: &State) -> Option<(usize, usize, u32)> {
        let mut best: Option<(usize, usize, u32)> = None;
        for line in 0..self.lines() {
            let open = !s.decided[line] & self.full(line);
            if open != 0 && best.is_none_or(|(n, _, _)| s.options[line] < n) {
                best = Some((s.options[line], line, open));
            }
        }
        best
    }

    /// Find up to `limit` solutions, depth first. Each branch decides one
    /// cell, so the depth is at most the number of cells.
    fn search(
        &self,
        s: State,
        limit: usize,
        found: &mut Vec<Answer>,
    ) -> Result<(), BudgetExceeded> {
        if !self.budget.spend(1) {
            return Err(BudgetExceeded);
        }
        let best = self.branch(&s);
        let Some((_, line, open)) = best else {
            found.push(self.answer(&s));
            return Ok(());
        };
        let k = open.trailing_zeros() as usize;
        let cross = if line < self.rows { self.rows + k } else { k };
        for digit in [false, true] {
            let mut next = s.clone();
            self.set(&mut next, line, k, digit);
            if self.propagate(&mut next, vec![line, cross])? {
                self.search(next, limit, found)?;
                if found.len() >= limit {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// A random solution: like `search`, but deciding a random open cell of
    /// the line with the fewest options, trying its digits in random order.
    fn fill(&self, s: State, rng: &mut SplitMix64) -> Result<Option<Answer>, BudgetExceeded> {
        if !self.budget.spend(1) {
            return Err(BudgetExceeded);
        }
        let best = self.branch(&s);
        let Some((_, line, open)) = best else {
            return Ok(Some(self.answer(&s)));
        };
        let skip = rng.below(u64::from(open.count_ones()));
        let mut rest = open;
        for _ in 0..skip {
            rest &= rest - 1;
        }
        let k = rest.trailing_zeros() as usize;
        let cross = if line < self.rows { self.rows + k } else { k };
        let first = rng.below(2) == 1;
        for digit in [first, !first] {
            let mut next = s.clone();
            self.set(&mut next, line, k, digit);
            if self.propagate(&mut next, vec![line, cross])?
                && let Some(a) = self.fill(next, rng)?
            {
                return Ok(Some(a));
            }
        }
        Ok(None)
    }

    fn start(&self, p: &Puzzle) -> Result<Option<State>, BudgetExceeded> {
        let mut s = State {
            ones: vec![0; self.lines()],
            decided: vec![0; self.lines()],
            options: vec![usize::MAX; self.lines()],
        };
        for r in 0..self.rows {
            for c in 0..self.cols {
                if let Some(d) = p.given(r, c).digit() {
                    self.set(&mut s, r, c, d == 1);
                }
            }
        }
        Ok(self
            .propagate(&mut s, (0..self.lines()).collect())?
            .then_some(s))
    }

    fn run(&self, p: &Puzzle, limit: usize) -> Result<Vec<Answer>, BudgetExceeded> {
        let mut found = Vec::new();
        if let Some(s) = self.start(p)? {
            self.search(s, limit, &mut found)?;
        }
        Ok(found)
    }
}

fn solver(shape: Shape, budget: &Budget) -> Solver<'_> {
    Solver {
        rows: shape.rows(),
        cols: shape.cols(),
        row_lines: Table::new(shape.cols()),
        col_lines: Table::new(shape.rows()),
        budget,
    }
}

fn solutions(p: &Puzzle, limit: usize, budget: &Budget) -> Result<Vec<Answer>, BudgetExceeded> {
    solver(p.shape(), budget).run(p, limit)
}

/// A random complete grid that keeps every rule, if the shape has one.
pub(crate) fn random_grid(
    shape: Shape,
    rng: &mut SplitMix64,
    budget: &Budget,
) -> Result<Option<Answer>, BudgetExceeded> {
    let empty = Puzzle::new(shape, vec![Mark::Unmarked; shape.cells()]).expect("valid shape");
    let solver = solver(shape, budget);
    match solver.start(&empty)? {
        Some(s) => solver.fill(s, rng),
        None => Ok(None),
    }
}

/// Find no solution, exactly one, or two different ones (P5).
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut found = solutions(p, 2, budget)?;
    Ok(match found.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(found.remove(0)),
        _ => {
            let second = found.pop().expect("two solutions");
            Solutions::Multiple(found.remove(0), second)
        }
    })
}

/// The number of solutions, counting at most `limit`.
pub fn count_solutions(p: &Puzzle, limit: usize, budget: &Budget) -> Result<usize, BudgetExceeded> {
    Ok(solutions(p, limit, budget)?.len())
}

/// A row or column, counted from 0.
fn place(rows: usize, line: usize) -> Place {
    if line < rows {
        Place::Row(line as u32)
    } else {
        Place::Col((line - rows) as u32)
    }
}

/// Every broken rule instance of an answer, complete or partial (P8). A
/// cell holds the answer's digit, or its given digit if the answer leaves
/// it unmarked. R1 is
/// reported for every run of three or more equal digits, at its line with
/// `start` (its first cell along the line, from 0), `length` and `digit`.
/// R2 is reported for a line with more than half its cells one digit, with
/// `zeros` and `ones`. R3 is reported at the later of two equal finished
/// lines, with `same_as` (the earlier one, from 0). R4 is reported at a
/// given cell where the answer has the other digit, with `given` and
/// `actual`.
pub fn check(p: &Puzzle, a: &Answer) -> Result<Vec<Violation>, ParseError> {
    if a.shape() != p.shape() {
        return Err(ParseError::new("the answer has the wrong shape"));
    }
    let (rows, cols) = (p.shape().rows(), p.shape().cols());
    let mut out = Vec::new();
    let digit = |r: usize, c: usize| a.get(r, c).digit().or(p.given(r, c).digit());
    // Each line's digits, and whether it is finished.
    let lines: Vec<Vec<Option<u8>>> = (0..rows)
        .map(|r| (0..cols).map(|c| digit(r, c)).collect())
        .chain((0..cols).map(|c| (0..rows).map(|r| digit(r, c)).collect()))
        .collect();
    for (l, line) in lines.iter().enumerate() {
        let mut k = 0;
        while k < line.len() {
            let mut end = k + 1;
            while end < line.len() && line[k].is_some() && line[end] == line[k] {
                end += 1;
            }
            if let Some(d) = line[k]
                && end - k >= 3
            {
                out.push(Violation::new(
                    1,
                    place(rows, l),
                    vec![
                        ("start", k as i64),
                        ("length", (end - k) as i64),
                        ("digit", i64::from(d)),
                    ],
                ));
            }
            k = end;
        }
    }
    for (l, line) in lines.iter().enumerate() {
        let count = |d: u8| line.iter().filter(|&&x| x == Some(d)).count();
        let (zeros, ones) = (count(0), count(1));
        if zeros.max(ones) > line.len() / 2 {
            out.push(Violation::new(
                2,
                place(rows, l),
                vec![("zeros", zeros as i64), ("ones", ones as i64)],
            ));
        }
    }
    for (l, line) in lines.iter().enumerate() {
        let parallel = if l < rows { 0..l } else { rows..l };
        if line.iter().all(Option::is_some)
            && let Some(first) = parallel.clone().find(|&m| lines[m] == *line)
        {
            out.push(Violation::new(
                3,
                place(rows, l),
                vec![("same_as", (first - parallel.start) as i64)],
            ));
        }
    }
    for r in 0..rows {
        for c in 0..cols {
            if let (Some(given), Some(actual)) = (p.given(r, c).digit(), a.get(r, c).digit())
                && given != actual
            {
                out.push(Violation::new(
                    4,
                    cell(r * cols + c, cols),
                    vec![("given", i64::from(given)), ("actual", i64::from(actual))],
                ));
            }
        }
    }
    Ok(out)
}

/// Cell `i` as a place.
pub(crate) fn cell(i: usize, cols: usize) -> Place {
    Place::Cell {
        row: (i / cols) as u32,
        col: (i % cols) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_line_counts() {
        let counts: Vec<usize> = (1..=8).map(|n| valid_lines(2 * n).len()).collect();
        assert_eq!(counts, [2, 6, 14, 34, 84, 208, 518, 1296]);
    }
}
