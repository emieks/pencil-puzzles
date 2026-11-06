//! Answers: a player's numbers, complete or partial.

use std::fmt;
use std::str::FromStr;

use pencil_puzzles_core::ParseError;

use crate::{MAX_SIZE, Shape};

/// The mark on one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mark {
    /// Not marked yet, written `.`.
    Unmarked,
    /// Left empty on purpose, written `0`. Finishing an answer turns
    /// unmarked cells into empty ones (P4).
    Empty,
    /// The size of the cell's region, from 1 to the number of cells.
    Number(u32),
}

impl Mark {
    /// The text for this mark.
    pub fn token(self) -> String {
        match self {
            Self::Unmarked => ".".into(),
            Self::Empty => "0".into(),
            Self::Number(v) => v.to_string(),
        }
    }

    /// The mark for a token, if it is one with a number of at most `max`.
    pub fn from_token(s: &str, max: usize) -> Option<Self> {
        match s {
            "." => Some(Self::Unmarked),
            "0" => Some(Self::Empty),
            _ => {
                if s.is_empty() || s.len() > 3 || s.starts_with('0') {
                    return None;
                }
                if !s.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                let v: u32 = s.parse().ok()?;
                (v as usize <= max).then_some(Self::Number(v))
            }
        }
    }
}

/// Split a row into tokens: single characters, or space-separated tokens
/// when the row (or any row of its grid) has a space. At most `limit + 1`
/// tokens are kept, so that oversized rows fail before allocating more.
pub(crate) fn tokens(row: &str, spaced: bool, limit: usize) -> Vec<String> {
    if spaced {
        row.split(' ').take(limit + 1).map(str::to_owned).collect()
    } else {
        row.chars().take(limit + 1).map(String::from).collect()
    }
}

/// Join tokens into rows, separated by spaces when any token is longer
/// than one character.
pub(crate) fn rows_of(tokens: Vec<String>, cols: usize) -> Vec<String> {
    let spaced = tokens.iter().any(|t| t.len() > 1);
    tokens
        .chunks(cols)
        .map(|row| row.join(if spaced { " " } else { "" }))
        .collect()
}

/// A player's numbers on a grid, complete or partial (P4). A solution is a
/// complete answer that satisfies every rule.
///
/// The text form has one line per row: a number for each numbered cell,
/// `.` for an unmarked cell and `0` for one left empty. When any number has
/// two digits, every cell is written as a space-separated token.
///
/// ```
/// use pencil_puzzles_fillomino::{Answer, Mark};
///
/// let answer: Answer = "22\n.1".parse().unwrap();
/// assert_eq!(answer.get(0, 1), Mark::Number(2));
/// assert_eq!(answer.get(1, 0), Mark::Unmarked);
/// assert!(!answer.is_complete());
/// assert_eq!(answer.finished().get(1, 0), Mark::Empty);
/// assert_eq!(answer.to_string(), "22\n.1");
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
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("an answer needs 1 to {MAX_SIZE} rows"),
            ));
        }
        let spaced = rows.iter().any(|r| r.as_ref().contains(' '));
        let first = tokens(rows[0].as_ref(), spaced, MAX_SIZE);
        let shape = Shape::new(rows.len(), first.len()).map_err(|e| e.or_at_line(1))?;
        let mut marks = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let cells = tokens(row.as_ref(), spaced, MAX_SIZE);
            if cells.len() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    format!("row {} has a different length from the first row", r + 1),
                ));
            }
            for t in cells {
                marks.push(Mark::from_token(&t, shape.cells()).ok_or_else(|| {
                    ParseError::at_line(
                        r + 1,
                        format!(
                            "{t:?} in row {} is not a mark; use `.`, `0` or a number from 1 to {}",
                            r + 1,
                            shape.cells()
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
    /// Panics if the cell is outside the grid, or the number is 0 or larger
    /// than the number of cells.
    pub fn set(&mut self, row: usize, col: usize, mark: Mark) {
        if let Mark::Number(v) = mark {
            assert!(
                v >= 1 && v as usize <= self.shape.cells(),
                "{v} is not a size in a {} grid",
                self.shape
            );
        }
        let index = self.index(row, col);
        self.marks[index] = mark;
    }

    /// Whether every cell has a number.
    pub fn is_complete(&self) -> bool {
        self.marks.iter().all(|m| matches!(m, Mark::Number(_)))
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
        rows_of(
            self.marks.iter().map(|m| m.token()).collect(),
            self.shape.cols(),
        )
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
