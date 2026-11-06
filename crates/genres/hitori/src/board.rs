//! The exact solver and the checker. Independent of the human techniques:
//! the solver keeps its own cell states and propagation.
use crate::{Answer, BudgetExceeded, Mark, Puzzle, Shape};
use pencil_puzzles_core::grid::{flood_from, neighbours};
use pencil_puzzles_core::{Budget, ParseError, Place, Solutions, Violation};

/// Every other cell in the same row or column holding the same number, for
/// each cell.
pub(crate) fn peers(p: &Puzzle) -> Vec<Vec<usize>> {
    let shape = p.shape();
    let (rows, cols) = (shape.rows(), shape.cols());
    let n = p.numbers();
    (0..shape.cells())
        .map(|i| {
            let (r, c) = (i / cols, i % cols);
            let mut out: Vec<usize> = (0..cols)
                .map(|k| r * cols + k)
                .chain((0..rows).map(|k| k * cols + c))
                .filter(|&j| j != i && n[j] == n[i])
                .collect();
            out.sort_unstable();
            out
        })
        .collect()
}

const OPEN: u8 = 0;
const WHITE: u8 = 1;
const BLACK: u8 = 2;

struct Solver<'a> {
    nb: Vec<Vec<usize>>,
    peers: Vec<Vec<usize>>,
    budget: &'a Budget,
}

impl Solver<'_> {
    fn spend(&self, n: usize) -> Result<(), BudgetExceeded> {
        if self.budget.spend(n as u64) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    /// Set `i` to `value` and apply its consequences. False on a clash.
    fn set(&self, state: &mut [u8], i: usize, value: u8) -> Result<bool, BudgetExceeded> {
        let mut queue = vec![(i, value)];
        while let Some((i, value)) = queue.pop() {
            if state[i] == value {
                continue;
            }
            if state[i] != OPEN {
                return Ok(false);
            }
            state[i] = value;
            let (others, other) = if value == BLACK {
                (&self.nb[i], WHITE)
            } else {
                (&self.peers[i], BLACK)
            };
            self.spend(others.len() + 1)?;
            for &j in others {
                if state[j] == value {
                    return Ok(false);
                }
                if state[j] == OPEN {
                    queue.push((j, other));
                }
            }
        }
        Ok(true)
    }

    /// Whether the cells not shaded are nonempty and connected.
    fn connected(&self, state: &[u8]) -> Result<bool, BudgetExceeded> {
        self.spend(state.len())?;
        let Some(start) = state.iter().position(|&s| s != BLACK) else {
            return Ok(false);
        };
        let reached = flood_from(&[start], |j| state[j] != BLACK, &self.nb, usize::MAX).len();
        Ok(reached == state.iter().filter(|&&s| s != BLACK).count())
    }

    /// The open cell to branch on: one with the most peers, else the first.
    fn branch(&self, state: &[u8]) -> Option<usize> {
        (0..state.len())
            .filter(|&i| state[i] == OPEN)
            .max_by_key(|&i| (self.peers[i].len(), usize::MAX - i))
    }

    fn search(
        &self,
        state: Vec<u8>,
        limit: usize,
        found: &mut Vec<Vec<u8>>,
    ) -> Result<(), BudgetExceeded> {
        // Each level decides one more cell, so recursion is at most one per cell.
        if found.len() >= limit || !self.connected(&state)? {
            return Ok(());
        }
        let Some(i) = self.branch(&state) else {
            found.push(state);
            return Ok(());
        };
        // Shading first finds the duplicates' usual answer sooner.
        for value in [BLACK, WHITE] {
            let mut next = state.clone();
            if self.set(&mut next, i, value)? {
                self.search(next, limit, found)?;
            }
            if found.len() >= limit {
                break;
            }
        }
        Ok(())
    }
}

fn run(p: &Puzzle, limit: usize, budget: &Budget) -> Result<Vec<Vec<u8>>, BudgetExceeded> {
    let shape = p.shape();
    let solver = Solver {
        nb: neighbours(shape.rows(), shape.cols()),
        peers: peers(p),
        budget,
    };
    let mut found = Vec::new();
    if limit > 0 {
        solver.search(vec![OPEN; shape.cells()], limit, &mut found)?;
    }
    Ok(found)
}

fn answer(shape: Shape, state: &[u8]) -> Answer {
    let mut a = Answer::new(shape);
    for (i, &s) in state.iter().enumerate() {
        let mark = if s == BLACK {
            Mark::Shaded
        } else {
            Mark::Unshaded
        };
        a.set(i / shape.cols(), i % shape.cols(), mark);
    }
    a
}

/// Prove zero, one or more solutions; two different ones when there are more.
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut found = run(p, 2, budget)?;
    let shape = p.shape();
    Ok(match found.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(answer(shape, &found[0])),
        _ => {
            let second = found.remove(1);
            Solutions::Multiple(answer(shape, &found[0]), answer(shape, &second))
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

/// Every broken rule of a complete or partial answer (P8). Unmarked cells
/// could still be shaded or not, so a rule is reported only when no way of
/// marking them could keep it.
pub fn check(
    p: &Puzzle,
    a: &Answer,
    budget: &Budget,
) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
    let shape = p.shape();
    if a.shape() != shape {
        return Err(ParseError::new("the answer has the wrong shape").into());
    }
    let (rows, cols) = (shape.rows(), shape.cols());
    let n = shape.cells();
    if !budget.spend((n * (rows + cols + 4)) as u64) {
        return Err(Box::new(BudgetExceeded));
    }
    let nb = neighbours(rows, cols);
    let marks: Vec<Mark> = (0..n).map(|i| a.get(i / cols, i % cols)).collect();
    let numbers = p.numbers();
    let mut violations = Vec::new();
    // R1: a number twice among the cells marked unshaded in a line.
    let lines = (0..rows)
        .map(|r| {
            (
                Place::Row(r as u32),
                (0..cols).map(|c| r * cols + c).collect::<Vec<_>>(),
            )
        })
        .chain((0..cols).map(|c| {
            (
                Place::Col(c as u32),
                (0..rows).map(|r| r * cols + c).collect(),
            )
        }));
    for (place, line) in lines {
        let mut count = vec![0i64; shape.largest() as usize + 1];
        for &i in &line {
            if marks[i] == Mark::Unshaded {
                count[numbers[i] as usize] += 1;
            }
        }
        for (number, &k) in count.iter().enumerate() {
            if k > 1 {
                violations.push(Violation::new(
                    1,
                    place,
                    vec![("number", number as i64), ("count", k)],
                ));
            }
        }
    }
    // R2: each shaded cell next to another shaded cell.
    for i in 0..n {
        if marks[i] == Mark::Shaded && nb[i].iter().any(|&j| marks[j] == Mark::Shaded) {
            violations.push(Violation::new(2, cell(i, cols), Vec::new()));
        }
    }
    // R3: the cells marked unshaded must connect through cells not marked
    // shaded, and some cell must be left unshaded.
    let open = |i: usize| marks[i] != Mark::Shaded;
    let mut seen = vec![false; n];
    let mut groups = 0;
    for i in 0..n {
        if marks[i] == Mark::Unshaded && !seen[i] {
            groups += 1;
            for j in flood_from(&[i], open, &nb, usize::MAX) {
                seen[j] = true;
            }
        }
    }
    if groups > 1 || marks.iter().all(|&m| m == Mark::Shaded) {
        violations.push(Violation::new(3, Place::Board, vec![("groups", groups)]));
    }
    violations.sort_by_key(|v| (v.rule, v.place));
    Ok(violations)
}
