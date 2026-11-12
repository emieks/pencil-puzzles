//! The exact solver and the checker. Independent of the human techniques:
//! the solver keeps its own propagation over 2x2 blocks and region counts.
use crate::{Answer, BudgetExceeded, Mark, Puzzle, Shape};
use pencil_puzzles_core::{Budget, ParseError, Place, Solutions, Violation, grid};

/// Shaded and unshaded cells, one bitset per row.
#[derive(Clone)]
struct State {
    shaded: Vec<u32>,
    unshaded: Vec<u32>,
}

struct Solver<'a> {
    rows: usize,
    cols: usize,
    /// Every cell of a row.
    full: u32,
    /// Each numbered region's clue and its cells, as (row, set) pairs.
    regions: Vec<(u32, Vec<(usize, u32)>)>,
    /// Whether each search node probes its open cells.
    probes: bool,
    budget: &'a Budget,
}

/// The 2x2 blocks of two rows that force or break something. Bit `c`
/// stands for the block in columns `c` and `c + 1`. The four cells of a
/// block are `a` (top left), `b` (top right), `c` (bottom left) and `d`
/// (bottom right); `s` are shaded and `u` unshaded sets.
struct Blocks {
    /// Per cell, the blocks where the cell is open and must be shaded.
    shade: [u32; 4],
    /// Per cell, the blocks where the cell is open and must be unshaded.
    unshade: [u32; 4],
    /// Blocks with exactly three shaded cells and one unshaded.
    broken: u32,
}

fn blocks(s: [u32; 4], u: [u32; 4], width: u32) -> Blocks {
    let mut shade = [0; 4];
    let mut unshade = [0; 4];
    let mut broken = 0;
    for (k, &[x, y, z]) in [[1, 2, 3], [0, 2, 3], [0, 1, 3], [0, 1, 2]]
        .iter()
        .enumerate()
    {
        let three = s[x] & s[y] & s[z];
        let two_and_one = (s[x] & s[y] & u[z]) | (s[x] & u[y] & s[z]) | (u[x] & s[y] & s[z]);
        let open = !s[k] & !u[k] & width;
        shade[k] = three & open;
        unshade[k] = two_and_one & open;
        broken |= three & u[k];
    }
    Blocks {
        shade,
        unshade,
        broken: broken & width,
    }
}

impl<'a> Solver<'a> {
    fn new(p: &Puzzle, budget: &'a Budget) -> Self {
        let (rows, cols) = (p.shape().rows(), p.shape().cols());
        let mut cells: Vec<Vec<u32>> = vec![vec![0; rows]; p.region_count()];
        for (i, &t) in p.regions().iter().enumerate() {
            cells[t as usize][i / cols] |= 1 << (i % cols);
        }
        let regions = (p.clues().iter().zip(cells))
            .filter_map(|(clue, rows)| {
                let rows = rows.into_iter().enumerate().filter(|&(_, s)| s != 0);
                clue.map(|n| (n, rows.collect()))
            })
            .collect();
        Self {
            rows,
            cols,
            full: (1 << cols) - 1,
            regions,
            probes: true,
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

    /// Apply the 2x2 rule and the region counts until nothing changes.
    /// False on a clash.
    fn propagate(&self, st: &mut State) -> Result<bool, BudgetExceeded> {
        let width = self.full >> 1;
        loop {
            self.spend(self.rows + self.regions.len())?;
            let mut changed = false;
            for r in 0..self.rows.saturating_sub(1) {
                let (s0, s1) = (st.shaded[r], st.shaded[r + 1]);
                let (u0, u1) = (st.unshaded[r], st.unshaded[r + 1]);
                let b = blocks(
                    [s0, s0 >> 1, s1, s1 >> 1],
                    [u0, u0 >> 1, u1, u1 >> 1],
                    width,
                );
                if b.broken != 0 {
                    return Ok(false);
                }
                let shade = [b.shade[0] | b.shade[1] << 1, b.shade[2] | b.shade[3] << 1];
                let unshade = [
                    b.unshade[0] | b.unshade[1] << 1,
                    b.unshade[2] | b.unshade[3] << 1,
                ];
                for k in 0..2 {
                    if shade[k] & unshade[k] != 0 {
                        return Ok(false);
                    }
                    if shade[k] | unshade[k] != 0 {
                        changed = true;
                        st.shaded[r + k] |= shade[k];
                        st.unshaded[r + k] |= unshade[k];
                    }
                }
            }
            for (clue, cells) in &self.regions {
                let (mut shaded, mut open) = (0, 0);
                for &(r, set) in cells {
                    shaded += (st.shaded[r] & set).count_ones();
                    open += (set & !st.shaded[r] & !st.unshaded[r]).count_ones();
                }
                if shaded > *clue || shaded + open < *clue {
                    return Ok(false);
                }
                if open > 0 && (shaded == *clue || shaded + open == *clue) {
                    changed = true;
                    for &(r, set) in cells {
                        let open = set & !st.shaded[r] & !st.unshaded[r];
                        if shaded == *clue {
                            st.unshaded[r] |= open;
                        } else {
                            st.shaded[r] |= open;
                        }
                    }
                }
            }
            if !changed {
                return Ok(true);
            }
        }
    }

    /// The open cell to branch on: the first one beside a shaded cell, or
    /// else the first open cell.
    fn branch(&self, st: &State) -> Option<(usize, usize)> {
        let open = |r: usize| self.full & !st.shaded[r] & !st.unshaded[r];
        let mut first = None;
        for r in 0..self.rows {
            let o = open(r);
            if o == 0 {
                continue;
            }
            let s = st.shaded[r];
            let mut near = (s << 1 | s >> 1) & o;
            if r > 0 {
                near |= st.shaded[r - 1] & o;
            }
            if r + 1 < self.rows {
                near |= st.shaded[r + 1] & o;
            }
            if near != 0 {
                return Some((r, near.trailing_zeros() as usize));
            }
            first = first.or(Some((r, o.trailing_zeros() as usize)));
        }
        first
    }

    /// Decide every open cell that clashes with one of its two values,
    /// until none is left. False if a cell clashes with both.
    fn probe(&self, st: &mut State) -> Result<bool, BudgetExceeded> {
        let cells = self.rows * self.cols;
        let mut quiet = 0;
        let mut i = 0;
        while quiet < cells {
            quiet += 1;
            let (r, bit) = (i / self.cols, 1 << (i % self.cols));
            if (st.shaded[r] | st.unshaded[r]) & bit == 0 {
                for shade in [true, false] {
                    let mut trial = st.clone();
                    let set = |s: &mut State, shade: bool| {
                        if shade {
                            s.shaded[r] |= bit;
                        } else {
                            s.unshaded[r] |= bit;
                        }
                    };
                    set(&mut trial, shade);
                    if !self.propagate(&mut trial)? {
                        set(st, !shade);
                        if !self.propagate(st)? {
                            return Ok(false);
                        }
                        quiet = 0;
                        break;
                    }
                }
            }
            i = (i + 1) % cells;
        }
        Ok(true)
    }

    fn search(
        &self,
        mut st: State,
        limit: usize,
        found: &mut Vec<State>,
    ) -> Result<(), BudgetExceeded> {
        // Each level decides one more cell, so recursion is at most one per
        // cell.
        if !self.propagate(&mut st)? || (self.probes && !self.probe(&mut st)?) {
            return Ok(());
        }
        let Some((r, c)) = self.branch(&st) else {
            found.push(st);
            return Ok(());
        };
        for shade in [true, false] {
            let mut next = st.clone();
            if shade {
                next.shaded[r] |= 1 << c;
            } else {
                next.unshaded[r] |= 1 << c;
            }
            self.search(next, limit, found)?;
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
    let solver = Solver::new(p, budget);
    let st = State {
        shaded: vec![0; solver.rows],
        unshaded: vec![0; solver.rows],
    };
    let mut found = Vec::new();
    solver.search(st, limit, &mut found)?;
    Ok(found
        .iter()
        .map(|st| answer(p.shape(), solver.cols, st))
        .collect())
}

/// The one solution that agrees with `masks` (one candidate mask per cell,
/// bit 0 for unshaded and bit 1 for shaded), as each cell's shading, or
/// `None` if there are none or several. The search does not probe, which
/// finds a second solution sooner.
pub(crate) fn unique_from(
    p: &Puzzle,
    masks: &[u16],
    budget: &Budget,
) -> Result<Option<Vec<bool>>, BudgetExceeded> {
    let mut solver = Solver::new(p, budget);
    solver.probes = false;
    let mut st = State {
        shaded: vec![0; solver.rows],
        unshaded: vec![0; solver.rows],
    };
    for (i, &m) in masks.iter().enumerate() {
        let (r, c) = (i / solver.cols, i % solver.cols);
        match m {
            0 => return Ok(None),
            1 => st.unshaded[r] |= 1 << c,
            2 => st.shaded[r] |= 1 << c,
            _ => {}
        }
    }
    let mut found = Vec::new();
    solver.search(st, 2, &mut found)?;
    Ok((found.len() == 1).then(|| {
        (0..masks.len())
            .map(|i| found[0].shaded[i / solver.cols] >> (i % solver.cols) & 1 == 1)
            .collect()
    }))
}

fn answer(shape: Shape, cols: usize, st: &State) -> Answer {
    let mut a = Answer::new(shape);
    for (r, &row) in st.shaded.iter().enumerate() {
        for c in 0..cols {
            let mark = if row >> c & 1 == 1 {
                Mark::Shaded
            } else {
                Mark::Unshaded
            };
            a.set(r, c, mark);
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
/// - **R1**, at a numbered region (numbered as in [`Puzzle::regions`]): its
///   shaded cells outnumber its clue, or its shaded and unmarked cells
///   together fall short of it. The values are `count`, the region's shaded
///   cells, and `clue`.
/// - **R2**, at the first cell (reading row by row) of a group of shaded
///   cells joined by their sides: the smallest rectangle around the group
///   holds a cell marked unshaded, so the group cannot become a rectangle.
///
/// Unmarked cells could still be shaded or unshaded, so a rule is reported
/// only when no way of marking them could keep it. Fails if the answer has
/// another shape.
pub fn check(p: &Puzzle, a: &Answer) -> Result<Vec<Violation>, ParseError> {
    let shape = p.shape();
    if a.shape() != shape {
        return Err(ParseError::new("the answer has the wrong shape"));
    }
    let (rows, cols) = (shape.rows(), shape.cols());
    let mark = |i: usize| a.get(i / cols, i % cols);
    let mut violations = Vec::new();
    let mut shaded = vec![0u32; p.region_count()];
    let mut unmarked = vec![0u32; p.region_count()];
    for (i, &t) in p.regions().iter().enumerate() {
        match mark(i) {
            Mark::Shaded => shaded[t as usize] += 1,
            Mark::Unmarked => unmarked[t as usize] += 1,
            Mark::Unshaded => {}
        }
    }
    for (t, clue) in p.clues().iter().enumerate() {
        if let &Some(clue) = clue
            && (shaded[t] > clue || shaded[t] + unmarked[t] < clue)
        {
            violations.push(Violation::new(
                1,
                Place::Region(t as u32),
                vec![("count", i64::from(shaded[t])), ("clue", i64::from(clue))],
            ));
        }
    }
    let nb = grid::neighbours(rows, cols);
    let mut seen = vec![false; shape.cells()];
    for start in 0..shape.cells() {
        if seen[start] || mark(start) != Mark::Shaded {
            continue;
        }
        let group = grid::flood(start, |j| mark(j) == Mark::Shaded, &nb);
        let (mut top, mut bottom, mut left, mut right) = (rows, 0, cols, 0);
        for &i in &group {
            seen[i] = true;
            (top, bottom) = (top.min(i / cols), bottom.max(i / cols));
            (left, right) = (left.min(i % cols), right.max(i % cols));
        }
        let gap = (top..=bottom).any(|r| (left..=right).any(|c| a.get(r, c) == Mark::Unshaded));
        if gap {
            violations.push(Violation::new(2, cell(start, cols), Vec::new()));
        }
    }
    Ok(violations)
}
