//! Answers: a player's marks, complete or partial.

use std::fmt;
use std::str::FromStr;

use pencil_puzzles_core::ParseError;

use crate::Shape;

/// The mark on one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    /// Not marked yet, written `.`.
    Unmarked,
    /// The digit 0, written `0`.
    Zero,
    /// The digit 1, written `1`.
    One,
}

impl Mark {
    /// The character for this mark (F5).
    pub fn to_char(self) -> char {
        match self {
            Self::Unmarked => '.',
            Self::Zero => '0',
            Self::One => '1',
        }
    }

    /// The mark for a character, if it is one.
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '.' => Some(Self::Unmarked),
            '0' => Some(Self::Zero),
            '1' => Some(Self::One),
            _ => None,
        }
    }

    /// The digit, if the cell has one.
    pub fn digit(self) -> Option<u8> {
        match self {
            Self::Unmarked => None,
            Self::Zero => Some(0),
            Self::One => Some(1),
        }
    }

    /// The mark for a digit: 0 is [`Mark::Zero`], anything else
    /// [`Mark::One`].
    pub fn of(digit: u8) -> Self {
        if digit == 0 { Self::Zero } else { Self::One }
    }
}

/// A player's marks on a grid, complete or partial (P4). A solution is a
/// complete answer that satisfies every rule.
///
/// The text form has one line per row, with `0` and `1` for the digits and
/// `.` for not marked:
///
/// ```
/// use pencil_puzzles_binairo::{Answer, Mark};
///
/// let answer: Answer = "10\n.1".parse().unwrap();
/// assert_eq!(answer.get(1, 0), Mark::Unmarked);
/// assert!(!answer.is_complete());
/// assert_eq!(answer.finished().get(1, 0), Mark::Zero);
/// assert_eq!(answer.to_string(), "10\n.1");
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
    /// An answer with no marks.
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            marks: vec![Mark::Unmarked; shape.cells()],
        }
    }

    /// Reads an answer from its rows.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        let cols = rows.first().map_or(0, |row| row.as_ref().chars().count());
        let shape = Shape::new(rows.len(), cols).map_err(|e| e.or_at_line(1))?;
        let mut marks = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let row = row.as_ref();
            if row.chars().count() != cols {
                return Err(ParseError::at_line(
                    r + 1,
                    format!("row {} has a different length from the first row", r + 1),
                ));
            }
            for c in row.chars() {
                marks.push(Mark::from_char(c).ok_or_else(|| {
                    ParseError::at_line(
                        r + 1,
                        format!("{c:?} in row {} is not a mark; use `0`, `1` or `.`", r + 1),
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
    /// Panics if the cell is outside the grid.
    pub fn set(&mut self, row: usize, col: usize, mark: Mark) {
        let index = self.index(row, col);
        self.marks[index] = mark;
    }

    /// Whether every cell is marked.
    pub fn is_complete(&self) -> bool {
        !self.marks.contains(&Mark::Unmarked)
    }

    /// The answer as checked when the player says they have finished: every
    /// unmarked cell counts as 0 (P4).
    pub fn finished(&self) -> Self {
        let mut finished = self.clone();
        for mark in &mut finished.marks {
            if *mark == Mark::Unmarked {
                *mark = Mark::Zero;
            }
        }
        finished
    }

    /// The marks in row-major order.
    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }

    /// The rows, as written in text.
    pub fn rows(&self) -> Vec<String> {
        self.marks
            .chunks(self.shape.cols())
            .map(|row| row.iter().map(|mark| mark.to_char()).collect())
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
            .take(crate::MAX_SIZE + 1)
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
        let rows = bounded_vec::<D, BoundedText<{ crate::MAX_SIZE }>, { crate::MAX_SIZE }>(d)?;
        let rows: Vec<_> = rows.into_iter().map(|r| r.0).collect();
        Self::from_rows(&rows).map_err(serde::de::Error::custom)
    }
}
