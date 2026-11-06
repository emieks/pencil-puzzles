//! Validated clue grids and strict shared records.
use crate::answer::{rows_of, tokens};
use crate::{Answer, MAX_SIZE, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, id,
    text::{self, Document, Writer},
};
use std::{fmt, str::FromStr};

/// The genre's text limits: rows, and bytes in a row of spaced 3-digit
/// tokens.
pub(crate) const LIMITS: text::GridLimits = text::GridLimits::new(MAX_SIZE, 4 * MAX_SIZE);

/// A rectangular grid of clues: region sizes printed in some cells. Zero
/// means no clue.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    clues: Vec<u32>,
}

impl Puzzle {
    /// A puzzle from row-major clues, each 0 (no clue) or from 1 to the
    /// number of cells. Contradictory clues are accepted; such a puzzle has
    /// no solution.
    pub fn new(shape: Shape, clues: Vec<u32>) -> Result<Self, ParseError> {
        if clues.len() != shape.cells() || clues.iter().any(|&n| n as usize > shape.cells()) {
            return Err(ParseError::new(
                "clues must match the grid and be between 1 and its cell count",
            ));
        }
        Ok(Self { shape, clues })
    }

    /// Read rows of one character per cell, or of space-separated tokens
    /// when a clue has more than one digit.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("Fillomino needs 1 to {MAX_SIZE} rows"),
            ));
        }
        let spaced = rows.iter().any(|r| r.as_ref().contains(' '));
        let first = tokens(rows[0].as_ref(), spaced, MAX_SIZE);
        let shape = Shape::new(rows.len(), first.len()).map_err(|e| e.or_at_line(1))?;
        let mut clues = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let cells = tokens(row.as_ref(), spaced, MAX_SIZE);
            if cells.len() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    "each grid row must have the same number of cells",
                ));
            }
            for t in cells {
                clues.push(clue(&t, shape.cells()).map_err(|e| e.or_at_line(r + 1))?);
            }
        }
        Self::new(shape, clues)
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// Clues in row-major order, 0 for cells without one.
    pub fn clues(&self) -> &[u32] {
        &self.clues
    }

    /// The clue rows, as written in text.
    pub fn rows(&self) -> Vec<String> {
        rows_of(
            self.clues
                .iter()
                .map(|&n| if n == 0 { ".".into() } else { n.to_string() })
                .collect(),
            self.shape.cols(),
        )
    }

    /// The puzzle's ID, such as `fillomino:3x3:2b1c3a3`.
    pub fn id(&self) -> String {
        id::join("fillomino", &self.shape.to_string(), &describe(&self.clues))
    }

    /// Read a puzzle ID, reporting the first wrong character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "fillomino" {
            return Err(ParseError::at_char(1, "expected a fillomino ID"));
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
        Self::new(parsed, clues)
    }

    /// Whether a complete answer obeys every rule.
    pub fn is_solved_by(&self, a: &Answer) -> bool {
        a.is_complete()
            && crate::check(self, a, &pencil_puzzles_core::Budget::unlimited())
                .is_ok_and(|v| v.is_empty())
    }

    /// Draw the clues, and optionally an answer's numbers, for a terminal.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| a.shape() != self.shape) {
            return "the answer has the wrong shape".into();
        }
        let cols = self.shape.cols();
        let cells: Vec<String> = (0..self.shape.cells())
            .map(|i| match self.clues[i] {
                0 => answer.map_or(".".into(), |a| a.get(i / cols, i % cols).token()),
                n => n.to_string(),
            })
            .collect();
        let width = cells.iter().map(String::len).max().unwrap_or(1);
        cells
            .chunks(cols)
            .map(|row| {
                row.iter()
                    .map(|t| format!("{t:>width$}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn clue(s: &str, max: usize) -> Result<u32, ParseError> {
    if s == "." {
        return Ok(0);
    }
    if s.is_empty() || s.starts_with('0') || s.len() > 3 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseError::new("use . or a clue from 1 to the cell count"));
    }
    let n: u32 = s
        .parse()
        .map_err(|_| ParseError::new("the clue is too large"))?;
    if n as usize > max {
        return Err(ParseError::new("the clue exceeds the grid's cell count"));
    }
    Ok(n)
}

/// The ID description: row-major, a digit for a clue from 1 to 9, `-`
/// and two digits for a larger clue (`-12`; 100 is `-00`), and a letter
/// for a run of cells without clues (`a` is one, `z` is 26).
fn describe(clues: &[u32]) -> String {
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
        if c < 10 {
            out.push(char::from(b'0' + c as u8));
        } else {
            out.push_str(&format!("-{:02}", c % 100));
        }
    }
    flush(&mut out, &mut blanks);
    out
}

/// Read an ID description, giving the 0-based position of an error.
fn read_description(desc: &str, cells: usize) -> Result<Vec<u32>, (usize, String)> {
    let bytes = desc.as_bytes();
    let mut clues = Vec::with_capacity(cells);
    let mut at = 0;
    while at < bytes.len() {
        let b = bytes[at];
        let (values, width): (Vec<u32>, usize) = match b {
            b'1'..=b'9' => (vec![u32::from(b - b'0')], 1),
            b'a'..=b'z' => (vec![0; usize::from(b - b'a') + 1], 1),
            b'-' => {
                let digits = bytes
                    .get(at + 1..at + 3)
                    .filter(|d| d.iter().all(u8::is_ascii_digit));
                let Some(d) = digits else {
                    return Err((at, "`-` needs two digits after it".into()));
                };
                let n = u32::from(d[0] - b'0') * 10 + u32::from(d[1] - b'0');
                let n = if n == 0 { 100 } else { n };
                if n < 10 {
                    return Err((at, "clues below 10 are written as one digit".into()));
                }
                (vec![n], 3)
            }
            _ => {
                return Err((
                    at,
                    "use a digit, `-` and two digits, or a letter for cells without clues".into(),
                ));
            }
        };
        if clues.len() + values.len() > cells {
            return Err((at, "the description has too many cells".into()));
        }
        if values.iter().any(|&n| n as usize > cells) {
            return Err((at, "the clue exceeds the grid's cell count".into()));
        }
        clues.extend(values);
        at += width;
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
    bounded_vec::<D, BoundedText<{ 4 * MAX_SIZE }>, MAX_SIZE>(d)
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

/// A Fillomino puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

/// Read Fillomino documents, keeping their shared keys (F6-F8).
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
    if doc.genre().is_some_and(|g| g != "fillomino") {
        return Err(ParseError::at_line(start, "expected a fillomino puzzle"));
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
                    "the solution must number every cell",
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
    let mut writer = Writer::new("fillomino");
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
        let mut clues = vec![0; 100];
        clues[0] = 100;
        clues[1] = 12;
        clues[40] = 3;
        let d = describe(&clues);
        assert_eq!(d, "-00-12zl3zzg");
        assert_eq!(read_description(&d, 100).unwrap(), clues);
        assert_eq!(read_description("1x", 2).unwrap_err().0, 1);
        assert_eq!(read_description("-1", 2).unwrap_err().0, 0);
        assert_eq!(read_description("-05", 100).unwrap_err().0, 0);
        assert_eq!(read_description("c", 2).unwrap_err().0, 0);
        assert_eq!(read_description("a", 2).unwrap_err().0, 1);
        assert_eq!(read_description("-12", 9).unwrap_err().0, 0);
    }
}
