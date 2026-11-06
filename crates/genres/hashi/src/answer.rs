//! Answers: the bridges a player has drawn, complete or partial.

use std::fmt;
use std::str::FromStr;

use pencil_puzzles_core::ParseError;

use crate::board::Layout;
use crate::{MAX_SIZE, Puzzle, Shape};

/// What one cell of an answer shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    /// An island and its number.
    Island(u8),
    /// Bridges passing through the cell: horizontal and vertical, each 0 to
    /// 2. Both are 0 for a cell with no bridge. Both are positive only where
    /// bridges cross, which breaks R2.
    Bridges {
        /// Horizontal bridges through the cell.
        across: u8,
        /// Vertical bridges through the cell.
        down: u8,
    },
}

impl Cell {
    const EMPTY: Self = Self::Bridges { across: 0, down: 0 };

    /// The text for this cell.
    pub fn token(self) -> String {
        match self {
            Self::Island(n) => n.to_string(),
            Self::Bridges { across, down } => {
                let h = ["", "-", "="][usize::from(across.min(2))];
                let v = ["", "|", "H"][usize::from(down.min(2))];
                if h.is_empty() && v.is_empty() {
                    ".".into()
                } else {
                    format!("{h}{v}")
                }
            }
        }
    }

    /// The cell for a token, if it is one.
    pub fn from_token(s: &str) -> Option<Self> {
        let mut chars = s.chars();
        let first = chars.next()?;
        if let Some(n) = first.to_digit(10)
            && chars.as_str().is_empty()
        {
            return (1..=8).contains(&n).then_some(Self::Island(n as u8));
        }
        let across = |c| match c {
            '-' => Some(1),
            '=' => Some(2),
            _ => None,
        };
        let down = |c| match c {
            '|' => Some(1),
            'H' => Some(2),
            _ => None,
        };
        let (h, v) = match (first, chars.next(), chars.next()) {
            ('.', None, _) => (0, 0),
            (c, None, _) => match (across(c), down(c)) {
                (Some(h), _) => (h, 0),
                (_, Some(v)) => (0, v),
                _ => return None,
            },
            (a, Some(b), None) => (across(a)?, down(b)?),
            _ => return None,
        };
        Some(Self::Bridges { across: h, down: v })
    }
}

/// Split a row into tokens: single characters, or space-separated tokens
/// when any row of its grid has a space. At most `MAX_SIZE + 1` tokens are
/// kept, so that oversized rows fail before allocating more.
fn tokens(row: &str, spaced: bool) -> Vec<String> {
    if spaced {
        row.split(' ')
            .take(MAX_SIZE + 1)
            .map(str::to_owned)
            .collect()
    } else {
        row.chars().take(MAX_SIZE + 1).map(String::from).collect()
    }
}

/// The bridges a player has drawn, complete or partial (P4). A solution is a
/// complete answer that satisfies every rule.
///
/// The text form has one line per row: an island's number, `.` for a cell
/// with no bridge, `-` and `=` for one and two horizontal bridges, and `|`
/// and `H` for one and two vertical bridges. Every cell a bridge passes
/// through shows it. Where bridges cross, which breaks R2, the cell shows
/// both, horizontal first (`-|`), and every cell is written as a
/// space-separated token.
///
/// A bridge that is not drawn is undecided until the player says they have
/// finished ([`Answer::finished`]); then it counts as no bridge. The text
/// form is the same either way.
///
/// ```
/// use pencil_puzzles_hashi::{Answer, Cell};
///
/// let answer: Answer = "2=2\n...".parse().unwrap();
/// assert_eq!(answer.get(0, 1), Cell::Bridges { across: 2, down: 0 });
/// assert!(!answer.is_finished());
/// assert!(answer.finished().is_finished());
/// assert_eq!(answer.to_string(), "2=2\n...");
/// ```
///
/// With the `serde` feature, an answer is a list of these lines.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "Vec<String>"))]
pub struct Answer {
    shape: Shape,
    cells: Vec<Cell>,
    finished: bool,
}

impl Answer {
    /// An answer with no bridges and no islands.
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            cells: vec![Cell::EMPTY; shape.cells()],
            finished: false,
        }
    }

    /// Reads an answer from its rows.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("an answer needs 1 to {MAX_SIZE} rows"),
            ));
        }
        let spaced = rows.iter().any(|r| r.as_ref().contains(' '));
        let first = tokens(rows[0].as_ref(), spaced);
        let shape = Shape::new(rows.len(), first.len()).map_err(|e| e.or_at_line(1))?;
        let mut cells = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let row = tokens(row.as_ref(), spaced);
            if row.len() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    format!("row {} has a different length from the first row", r + 1),
                ));
            }
            for t in row {
                cells.push(Cell::from_token(&t).ok_or_else(|| {
                    ParseError::at_line(
                        r + 1,
                        format!(
                            "{t:?} in row {} is not a cell; use `.`, an island number, `-`, `=`, `|` or `H`",
                            r + 1
                        ),
                    )
                })?);
            }
        }
        Ok(Self {
            shape,
            cells,
            finished: false,
        })
    }

    /// The answer's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// The cell at `row`, `col` (both counted from 0).
    ///
    /// # Panics
    ///
    /// Panics if the cell is outside the grid.
    pub fn get(&self, row: usize, col: usize) -> Cell {
        self.cells[self.index(row, col)]
    }

    /// Sets the cell at `row`, `col` (both counted from 0).
    ///
    /// # Panics
    ///
    /// Panics if the cell is outside the grid, or a number is not from 1
    /// to 8, or a bridge count is above 2.
    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        match cell {
            Cell::Island(n) => assert!((1..=8).contains(&n), "{n} is not an island number"),
            Cell::Bridges { across, down } => {
                assert!(across <= 2 && down <= 2, "at most two bridges");
            }
        }
        let index = self.index(row, col);
        self.cells[index] = cell;
    }

    /// The text for the cell at `row`, `col`.
    pub fn token(&self, row: usize, col: usize) -> String {
        self.get(row, col).token()
    }

    /// Whether the player has finished, so that undrawn bridges count as no
    /// bridge.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// The answer as checked when the player says they have finished: every
    /// bridge not drawn is left out (P4).
    pub fn finished(&self) -> Self {
        Self {
            finished: true,
            ..self.clone()
        }
    }

    /// The rows, as written in text.
    pub fn rows(&self) -> Vec<String> {
        let tokens: Vec<String> = self.cells.iter().map(|c| c.token()).collect();
        let spaced = tokens.iter().any(|t| t.len() > 1);
        tokens
            .chunks(self.shape.cols())
            .map(|row| row.join(if spaced { " " } else { "" }))
            .collect()
    }

    fn index(&self, row: usize, col: usize) -> usize {
        assert!(
            row < self.shape.rows() && col < self.shape.cols(),
            "cell ({row}, {col}) is outside a {} grid",
            self.shape
        );
        row * self.shape.cols() + col
    }

    /// Check the answer against a puzzle's islands and write in their
    /// numbers. Every cell a possible bridge passes through must show the
    /// same bridges, and bridges appear only between islands.
    pub fn for_puzzle(&self, p: &Puzzle) -> Result<Self, ParseError> {
        if self.shape != p.shape() {
            return Err(ParseError::new("the answer has the wrong shape"));
        }
        let layout = Layout::new(p);
        let counts = self.counts(&layout)?;
        Ok(Self::from_counts(p, &layout, &counts, self.finished))
    }

    /// Bridges on each slot, checking that the cells agree with the layout.
    pub(crate) fn counts(&self, layout: &Layout) -> Result<Vec<u8>, ParseError> {
        let cols = self.shape.cols();
        let at = |i: usize| format!("row {}, column {}", i / cols + 1, i % cols + 1);
        for (i, &cell) in self.cells.iter().enumerate() {
            let island = layout.island_at[i];
            match (cell, island) {
                (Cell::Island(n), Some(k)) if n != layout.numbers[k] => {
                    return Err(ParseError::new(format!(
                        "the island in {} has the number {}",
                        at(i),
                        layout.numbers[k]
                    )));
                }
                (Cell::Island(_), None) => {
                    return Err(ParseError::new(format!("there is no island in {}", at(i))));
                }
                (Cell::Bridges { across, down }, Some(_)) if across + down > 0 => {
                    return Err(ParseError::new(format!(
                        "a bridge can't pass through the island in {}",
                        at(i)
                    )));
                }
                (Cell::Bridges { across, down }, None) => {
                    let [h, v] = layout.through[i];
                    if (across > 0 && h.is_none()) || (down > 0 && v.is_none()) {
                        return Err(ParseError::new(format!(
                            "the bridge in {} doesn't join two islands",
                            at(i)
                        )));
                    }
                }
                _ => {}
            }
        }
        layout
            .slots
            .iter()
            .map(|slot| {
                let count = |i: usize| match self.cells[i] {
                    Cell::Bridges { across, down } => {
                        if slot.horizontal {
                            across
                        } else {
                            down
                        }
                    }
                    Cell::Island(_) => 0,
                };
                let first = count(slot.interior[0]);
                if let Some(&bad) = slot.interior.iter().find(|&&i| count(i) != first) {
                    return Err(ParseError::new(format!(
                        "the bridge through {} is broken off",
                        at(bad)
                    )));
                }
                Ok(first)
            })
            .collect()
    }

    /// The answer showing `counts` bridges on each slot.
    pub(crate) fn from_counts(p: &Puzzle, layout: &Layout, counts: &[u8], finished: bool) -> Self {
        let mut a = Self::new(p.shape());
        a.finished = finished;
        for (k, &cell) in layout.cells.iter().enumerate() {
            a.cells[cell] = Cell::Island(layout.numbers[k]);
        }
        for (slot, &n) in layout.slots.iter().zip(counts) {
            for &i in &slot.interior {
                if let Cell::Bridges { across, down } = &mut a.cells[i] {
                    if slot.horizontal {
                        *across = n;
                    } else {
                        *down = n;
                    }
                }
            }
        }
        a
    }
}

impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.rows().join("\n"))
    }
}

impl FromStr for Answer {
    type Err = ParseError;

    /// Reads one row per line, ignoring blank lines and spaces at the start
    /// and end of lines.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rows: Vec<(usize, &str)> = s
            .lines()
            .enumerate()
            .map(|(i, line)| (i + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty())
            .take(MAX_SIZE + 1)
            .collect();
        let values: Vec<_> = rows.iter().map(|(_, line)| *line).collect();
        Self::from_rows(&values).map_err(|error| {
            let line = match error.position() {
                pencil_puzzles_core::Position::Line(n) => {
                    rows.get(n - 1).map_or(n, |(line, _)| *line)
                }
                _ => 1,
            };
            ParseError::at_line(line, error.message())
        })
    }
}

impl From<Answer> for Vec<String> {
    fn from(answer: Answer) -> Self {
        answer.rows()
    }
}

impl TryFrom<Vec<String>> for Answer {
    type Error = ParseError;

    fn try_from(rows: Vec<String>) -> Result<Self, Self::Error> {
        Self::from_rows(&rows)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Answer {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use pencil_puzzles_core::serde_helpers::{BoundedText, bounded_vec};
        let rows = bounded_vec::<D, BoundedText<{ 3 * MAX_SIZE }>, { MAX_SIZE }>(d)?;
        let rows: Vec<_> = rows.into_iter().map(|r| r.0).collect();
        Self::from_rows(&rows).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_round_trip() {
        for t in [".", "1", "8", "-", "=", "|", "H", "-|", "=H", "-H", "=|"] {
            assert_eq!(Cell::from_token(t).unwrap().token(), t);
        }
        for bad in ["", "0", "9", "x", "|-", "--", "-|-", "12", "h"] {
            assert!(Cell::from_token(bad).is_none(), "{bad:?}");
        }
    }
}
