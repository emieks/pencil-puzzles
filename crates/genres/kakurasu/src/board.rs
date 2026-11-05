//! A partly solved puzzle: the candidates left for every cell.

use std::collections::VecDeque;

use pencil_puzzles_core::Budget;

use crate::line::{Cell, Contradiction, LineSolver};
use crate::{Answer, Mark, Puzzle};

/// The cell may still be shaded.
pub(crate) const SHADED: u8 = 0b01;
/// The cell may still be unshaded.
pub(crate) const UNSHADED: u8 = 0b10;
/// Both candidates are left.
pub(crate) const BOTH: u8 = SHADED | UNSHADED;

/// The candidates of every cell, as bit sets of [`SHADED`] and [`UNSHADED`].
/// A cell with no candidates left makes the state contradictory.
///
/// Rule instances, called lines here, are numbered with rows first: line `r`
/// is row `r`, and line `rows + c` is column `c`. This is also their
/// canonical order.
///
/// Every line solve spends one operation of the budget. Once the budget runs
/// out, every line solve reports a contradiction, so that work stops quickly;
/// callers then check the budget and discard the result.
#[derive(Clone, Debug)]
pub(crate) struct Board<'a> {
    puzzle: &'a Puzzle,
    budget: &'a Budget,
    cells: Vec<u8>,
}

/// A deduction about one cell: the candidates to keep.
pub(crate) type Change = (usize, u8);

impl<'a> Board<'a> {
    pub(crate) fn new(puzzle: &'a Puzzle, budget: &'a Budget) -> Self {
        Self {
            puzzle,
            budget,
            cells: vec![BOTH; puzzle.rows() * puzzle.cols()],
        }
    }

    pub(crate) fn puzzle(&self) -> &'a Puzzle {
        self.puzzle
    }

    pub(crate) fn rows(&self) -> usize {
        self.puzzle.rows()
    }

    pub(crate) fn cols(&self) -> usize {
        self.puzzle.cols()
    }

    pub(crate) fn line_count(&self) -> usize {
        self.rows() + self.cols()
    }

    pub(crate) fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub(crate) fn candidates(&self, index: usize) -> u8 {
        self.cells[index]
    }

    pub(crate) fn is_contradictory(&self) -> bool {
        self.cells.contains(&0)
    }

    /// Removes every candidate of `index` except those in `keep`. Returns the
    /// candidates removed.
    pub(crate) fn restrict(&mut self, index: usize, keep: u8) -> u8 {
        let old = self.cells[index];
        self.cells[index] = old & keep;
        old & !keep
    }

    /// The cells on `line`, in order: column by column for a row, row by row
    /// for a column. The `k`th cell is worth `k + 1`.
    pub(crate) fn line_cells(&self, line: usize) -> impl Iterator<Item = usize> + use<> {
        let (rows, cols) = (self.rows(), self.cols());
        let (start, step, len) = if line < rows {
            (line * cols, 1, cols)
        } else {
            (line - rows, cols, rows)
        };
        (0..len).map(move |k| start + k * step)
    }

    /// The two lines through a cell: its row and its column.
    pub(crate) fn lines_of(&self, index: usize) -> [usize; 2] {
        [index / self.cols(), self.rows() + index % self.cols()]
    }

    pub(crate) fn target(&self, line: usize) -> u32 {
        if line < self.rows() {
            self.puzzle.row_clues()[line]
        } else {
            self.puzzle.col_clues()[line - self.rows()]
        }
    }

    /// Applies `solver` to `line` without changing the board. Returns the
    /// changes it would make, or `Err` if the line is broken (or the budget
    /// has run out).
    pub(crate) fn evaluate(
        &self,
        line: usize,
        solver: LineSolver,
        cells: &mut Vec<Cell>,
    ) -> Result<Vec<Change>, Contradiction> {
        if !self.budget.spend(1) {
            return Err(Contradiction);
        }
        cells.clear();
        cells.extend(self.line_cells(line).map(|index| match self.cells[index] {
            SHADED => Cell::Shaded,
            UNSHADED => Cell::Empty,
            _ => Cell::Unknown,
        }));
        if self.line_cells(line).any(|index| self.cells[index] == 0) {
            return Err(Contradiction);
        }
        solver(cells, self.target(line))?;
        Ok(self
            .line_cells(line)
            .zip(cells.iter())
            .filter(|&(index, &cell)| self.cells[index] == BOTH && cell != Cell::Unknown)
            .map(|(index, &cell)| {
                (
                    index,
                    if cell == Cell::Shaded {
                        SHADED
                    } else {
                        UNSHADED
                    },
                )
            })
            .collect())
    }

    /// Applies `solver` to every line until nothing changes, in any order.
    /// Used by the solver and generator, where order doesn't matter.
    pub(crate) fn propagate(&mut self, solver: LineSolver) -> Result<(), Contradiction> {
        self.propagate_lines(0..self.line_count(), solver)
    }

    /// Sets a cell, then propagates from its lines.
    pub(crate) fn assume(
        &mut self,
        index: usize,
        keep: u8,
        solver: LineSolver,
    ) -> Result<(), Contradiction> {
        self.restrict(index, keep);
        if self.cells[index] == 0 {
            return Err(Contradiction);
        }
        let lines = self.lines_of(index);
        self.propagate_lines(lines.into_iter(), solver)
    }

    fn propagate_lines(
        &mut self,
        lines: impl Iterator<Item = usize>,
        solver: LineSolver,
    ) -> Result<(), Contradiction> {
        let mut queued = vec![false; self.line_count()];
        let mut queue = VecDeque::with_capacity(self.line_count());
        for line in lines {
            queued[line] = true;
            queue.push_back(line);
        }
        let mut cells = Vec::with_capacity(self.rows().max(self.cols()));
        while let Some(line) = queue.pop_front() {
            queued[line] = false;
            for (index, keep) in self.evaluate(line, solver, &mut cells)? {
                self.restrict(index, keep);
                for crossing in self.lines_of(index) {
                    if !queued[crossing] {
                        queued[crossing] = true;
                        queue.push_back(crossing);
                    }
                }
            }
        }
        Ok(())
    }

    /// An unsolved cell to branch on when searching: one in the line with
    /// the fewest unsolved cells, where a guess is most likely to lead to
    /// further deductions.
    pub(crate) fn branch_cell(&self) -> Option<usize> {
        (0..self.line_count())
            .filter_map(|line| {
                let mut unsolved = self
                    .line_cells(line)
                    .filter(|&index| self.cells[index] == BOTH);
                let first = unsolved.next()?;
                Some((1 + unsolved.count(), first))
            })
            .min_by_key(|&(unsolved, _)| unsolved)
            .map(|(_, index)| index)
    }

    /// The board as an answer: unsolved cells are unmarked.
    pub(crate) fn to_answer(&self) -> Answer {
        let mut answer = Answer::new(self.puzzle.shape());
        for (index, &cell) in self.cells.iter().enumerate() {
            let mark = match cell {
                SHADED => Mark::Shaded,
                UNSHADED => Mark::Unshaded,
                _ => Mark::Unmarked,
            };
            answer.set(index / self.cols(), index % self.cols(), mark);
        }
        answer
    }
}
