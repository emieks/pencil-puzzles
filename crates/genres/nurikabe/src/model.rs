//! Validated numbered grids and strict shared records.
use crate::{Answer, MAX_SIZE, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, id,
    text::{self, Document, Writer},
};
use std::{fmt, str::FromStr};
/// A rectangular grid of clues. Zero means no clue.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    clues: Vec<u32>,
}
impl Puzzle {
    /// Validate dimensions and clue ranges, without solving contradictory clues.
    pub fn new(shape: Shape, clues: Vec<u32>) -> Result<Self, ParseError> {
        if clues.len() != shape.cells() || clues.iter().any(|&n| n as usize > shape.cells()) {
            return Err(ParseError::new(
                "clues must match the grid and be between 1 and its cell count",
            ));
        }
        Ok(Self { shape, clues })
    }
    /// Read compact rows, or single-space-separated tokens for multi-digit clues.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(1, "Nurikabe needs 1 to 8 rows"));
        }
        let spaced = rows.iter().any(|r| r.as_ref().contains(' '));
        let split = |s: &str| -> Vec<String> {
            if spaced {
                s.split(' ').take(MAX_SIZE + 1).map(str::to_owned).collect()
            } else {
                s.chars()
                    .take(MAX_SIZE + 1)
                    .map(|c| c.to_string())
                    .collect()
            }
        };
        let first = split(rows[0].as_ref());
        let shape = Shape::new(rows.len(), first.len()).map_err(|e| e.or_at_line(1))?;
        let mut clues = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let tokens = split(row.as_ref());
            if tokens.len() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    "each grid row must have the same number of cells",
                ));
            }
            for token in tokens {
                clues.push(number(&token, shape.cells()).map_err(|e| e.or_at_line(r + 1))?);
            }
        }
        Self::new(shape, clues)
    }
    /// Shape of this grid.
    pub fn shape(&self) -> Shape {
        self.shape
    }
    /// Clues in row-major order, zero for unnumbered cells.
    pub fn clues(&self) -> &[u32] {
        &self.clues
    }
    /// Canonical clue rows.
    pub fn rows(&self) -> Vec<String> {
        let separator = if self.clues.iter().any(|&n| n > 9) {
            " "
        } else {
            ""
        };
        self.clues
            .chunks(self.shape.cols())
            .map(|row| {
                row.iter()
                    .map(|n| if *n == 0 { ".".into() } else { n.to_string() })
                    .collect::<Vec<_>>()
                    .join(separator)
            })
            .collect()
    }
    /// Stable URL-safe puzzle identifier.
    pub fn id(&self) -> String {
        id::join(
            "nurikabe",
            &self.shape.to_string(),
            &self
                .clues
                .iter()
                .map(|n| if *n == 0 { ".".into() } else { n.to_string() })
                .collect::<Vec<_>>()
                .join(","),
        )
    }
    /// Decode a bounded identifier, reporting the invalid character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "nurikabe" {
            return Err(ParseError::at_char(1, "expected a nurikabe ID"));
        }
        let parsed: Shape = shape.parse().map_err(|e: ParseError| {
            ParseError::at_char(
                10 + match e.position() {
                    Position::Char(n) => n - 1,
                    _ => 0,
                },
                e.message(),
            )
        })?;
        let start = 11 + shape.len();
        let mut clues = Vec::with_capacity(parsed.cells());
        let mut offset = start;
        for token in desc.split(',') {
            if clues.len() == parsed.cells() {
                return Err(ParseError::at_char(offset, "too many clue cells"));
            }
            clues.push(
                number(token, parsed.cells())
                    .map_err(|e| ParseError::at_char(offset, e.message()))?,
            );
            offset += token.len() + 1;
        }
        if clues.len() != parsed.cells() {
            return Err(ParseError::at_char(
                start + desc.len(),
                "not enough clue cells",
            ));
        }
        Self::new(parsed, clues)
    }
    /// Whether a complete answer obeys every rule.
    pub fn is_solved_by(&self, a: &Answer) -> bool {
        a.is_complete()
            && crate::check(self, a, &pencil_puzzles_core::Budget::unlimited())
                .is_ok_and(|v| v.is_empty())
    }
    /// Draw numbered cells and optional shading for a terminal.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| a.shape() != self.shape) {
            return "the answer has the wrong shape".into();
        }
        (0..self.shape.rows())
            .map(|r| {
                (0..self.shape.cols())
                    .map(|c| {
                        let n = self.clues[r * self.shape.cols() + c];
                        if n > 0 {
                            n.to_string()
                        } else {
                            answer.map_or('.', |a| a.get(r, c).to_char()).to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
fn number(s: &str, max: usize) -> Result<u32, ParseError> {
    if s == "." {
        return Ok(0);
    }
    if s.is_empty() || s.starts_with('0') || s.len() > 2 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseError::new("use . or a positive clue number"));
    }
    let n = s
        .parse::<u32>()
        .map_err(|_| ParseError::new("the clue is too large"))?;
    if n as usize > max {
        return Err(ParseError::new("the clue exceeds the grid's cell count"));
    }
    Ok(n)
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
/// Parsed puzzle and preserved shared metadata.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;
/// Read Nurikabe documents with preserved metadata (F6-F8).
pub fn parse_records(source: &str) -> Result<Vec<Record>, ParseError> {
    let docs = text::parse_with_limits(source, text::GridLimits::new(8, 24))?;
    if docs.is_empty() {
        return Err(ParseError::at_line(1, "there is no puzzle"));
    }
    if docs.len() > 1 && docs.iter().any(|d| d.genre().is_none()) {
        return Err(ParseError::at_line(1, "each puzzle needs a genre line"));
    }
    docs.into_iter().map(read_document).collect()
}
pub(crate) fn read_document(mut doc: Document) -> Result<Record, ParseError> {
    let start = doc.start_line();
    if doc.genre().is_some_and(|g| g != "nurikabe") {
        return Err(ParseError::at_line(start, "expected a nurikabe puzzle"));
    }
    let params_line = doc.line_of("params");
    let mut shared = doc.take_shared()?;
    let grid = doc
        .take_grid("grid")?
        .ok_or_else(|| ParseError::at_line(start, "missing grid block"))?;
    let p = Puzzle::from_rows(&grid.rows).map_err(|e| {
        ParseError::at_line(
            grid.line
                + match e.position() {
                    Position::Line(n) => n,
                    _ => 1,
                },
            e.message(),
        )
    })?;
    if let Some(params) = &shared.params {
        params
            .parse::<Params>()
            .map_err(|e| ParseError::at_line(params_line.unwrap_or(start), e.message()))?;
    }
    let solution = shared
        .solution
        .take()
        .map(|block| {
            let a = Answer::from_rows(&block.rows).map_err(|e| {
                ParseError::at_line(
                    block.line
                        + match e.position() {
                            Position::Line(n) => n,
                            _ => 1,
                        },
                    e.message(),
                )
            })?;
            if a.shape() != p.shape() || !a.is_complete() {
                return Err(ParseError::at_line(
                    block.line,
                    "the solution must mark every cell",
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
/// Write a canonical Nurikabe record.
pub fn write_record(record: &Record) -> String {
    let mut writer = Writer::new("nurikabe");
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
