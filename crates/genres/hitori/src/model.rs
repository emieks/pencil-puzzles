//! Validated number grids and strict shared records.
use crate::{Answer, MAX_SIZE, Mark, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, id,
    text::{self, Document, Writer},
};
use std::{fmt, str::FromStr};

/// The genre's text limits: rows, and bytes in a row of spaced 2-digit
/// tokens.
pub(crate) const LIMITS: text::GridLimits = text::GridLimits::new(MAX_SIZE, 3 * MAX_SIZE);

/// A rectangular grid with a number in every cell, from 1 to the larger of
/// the grid's two sizes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    numbers: Vec<u32>,
}

/// Split a row into tokens: single characters, or space-separated tokens
/// when any row of the grid has a space. At most `MAX_SIZE + 1` tokens are
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

impl Puzzle {
    /// A puzzle from row-major numbers, each from 1 to the larger of the
    /// shape's two sizes.
    pub fn new(shape: Shape, numbers: Vec<u32>) -> Result<Self, ParseError> {
        if numbers.len() != shape.cells() || numbers.iter().any(|&n| n == 0 || n > shape.largest())
        {
            return Err(ParseError::new(format!(
                "a {shape} grid needs a number from 1 to {} in every cell",
                shape.largest()
            )));
        }
        Ok(Self { shape, numbers })
    }

    /// Read rows of one digit per cell, or of space-separated numbers when a
    /// number has two digits.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("Hitori needs 1 to {MAX_SIZE} rows"),
            ));
        }
        let spaced = rows.iter().any(|r| r.as_ref().contains(' '));
        let first = tokens(rows[0].as_ref(), spaced);
        let shape = Shape::new(rows.len(), first.len()).map_err(|e| e.or_at_line(1))?;
        let mut numbers = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let cells = tokens(row.as_ref(), spaced);
            if cells.len() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    "each grid row must have the same number of cells",
                ));
            }
            for t in cells {
                numbers.push(number(&t, shape.largest()).map_err(|e| e.or_at_line(r + 1))?);
            }
        }
        Self::new(shape, numbers)
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// Numbers in row-major order.
    pub fn numbers(&self) -> &[u32] {
        &self.numbers
    }

    /// The number rows, as written in text.
    pub fn rows(&self) -> Vec<String> {
        let separator = if self.numbers.iter().any(|&n| n > 9) {
            " "
        } else {
            ""
        };
        self.numbers
            .chunks(self.shape.cols())
            .map(|row| {
                row.iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(separator)
            })
            .collect()
    }

    /// The puzzle's ID, such as `hitori:2x2:1221`.
    pub fn id(&self) -> String {
        id::join("hitori", &self.shape.to_string(), &describe(&self.numbers))
    }

    /// Read a puzzle ID, reporting the first wrong character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "hitori" {
            return Err(ParseError::at_char(1, "expected a hitori ID"));
        }
        let parsed: Shape = shape.parse().map_err(|e: ParseError| {
            ParseError::at_char(
                genre.len()
                    + 1
                    + match e.position() {
                        Position::Char(n) => n,
                        _ => 1,
                    },
                e.message(),
            )
        })?;
        let start = genre.len() + shape.len() + 3;
        let numbers = read_description(desc, parsed)
            .map_err(|(at, message)| ParseError::at_char(start + at, message))?;
        Self::new(parsed, numbers)
    }

    /// Whether a complete answer obeys every rule.
    pub fn is_solved_by(&self, a: &Answer) -> bool {
        a.is_complete()
            && crate::check(self, a, &pencil_puzzles_core::Budget::unlimited())
                .is_ok_and(|v| v.is_empty())
    }

    /// Draw the numbers for a terminal, with shaded cells as `#`.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| a.shape() != self.shape) {
            return "the answer has the wrong shape".into();
        }
        let cols = self.shape.cols();
        let width = if self.numbers.iter().any(|&n| n > 9) {
            2
        } else {
            1
        };
        (0..self.shape.rows())
            .map(|r| {
                (0..cols)
                    .map(|c| {
                        let shaded = answer.is_some_and(|a| a.get(r, c) == Mark::Shaded);
                        let token = if shaded {
                            "#".to_string()
                        } else {
                            self.numbers[r * cols + c].to_string()
                        };
                        format!("{token:>width$}")
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn number(s: &str, max: u32) -> Result<u32, ParseError> {
    if s.is_empty() || s.starts_with('0') || s.len() > 2 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseError::new(format!(
            "{s:?} is not a number; use a number from 1 to {max}"
        )));
    }
    let n: u32 = s
        .parse()
        .map_err(|_| ParseError::new("the number is too large"))?;
    if n > max {
        return Err(ParseError::new(format!(
            "{n} is larger than the grid allows ({max})"
        )));
    }
    Ok(n)
}

/// The ID description: one character per cell, row-major, `1`-`9` for 1 to
/// 9 and `a`-`z` for 10 to 35.
fn describe(numbers: &[u32]) -> String {
    numbers
        .iter()
        .map(|&n| char::from_digit(n, 36).expect("numbers are below 36"))
        .collect()
}

/// Read an ID description, giving the 0-based position of an error.
fn read_description(desc: &str, shape: Shape) -> Result<Vec<u32>, (usize, String)> {
    let mut numbers = Vec::with_capacity(shape.cells());
    for (at, b) in desc.bytes().enumerate() {
        if numbers.len() == shape.cells() {
            return Err((at, "the description has too many cells".into()));
        }
        let n = match b {
            b'1'..=b'9' | b'a'..=b'z' => char::from(b).to_digit(36).expect("base 36 digit"),
            _ => {
                return Err((at, "use 1-9, or a-z for 10 to 35".into()));
            }
        };
        if n > shape.largest() {
            return Err((
                at,
                format!("{n} is larger than the grid allows ({})", shape.largest()),
            ));
        }
        numbers.push(n);
    }
    if numbers.len() != shape.cells() {
        return Err((desc.len(), "the description has too few cells".into()));
    }
    Ok(numbers)
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PuzzleData {
    #[serde(deserialize_with = "read_rows")]
    grid: Vec<String>,
}

#[cfg(feature = "serde")]
fn read_rows<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    use pencil_puzzles_core::serde_helpers::{BoundedText, bounded_vec};
    bounded_vec::<D, BoundedText<{ 3 * MAX_SIZE }>, MAX_SIZE>(d)
        .map(|v| v.into_iter().map(|s| s.0).collect())
}

#[cfg(feature = "serde")]
impl From<Puzzle> for PuzzleData {
    fn from(p: Puzzle) -> Self {
        Self { grid: p.rows() }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<PuzzleData> for Puzzle {
    type Error = ParseError;
    fn try_from(d: PuzzleData) -> Result<Self, ParseError> {
        Self::from_rows(&d.grid)
    }
}

/// A Hitori puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

/// Read Hitori documents, keeping their shared keys (F6-F8).
pub fn parse_records(source: &str) -> Result<Vec<Record>, ParseError> {
    let docs = text::parse_with_limits(source, LIMITS)?;
    if docs.is_empty() {
        return Err(ParseError::at_line(1, "there is no puzzle"));
    }
    if docs.len() > 1 && docs.iter().any(|d| d.genre().is_none()) {
        return Err(ParseError::at_line(1, "each puzzle needs a genre line"));
    }
    docs.into_iter().map(read_document).collect()
}

fn block_line(line: usize, e: &ParseError) -> usize {
    line + match e.position() {
        Position::Line(n) => n,
        _ => 1,
    }
}

pub(crate) fn read_document(mut doc: Document) -> Result<Record, ParseError> {
    let start = doc.start_line();
    if doc.genre().is_some_and(|g| g != "hitori") {
        return Err(ParseError::at_line(start, "expected a hitori puzzle"));
    }
    let params_line = doc.line_of("params");
    let mut shared = doc.take_shared()?;
    let grid = doc
        .take_grid("grid")?
        .ok_or_else(|| ParseError::at_line(start, "missing grid block"))?;
    let p = Puzzle::from_rows(&grid.rows)
        .map_err(|e| ParseError::at_line(block_line(grid.line, &e), e.message()))?;
    if let Some(params) = &shared.params {
        params
            .parse::<Params>()
            .map_err(|e| ParseError::at_line(params_line.unwrap_or(start), e.message()))?;
    }
    let solution = shared
        .solution
        .take()
        .map(|block| {
            let a = Answer::from_rows(&block.rows)
                .map_err(|e| ParseError::at_line(block_line(block.line, &e), e.message()))?;
            if a.shape() != p.shape() || !a.is_complete() {
                return Err(ParseError::at_line(
                    block.line,
                    "the solution must mark every cell `#` or `-`",
                ));
            }
            Ok(a)
        })
        .transpose()?;
    doc.finish()?;
    let mut r = Record::new(p);
    r.solution = solution;
    r.shared = shared;
    Ok(r)
}

/// Write a record in canonical text.
pub fn write_record(record: &Record) -> String {
    let mut writer = Writer::new("hitori");
    writer.shared_info(&record.shared);
    writer.grid("grid", record.puzzle.rows());
    let mut shared = record.shared.clone();
    shared.solution = record.solution.as_ref().map(|a| text::GridBlock {
        rows: a.rows(),
        line: 0,
    });
    writer.shared_record(&shared);
    writer.finish()
}

impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&write_record(&Record::new(self.clone())))
    }
}

impl FromStr for Puzzle {
    type Err = ParseError;
    fn from_str(source: &str) -> Result<Self, ParseError> {
        let mut r = parse_records(source)?;
        if r.len() != 1 {
            return Err(ParseError::at_line(1, "expected one puzzle"));
        }
        Ok(r.remove(0).puzzle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptions_round_trip_and_point_at_errors() {
        let shape: Shape = "1x12".parse().unwrap();
        let numbers: Vec<u32> = (1..=12).collect();
        let d = describe(&numbers);
        assert_eq!(d, "123456789abc");
        assert_eq!(read_description(&d, shape).unwrap(), numbers);
        let small: Shape = "2x2".parse().unwrap();
        assert_eq!(read_description("12x1", small).unwrap_err().0, 2);
        assert_eq!(read_description("1231", small).unwrap_err().0, 2);
        assert_eq!(read_description("121", small).unwrap_err().0, 3);
        assert_eq!(read_description("12121", small).unwrap_err().0, 4);
        assert_eq!(read_description("0121", small).unwrap_err().0, 0);
        assert_eq!(read_description("A121", small).unwrap_err().0, 0);
    }
}
