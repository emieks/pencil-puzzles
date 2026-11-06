//! Answers: a player's digits, complete or partial.

use std::fmt;
use std::str::FromStr;

use pencil_puzzles_core::ParseError;

use crate::{MAX_SIZE, Shape};

/// The mark on one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    /// A block, with or without sums, written `#`.
    Block,
    /// A white cell not marked yet, written `.`.
    Unmarked,
    /// A white cell left empty on purpose, written `0`. Finishing an answer
    /// turns unmarked cells into empty ones (P4).
    Empty,
    /// A digit from 1 to 9.
    Digit(u8),
}

impl Mark {
    /// The character for this mark.
    pub fn symbol(self) -> char {
        match self {
            Self::Block => '#',
            Self::Unmarked => '.',
            Self::Empty => '0',
            Self::Digit(d) => char::from(b'0' + d),
        }
    }

    /// The mark for a character, if it is one.
    pub fn from_symbol(c: char) -> Option<Self> {
        match c {
            '#' => Some(Self::Block),
            '.' => Some(Self::Unmarked),
            '0' => Some(Self::Empty),
            '1'..='9' => Some(Self::Digit(c as u8 - b'0')),
            _ => None,
        }
    }
}

/// A player's digits on a grid, complete or partial (P4). A solution is a
/// complete answer that satisfies every rule.
///
/// The text form has one line per row and one character per cell: `#` for
/// a block, `.` for an unmarked white cell, `0` for one left empty and a
/// digit for one with a digit.
///
/// ```
/// use pencil_puzzles_kakuro::{Answer, Mark};
///
/// let answer: Answer = "###\n#1.\n#0.".parse().unwrap();
/// assert_eq!(answer.get(1, 1), Mark::Digit(1));
/// assert_eq!(answer.get(1, 2), Mark::Unmarked);
/// assert!(!answer.is_complete());
/// assert_eq!(answer.finished().get(1, 2), Mark::Empty);
/// assert_eq!(answer.to_string(), "###\n#1.\n#0.");
/// ```
///
/// With the `serde` feature, an answer is a list of these lines.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "Vec<String>"))]
pub struct Answer {
    shape: Shape,
    marks: Vec<Mark>,
}

impl Answer {
    /// An answer with every cell a block.
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            marks: vec![Mark::Block; shape.cells()],
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
        let width = rows[0].as_ref().chars().take(MAX_SIZE + 1).count();
        let shape = Shape::new(rows.len(), width).map_err(|e| e.or_at_line(1))?;
        let mut marks = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let row = row.as_ref();
            if row.chars().take(MAX_SIZE + 1).count() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    format!("row {} has a different length from the first row", r + 1),
                ));
            }
            for c in row.chars() {
                marks.push(Mark::from_symbol(c).ok_or_else(|| {
                    ParseError::at_line(
                        r + 1,
                        format!(
                            "{c:?} in row {} is not a mark; use `#`, `.`, `0` or a digit",
                            r + 1
                        ),
                    )
                })?);
            }
        }
        Ok(Self { shape, marks })
    }

    /// The answer's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// The mark at `row`, `col` (both counted from 0).
    ///
    /// # Panics
    ///
    /// Panics if the cell is outside the grid.
    pub fn get(&self, row: usize, col: usize) -> Mark {
        self.marks[self.index(row, col)]
    }

    /// Sets the mark at `row`, `col` (both counted from 0).
    ///
    /// # Panics
    ///
    /// Panics if the cell is outside the grid or the digit is not 1 to 9.
    pub fn set(&mut self, row: usize, col: usize, mark: Mark) {
        if let Mark::Digit(d) = mark {
            assert!((1..=9).contains(&d), "{d} is not a digit from 1 to 9");
        }
        let index = self.index(row, col);
        self.marks[index] = mark;
    }

    /// The marks in row-major order.
    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }

    /// Whether every cell is a block or has a digit.
    pub fn is_complete(&self) -> bool {
        self.marks
            .iter()
            .all(|m| matches!(m, Mark::Block | Mark::Digit(_)))
    }

    /// The answer as checked when the player says they have finished: every
    /// unmarked cell is left empty (P4).
    pub fn finished(&self) -> Self {
        let mut finished = self.clone();
        for mark in &mut finished.marks {
            if *mark == Mark::Unmarked {
                *mark = Mark::Empty;
            }
        }
        finished
    }

    /// The rows, as written in text.
    pub fn rows(&self) -> Vec<String> {
        self.marks
            .chunks(self.shape.cols())
            .map(|row| row.iter().map(|m| m.symbol()).collect())
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
        let rows = bounded_vec::<D, BoundedText<{ 4 * MAX_SIZE }>, { MAX_SIZE }>(d)?;
        let rows: Vec<_> = rows.into_iter().map(|r| r.0).collect();
        Self::from_rows(&rows).map_err(serde::de::Error::custom)
    }
}
