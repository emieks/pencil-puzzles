//! Checking answers against the rules (P8).

use pencil_puzzles_core::{ParseError, Place, Violation};

use crate::line::{Cell, solve_full};
use crate::{Answer, Mark, Puzzle};

/// Every broken rule in an answer (P8).
///
/// - **R1**, at a row: its shaded cells don't add up to its clue.
/// - **R2**, at a column: likewise.
///
/// Each violation has the values `total`, the total of the line's shaded
/// cells, and `clue`.
///
/// For a partial answer, a line is only reported if no way of filling in its
/// unmarked cells reaches its clue. To check an answer the player says is
/// finished, check [`Answer::finished`].
///
/// ```
/// use pencil_puzzles_kakurasu::Puzzle;
///
/// let puzzle: Puzzle = "rows: 1 2\ncols: 1 2".parse().unwrap();
/// let answer = "#-\n#-".parse().unwrap();
/// let violations = pencil_puzzles_kakurasu::check(&puzzle, &answer).unwrap();
/// assert_eq!(violations.len(), 3); // Row 2 and both columns.
/// ```
///
/// Fails if the answer is not the same shape as the puzzle.
pub fn check(puzzle: &Puzzle, answer: &Answer) -> Result<Vec<Violation>, ParseError> {
    if answer.shape() != puzzle.shape() {
        return Err(ParseError::new(format!(
            "the answer is {}, but the puzzle is {}",
            answer.shape(),
            puzzle.shape()
        )));
    }
    let mut violations = Vec::new();
    let mut cells = Vec::new();
    let mut check_line =
        |rule: u32, place: Place, marks: &mut dyn Iterator<Item = Mark>, clue: u32| {
            cells.clear();
            let mut total = 0;
            for (value, mark) in (1..).zip(marks) {
                cells.push(match mark {
                    Mark::Unmarked => Cell::Unknown,
                    Mark::Shaded => {
                        total += value;
                        Cell::Shaded
                    }
                    Mark::Unshaded => Cell::Empty,
                });
            }
            if solve_full(&mut cells, clue).is_err() {
                violations.push(Violation::new(
                    rule,
                    place,
                    vec![("total", i64::from(total)), ("clue", i64::from(clue))],
                ));
            }
        };
    let shape = puzzle.shape();
    for row in 0..shape.rows() {
        let mut marks = (0..shape.cols()).map(|col| answer.get(row, col));
        check_line(
            1,
            Place::Row(row as u32),
            &mut marks,
            puzzle.row_clues()[row],
        );
    }
    for col in 0..shape.cols() {
        let mut marks = (0..shape.rows()).map(|row| answer.get(row, col));
        check_line(
            2,
            Place::Col(col as u32),
            &mut marks,
            puzzle.col_clues()[col],
        );
    }
    Ok(violations)
}
