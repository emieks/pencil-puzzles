//! Validated island grids and strict shared records.
use crate::{Answer, MAX_SIZE, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, id,
    text::{self, Document, Writer},
};
use std::{fmt, str::FromStr};

/// The genre's text limits: rows, and bytes in a row of spaced two-character
/// answer tokens.
pub(crate) const LIMITS: text::GridLimits = text::GridLimits::new(MAX_SIZE, 3 * MAX_SIZE);

/// The largest island number: two bridges in each of four directions.
pub const MAX_NUMBER: u8 = 8;

/// A rectangular grid of islands, each with a number from 1 to 8. Zero
/// means an empty cell. Islands are never orthogonally adjacent, so every
/// bridge has a cell to be drawn in. Puzzles whose numbers cannot be met
/// are accepted; they have no solution.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    clues: Vec<u8>,
}

impl Puzzle {
    /// A puzzle from row-major clues: 0 for an empty cell, 1 to 8 for an
    /// island.
    pub fn new(shape: Shape, clues: Vec<u8>) -> Result<Self, ParseError> {
        if clues.len() != shape.cells() || clues.iter().any(|&n| n > MAX_NUMBER) {
            return Err(ParseError::new(
                "clues must match the grid and be between 1 and 8",
            ));
        }
        let cols = shape.cols();
        for i in 0..clues.len() {
            let right = i % cols + 1 < cols && clues[i + 1] > 0;
            let down = i + cols < clues.len() && clues[i + cols] > 0;
            if clues[i] > 0 && (right || down) {
                return Err(ParseError::new(format!(
                    "the island in row {}, column {} touches another island; islands need an empty cell between them",
                    i / cols + 1,
                    i % cols + 1
                )));
            }
        }
        Ok(Self { shape, clues })
    }

    /// Read rows of one character per cell: `.` for an empty cell or an
    /// island's number.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("Hashi needs 1 to {MAX_SIZE} rows"),
            ));
        }
        let width = rows[0].as_ref().chars().take(MAX_SIZE + 1).count();
        let shape = Shape::new(rows.len(), width).map_err(|e| e.or_at_line(1))?;
        let mut clues = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let row = row.as_ref();
            if row.chars().take(MAX_SIZE + 1).count() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    "each grid row must have the same number of cells",
                ));
            }
            for c in row.chars() {
                clues.push(match c {
                    '.' => 0,
                    '1'..='8' => c as u8 - b'0',
                    _ => {
                        return Err(ParseError::at_line(
                            r + 1,
                            format!("{c:?} is not a cell; use `.` or an island number from 1 to 8"),
                        ));
                    }
                });
            }
        }
        Self::new(shape, clues).map_err(|e| e.or_at_line(1))
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// Clues in row-major order, 0 for empty cells.
    pub fn clues(&self) -> &[u8] {
        &self.clues
    }

    /// The clue rows, as written in text.
    pub fn rows(&self) -> Vec<String> {
        self.clues
            .chunks(self.shape.cols())
            .map(|row| {
                row.iter()
                    .map(|&n| if n == 0 { '.' } else { char::from(b'0' + n) })
                    .collect()
            })
            .collect()
    }

    /// The puzzle's ID, such as `hashi:3x3:2a2c1a1`.
    pub fn id(&self) -> String {
        id::join("hashi", &self.shape.to_string(), &describe(&self.clues))
    }

    /// Read a puzzle ID, reporting the first wrong character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "hashi" {
            return Err(ParseError::at_char(1, "expected a hashi ID"));
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
        let clues = read_description(desc, parsed.cells())
            .map_err(|(at, message)| ParseError::at_char(start + at, message))?;
        Self::new(parsed, clues).map_err(|e| ParseError::at_char(start, e.message()))
    }

    /// Whether a complete answer obeys every rule.
    pub fn is_solved_by(&self, a: &Answer) -> bool {
        a.shape() == self.shape
            && crate::check(
                self,
                &a.finished(),
                &pencil_puzzles_core::Budget::unlimited(),
            )
            .is_ok_and(|v| v.is_empty())
    }

    /// Draw the islands, and optionally an answer's bridges, for a terminal.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| a.shape() != self.shape) {
            return "the answer has the wrong shape".into();
        }
        let cols = self.shape.cols();
        (0..self.shape.rows())
            .map(|r| {
                (0..cols)
                    .map(|c| match self.clues[r * cols + c] {
                        0 => answer.map_or(".".into(), |a| a.token(r, c)),
                        n => n.to_string(),
                    })
                    .map(|t| format!("{t:<2}"))
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The ID description: row-major, a digit for an island and a letter for a
/// run of empty cells (`a` is one, `z` is 26).
fn describe(clues: &[u8]) -> String {
    let mut out = String::new();
    let mut blanks = 0u32;
    let flush = |out: &mut String, blanks: &mut u32| {
        while *blanks > 0 {
            let run = (*blanks).min(26);
            out.push(char::from(b'a' + run as u8 - 1));
            *blanks -= run;
        }
    };
    for &c in clues {
        if c == 0 {
            blanks += 1;
            continue;
        }
        flush(&mut out, &mut blanks);
        out.push(char::from(b'0' + c));
    }
    flush(&mut out, &mut blanks);
    out
}

/// Read an ID description, giving the 0-based position of an error.
fn read_description(desc: &str, cells: usize) -> Result<Vec<u8>, (usize, String)> {
    let mut clues = Vec::with_capacity(cells);
    for (at, b) in desc.bytes().enumerate() {
        let run = match b {
            b'1'..=b'8' => 1,
            b'a'..=b'z' => usize::from(b - b'a') + 1,
            _ => {
                return Err((
                    at,
                    "use an island number from 1 to 8, or a letter for empty cells".into(),
                ));
            }
        };
        if clues.len() + run > cells {
            return Err((at, "the description has too many cells".into()));
        }
        if b.is_ascii_digit() {
            clues.push(b - b'0');
        } else {
            clues.resize(clues.len() + run, 0);
        }
    }
    if clues.len() != cells {
        return Err((desc.len(), "the description has too few cells".into()));
    }
    Ok(clues)
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

/// A Hashi puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

/// Read Hashi documents, keeping their shared keys (F6-F8).
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
    if doc.genre().is_some_and(|g| g != "hashi") {
        return Err(ParseError::at_line(start, "expected a hashi puzzle"));
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
                .and_then(|a| a.for_puzzle(&p))
                .map_err(|e| ParseError::at_line(block_line(block.line, &e), e.message()))?;
            Ok::<_, ParseError>(a.finished())
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
    let mut writer = Writer::new("hashi");
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
        let mut clues = vec![0; 60];
        clues[0] = 8;
        clues[2] = 3;
        clues[40] = 1;
        let d = describe(&clues);
        assert_eq!(d, "8a3zk1s");
        assert_eq!(read_description(&d, 60).unwrap(), clues);
        assert_eq!(read_description("1x", 2).unwrap_err().0, 1);
        assert_eq!(read_description("9", 2).unwrap_err().0, 0);
        assert_eq!(read_description("c", 2).unwrap_err().0, 0);
        assert_eq!(read_description("a", 2).unwrap_err().0, 1);
    }

    #[test]
    fn adjacent_islands_are_rejected() {
        let shape = "2x2".parse().unwrap();
        assert!(Puzzle::new(shape, vec![1, 1, 0, 0]).is_err());
        assert!(Puzzle::new(shape, vec![1, 0, 1, 0]).is_err());
        assert!(Puzzle::new(shape, vec![1, 0, 0, 1]).is_ok());
        assert!(Puzzle::new(shape, vec![9, 0, 0, 0]).is_err());
    }
}
