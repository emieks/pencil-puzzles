//! The exact solver and the checker. Independent of the human techniques:
//! the solver decides whole groups (a tank's cells in one row) and keeps
//! its own propagation.
use crate::model::Layout;
use crate::{Answer, BudgetExceeded, Mark, Puzzle, Shape};
use pencil_puzzles_core::{Budget, ParseError, Place, Solutions, Violation};

const OPEN: u8 = 0;
const AIR: u8 = 1;
const WATER: u8 = 2;

/// A row or column: its clue and the groups with a cell in it, by tank.
/// In a row each tank has one group, with `size` cells there. In a column a
/// tank's groups are listed top to bottom and each has one cell there.
struct Line {
    clue: u32,
    size: Vec<u32>,
    tanks: Vec<Vec<usize>>,
}

struct Solver<'a> {
    rows: usize,
    lines: Vec<Line>,
    /// The lines each group has cells in.
    lines_of: Vec<Vec<usize>>,
    /// The group above and the group below each group in its tank.
    above: Vec<Option<usize>>,
    below: Vec<Option<usize>>,
    budget: &'a Budget,
}

impl<'a> Solver<'a> {
    fn new(p: &Puzzle, layout: &Layout, budget: &'a Budget) -> Self {
        let (rows, cols) = (layout.rows, layout.cols);
        let mut lines: Vec<Line> = (p.row_clues().iter().chain(p.col_clues()))
            .map(|&clue| Line {
                clue,
                size: Vec::new(),
                tanks: Vec::new(),
            })
            .collect();
        let mut lines_of = vec![Vec::new(); layout.groups.len()];
        let mut above = vec![None; layout.groups.len()];
        let mut below = vec![None; layout.groups.len()];
        for tank in &layout.tank_groups {
            for pair in tank.windows(2) {
                below[pair[0]] = Some(pair[1]);
                above[pair[1]] = Some(pair[0]);
            }
            // The tank's place in each column's list, once it has one.
            let mut place = vec![usize::MAX; cols];
            for &g in tank {
                let cells = &layout.groups[g];
                let row = &mut lines[cells[0] / cols];
                row.tanks.push(vec![g]);
                row.size.push(cells.len() as u32);
                lines_of[g].push(cells[0] / cols);
                for &i in cells {
                    let col = &mut lines[rows + i % cols];
                    if place[i % cols] == usize::MAX {
                        place[i % cols] = col.tanks.len();
                        col.tanks.push(Vec::new());
                        col.size.push(1);
                    }
                    col.tanks[place[i % cols]].push(g);
                    lines_of[g].push(rows + i % cols);
                }
            }
        }
        Self {
            rows,
            lines,
            lines_of,
            above,
            below,
            budget,
        }
    }

    fn spend(&self, n: usize) -> Result<(), BudgetExceeded> {
        if self.budget.spend(n as u64) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    /// What one line's count says about its open groups: the groups to
    /// decide are added to `todo`. False if the clue is out of reach.
    fn line(&self, state: &[u8], l: usize, todo: &mut Vec<(usize, u8)>) -> bool {
        let line = &self.lines[l];
        let mut water = 0;
        // Each tank's open cells in the line, and their total.
        let mut open = Vec::with_capacity(line.tanks.len());
        let mut total = 0;
        for (groups, &size) in line.tanks.iter().zip(&line.size) {
            let count = |value: u8| groups.iter().filter(|&&g| state[g] == value).count() as u32;
            water += count(WATER) * size;
            open.push(count(OPEN) * size);
            total += count(OPEN) * size;
        }
        if water > line.clue || water + total < line.clue {
            return false;
        }
        let need = line.clue - water;
        if l < self.rows {
            return self.row(state, line, &open, need, todo);
        }
        // A column: each tank's water fills its groups from the bottom,
        // and the tank holds between `least` and `most` of the need.
        for (groups, &open) in line.tanks.iter().zip(&open) {
            let most = open.min(need);
            let least = need.saturating_sub(total - open);
            let open_groups = groups.iter().filter(|&&g| state[g] == OPEN);
            for (k, &g) in open_groups.enumerate() {
                if (k as u32) < open - most {
                    todo.push((g, AIR));
                }
                if k as u32 >= open - least {
                    todo.push((g, WATER));
                }
            }
        }
        true
    }

    /// A row: each open group is all water or all air. A group gets the
    /// value it has in every way of making up the need. False if there is
    /// no way.
    fn row(
        &self,
        state: &[u8],
        line: &Line,
        open: &[u32],
        need: u32,
        todo: &mut Vec<(usize, u8)>,
    ) -> bool {
        // Bit `s` of `before[k]` is set when the first k tanks can hold
        // exactly `s` cells of water; `after[k]` is the same for the tanks
        // from the k-th on.
        let fits = (1u64 << (need + 1)) - 1;
        let n = open.len();
        let mut before = vec![1u64; n + 1];
        for k in 0..n {
            before[k + 1] = (before[k] | before[k] << open[k]) & fits;
        }
        if before[n] >> need & 1 == 0 {
            return false;
        }
        let mut after = vec![1u64; n + 1];
        for k in (0..n).rev() {
            after[k] = (after[k + 1] | after[k + 1] << open[k]) & fits;
        }
        for k in (0..n).filter(|&k| open[k] > 0) {
            let makes = |rest: u32| {
                (0..=rest).any(|s| before[k] >> s & 1 == 1 && after[k + 1] >> (rest - s) & 1 == 1)
            };
            let g = line.tanks[k][0];
            debug_assert_eq!(state[g], OPEN);
            if !makes(need) {
                todo.push((g, WATER));
            } else if open[k] > need || !makes(need - open[k]) {
                todo.push((g, AIR));
            }
        }
        true
    }

    /// Decide the groups in `todo` and apply the consequences: the water
    /// level in each tank, then the counts of every line in `dirty` or
    /// touched since. False on a clash.
    fn propagate(
        &self,
        state: &mut [u8],
        mut todo: Vec<(usize, u8)>,
        mut dirty: Vec<usize>,
    ) -> Result<bool, BudgetExceeded> {
        let mut listed = vec![false; self.lines.len()];
        for &l in &dirty {
            listed[l] = true;
        }
        loop {
            while let Some((g, value)) = todo.pop() {
                if state[g] == value {
                    continue;
                }
                if state[g] != OPEN {
                    return Ok(false);
                }
                state[g] = value;
                let next = if value == WATER {
                    self.below[g]
                } else {
                    self.above[g]
                };
                todo.extend(next.map(|n| (n, value)));
                for &l in &self.lines_of[g] {
                    if !listed[l] {
                        listed[l] = true;
                        dirty.push(l);
                    }
                }
            }
            let Some(l) = dirty.pop() else {
                return Ok(true);
            };
            listed[l] = false;
            self.spend(self.lines[l].tanks.len() + 1)?;
            if !self.line(state, l, &mut todo) {
                return Ok(false);
            }
        }
    }

    /// The open group to branch on: the largest.
    fn branch(&self, state: &[u8]) -> Option<usize> {
        (0..self.rows)
            .flat_map(|l| self.lines[l].tanks.iter().zip(&self.lines[l].size))
            .filter(|(groups, _)| state[groups[0]] == OPEN)
            .max_by_key(|&(groups, &size)| (size, usize::MAX - groups[0]))
            .map(|(groups, _)| groups[0])
    }

    /// Decide every open group that clashes with one of its two values,
    /// until none is left. False if a group clashes with both.
    fn probe(&self, state: &mut [u8]) -> Result<bool, BudgetExceeded> {
        let mut scratch = state.to_vec();
        let mut quiet = 0;
        let mut g = 0;
        while quiet < state.len() {
            quiet += 1;
            if state[g] == OPEN {
                for (value, other) in [(WATER, AIR), (AIR, WATER)] {
                    scratch.copy_from_slice(state);
                    if !self.propagate(&mut scratch, vec![(g, value)], Vec::new())? {
                        if !self.propagate(state, vec![(g, other)], Vec::new())? {
                            return Ok(false);
                        }
                        quiet = 0;
                        break;
                    }
                }
            }
            g = (g + 1) % state.len();
        }
        Ok(true)
    }

    fn search(
        &self,
        mut state: Vec<u8>,
        limit: usize,
        found: &mut Vec<Vec<u8>>,
    ) -> Result<(), BudgetExceeded> {
        // Each level decides one more group, so recursion is at most one
        // per group.
        self.spend(state.len())?;
        if !self.probe(&mut state)? {
            return Ok(());
        }
        let Some(g) = self.branch(&state) else {
            found.push(state);
            return Ok(());
        };
        for value in [WATER, AIR] {
            let mut next = state.clone();
            if self.propagate(&mut next, vec![(g, value)], Vec::new())? {
                self.search(next, limit, found)?;
            }
            if found.len() >= limit {
                break;
            }
        }
        Ok(())
    }
}

fn run(p: &Puzzle, limit: usize, budget: &Budget) -> Result<Vec<Answer>, BudgetExceeded> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let layout = Layout::new(p);
    let solver = Solver::new(p, &layout, budget);
    let mut state = vec![OPEN; layout.groups.len()];
    let mut found = Vec::new();
    let every_line = (0..solver.lines.len()).collect();
    if solver.propagate(&mut state, Vec::new(), every_line)? {
        solver.search(state, limit, &mut found)?;
    }
    Ok(found
        .iter()
        .map(|state| answer(p.shape(), &layout, state))
        .collect())
}

fn answer(shape: Shape, layout: &Layout, state: &[u8]) -> Answer {
    let mut a = Answer::new(shape);
    for (g, cells) in layout.groups.iter().enumerate() {
        let mark = if state[g] == WATER {
            Mark::Shaded
        } else {
            Mark::Unshaded
        };
        for &i in cells {
            a.set(i / shape.cols(), i % shape.cols(), mark);
        }
    }
    a
}

/// Prove zero, one or more solutions; two different ones when there are more.
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut found = run(p, 2, budget)?;
    Ok(match found.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(found.remove(0)),
        _ => {
            let second = found.remove(1);
            Solutions::Multiple(found.remove(0), second)
        }
    })
}

/// Count solutions, stopping at `limit`.
pub fn count_solutions(p: &Puzzle, limit: usize, budget: &Budget) -> Result<usize, BudgetExceeded> {
    run(p, limit, budget).map(|f| f.len())
}

pub(crate) fn cell(i: usize, cols: usize) -> Place {
    Place::Cell {
        row: (i / cols) as u32,
        col: (i % cols) as u32,
    }
}

/// Every broken rule of a complete or partial answer (P8).
///
/// - **R1**, at a row, and **R2**, at a column: its water cells outnumber
///   its clue, or its water and unmarked cells together fall short of it.
///   The values are `count`, the line's water cells, and `clue`.
/// - **R3**, at a tank (a region, numbered as in [`Puzzle::tanks`]): a cell
///   marked as air is level with or below a water cell of the tank.
///
/// Unmarked cells could still be water or air, so a rule is reported only
/// when no way of marking them could keep it. Fails if the answer has
/// another shape.
pub fn check(p: &Puzzle, a: &Answer) -> Result<Vec<Violation>, ParseError> {
    let shape = p.shape();
    if a.shape() != shape {
        return Err(ParseError::new("the answer has the wrong shape"));
    }
    let (rows, cols) = (shape.rows(), shape.cols());
    let mut violations = Vec::new();
    let mut line = |rule: u32, place: Place, marks: &mut dyn Iterator<Item = Mark>, clue: u32| {
        let (mut water, mut unmarked) = (0, 0);
        for mark in marks {
            match mark {
                Mark::Shaded => water += 1,
                Mark::Unmarked => unmarked += 1,
                Mark::Unshaded => {}
            }
        }
        if water > clue || water + unmarked < clue {
            violations.push(Violation::new(
                rule,
                place,
                vec![("count", i64::from(water)), ("clue", i64::from(clue))],
            ));
        }
    };
    for r in 0..rows {
        let mut marks = (0..cols).map(|c| a.get(r, c));
        line(1, Place::Row(r as u32), &mut marks, p.row_clues()[r]);
    }
    for c in 0..cols {
        let mut marks = (0..rows).map(|r| a.get(r, c));
        line(2, Place::Col(c as u32), &mut marks, p.col_clues()[c]);
    }
    // R3: each tank's highest water and lowest air.
    let mut top_water = vec![rows; p.tank_count()];
    let mut low_air = vec![None; p.tank_count()];
    for (i, &t) in p.tanks().iter().enumerate() {
        let (r, t) = (i / cols, t as usize);
        match a.get(r, i % cols) {
            Mark::Shaded => top_water[t] = top_water[t].min(r),
            Mark::Unshaded => low_air[t] = Some(r),
            Mark::Unmarked => {}
        }
    }
    for t in 0..p.tank_count() {
        if low_air[t].is_some_and(|air| air >= top_water[t]) {
            violations.push(Violation::new(3, Place::Region(t as u32), Vec::new()));
        }
    }
    Ok(violations)
}
