//! Finding solutions: line reasoning plus backtracking search.

use std::ops::ControlFlow;

use pencil_puzzles_core::{Budget, BudgetExceeded};

use crate::board::{BOTH, Board, SHADED, UNSHADED};
use crate::line::solve_full;
use crate::{Answer, Puzzle};

/// The solutions of a Kakurasu puzzle.
pub type Solutions = pencil_puzzles_core::Solutions<Answer>;

/// Solves a puzzle, finding up to two solutions to tell whether the solution
/// is unique (P5, G1).
///
/// ```
/// use pencil_puzzles_kakurasu::{Budget, Puzzle, Solutions};
///
/// let puzzle: Puzzle = "rows: 1 2\ncols: 1 2".parse().unwrap();
/// let expected = "#-\n-#".parse().unwrap();
/// assert_eq!(
///     pencil_puzzles_kakurasu::solve(&puzzle, &Budget::unlimited()),
///     Ok(Solutions::Unique(expected))
/// );
/// ```
pub fn solve(puzzle: &Puzzle, budget: &Budget) -> Result<Solutions, BudgetExceeded> {
    let solutions = solve_board(Board::new(puzzle, budget));
    budget.check()?;
    Ok(solutions)
}

/// Counts a puzzle's solutions, stopping once `limit` have been found.
pub fn count_solutions(
    puzzle: &Puzzle,
    limit: usize,
    budget: &Budget,
) -> Result<usize, BudgetExceeded> {
    let mut count = 0;
    let mut board = Board::new(puzzle, budget);
    if limit > 0 && board.propagate(solve_full).is_ok() {
        let _ = search(board, None, &mut |_| {
            count += 1;
            if count == limit {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
    }
    budget.check()?;
    Ok(count)
}

/// Like [`solve`], but continues from a partly solved board. The caller
/// checks the budget.
pub(crate) fn solve_board(mut board: Board<'_>) -> Solutions {
    if board.propagate(solve_full).is_err() {
        return Solutions::None;
    }
    let Some(first) = first_solution(board.clone(), None) else {
        return Solutions::None;
    };
    match other_solution(board, &first) {
        Some(second) => Solutions::Multiple(first, second),
        None => Solutions::Unique(first),
    }
}

/// Finds a solution that agrees with `board` but differs from `known`,
/// which must be a solution consistent with `board`.
///
/// Goes through the unsolved cells one at a time and looks for a solution
/// where that cell differs from `known`. If there is none, every solution
/// has the cell as in `known`, so it is fixed before moving on, and each
/// later search has less to explore. Searches prefer `known`'s values,
/// because other solutions usually differ from it in only a few cells.
fn other_solution(mut board: Board<'_>, known: &Answer) -> Option<Answer> {
    let cols = board.cols();
    let known_value = |index: usize| {
        if known.is_shaded(index / cols, index % cols) {
            SHADED
        } else {
            UNSHADED
        }
    };
    for index in 0..board.cell_count() {
        if board.candidates(index) != BOTH {
            continue;
        }
        let value = known_value(index);
        let mut trial = board.clone();
        if trial.assume(index, BOTH & !value, solve_full).is_ok()
            && let Some(other) = first_solution(trial, Some(known))
        {
            return Some(other);
        }
        // A contradiction here can only come from an exhausted budget, which
        // the caller reports.
        if board.assume(index, value, solve_full).is_err() {
            return None;
        }
    }
    None
}

/// Finds any solution that agrees with `board`, preferring `prefer`'s values
/// when guessing.
fn first_solution(board: Board<'_>, prefer: Option<&Answer>) -> Option<Answer> {
    let mut found = None;
    let _ = search(board, prefer, &mut |answer| {
        found = Some(answer);
        ControlFlow::Break(())
    });
    found
}

/// Visits every solution reachable from `board`, which must have no line
/// deductions left to make. Each level of recursion solves at least one more
/// cell, so the depth is at most the number of cells (G7).
fn search(
    board: Board<'_>,
    prefer: Option<&Answer>,
    visit: &mut impl FnMut(Answer) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let Some(index) = board.branch_cell() else {
        let answer = board.to_answer();
        debug_assert!(board.puzzle().is_solved_by(&answer) || board.is_contradictory());
        return visit(answer);
    };
    let cols = board.cols();
    let first = match prefer {
        Some(answer) if !answer.is_shaded(index / cols, index % cols) => UNSHADED,
        _ => SHADED,
    };
    for value in [first, BOTH & !first] {
        let mut next = board.clone();
        if next.assume(index, value, solve_full).is_ok() {
            search(next, prefer, visit)?;
        }
    }
    ControlFlow::Continue(())
}
