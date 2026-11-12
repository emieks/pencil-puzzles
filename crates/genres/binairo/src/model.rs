//! Validated grids of given digits, their formats, and strict shared records.
use crate::{Answer, MAX_SIZE, Mark, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, id,
    text::{self, Document, Writer},
};
use std::{fmt, str::FromStr};

/// The genre's text limits: rows, and bytes in a row.
pub(crate) const LIMITS: text::GridLimits = text::GridLimits::new(MAX_SIZE, MAX_SIZE);

/// A grid with some cells given as 0 or 1.
///
/// ```
/// use pencil_puzzles_binairo::{Mark, Puzzle};
///
/// let puzzle: Puzzle = "grid:\n1...\n..0.\n....\n.1..".parse().unwrap();
/// assert_eq!(puzzle.given(1, 2), Mark::Zero);
/// assert_eq!(puzzle.id(), "binairo:4x4:1e0f1b");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    givens: Vec<Mark>,
}

impl Puzzle {
    /// A puzzle from each cell's given digit in row-major order, with
    /// [`Mark::Unmarked`] for a cell without one.
    pub fn new(shape: Shape, givens: Vec<Mark>) -> Result<Self, ParseError> {
        if givens.len() != shape.cells() {
            return Err(ParseError::new(format!(
                "a {shape} grid needs {} cells",
                shape.cells()
            )));
        }
        Ok(Self { shape, givens })
    }

    /// Read the grid rows as written in text: `0`, `1` or `.` for each cell.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("Binairo needs 2 to {MAX_SIZE} rows"),
            ));
        }
        let a = Answer::from_rows(rows)?;
        Self::new(a.shape(), a.marks().to_vec())
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// The given digits in row-major order, [`Mark::Unmarked`] where there
    /// is none.
    pub fn givens(&self) -> &[Mark] {
        &self.givens
    }

    /// The given digit at `row`, `col` (both counted from 0), or
    /// [`Mark::Unmarked`].
    ///
    /// # Panics
    ///
    /// Panics if the cell is outside the grid.
    pub fn given(&self, row: usize, col: usize) -> Mark {
        assert!(
            row < self.shape.rows() && col < self.shape.cols(),
            "cell ({row}, {col}) is outside a {} grid",
            self.shape
        );
        self.givens[row * self.shape.cols() + col]
    }

    /// The grid rows, as written in text.
    pub fn rows(&self) -> Vec<String> {
        self.givens
            .chunks(self.shape.cols())
            .map(|row| row.iter().map(|m| m.to_char()).collect())
            .collect()
    }

    /// The puzzle's ID, such as `binairo:4x4:1e0e1b`: the givens in reading
    /// order, with a letter for each run of empty cells (`a` for 1 to `z`
    /// for 26).
    pub fn id(&self) -> String {
        id::join("binairo", &self.shape.to_string(), &describe(&self.givens))
    }

    /// Read a puzzle ID, reporting the first wrong character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "binairo" {
            return Err(ParseError::at_char(1, "expected a binairo ID"));
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
        let givens = read_description(desc, parsed)
            .map_err(|(at, message)| ParseError::at_char(start + at, message))?;
        Self::new(parsed, givens)
    }

    /// Whether a complete answer obeys every rule and keeps the givens.
    pub fn is_solved_by(&self, a: &Answer) -> bool {
        a.is_complete() && crate::check(self, a).is_ok_and(|v| v.is_empty())
    }

    /// Draw the grid for a terminal: the givens, and the answer's digits in
    /// the other cells, with `.` for empty cells.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| a.shape() != self.shape) {
            return "the answer has the wrong shape".into();
        }
        let cols = self.shape.cols();
        (0..self.shape.rows())
            .map(|r| {
                (0..cols)
                    .map(|c| match self.givens[r * cols + c] {
                        Mark::Unmarked => answer.map_or('.', |a| a.get(r, c).to_char()),
                        given => given.to_char(),
                    })
                    .map(String::from)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Run letters: `a` is 1 empty cell, `z` is 26.
const RUN: u8 = 26;

/// The ID description: `0` and `1` for givens, and a letter for each run of
/// empty cells; a run longer than 26 is `z`s followed by one more letter.
fn describe(givens: &[Mark]) -> String {
    let mut out = String::new();
    let mut run = 0u32;
    let flush = |out: &mut String, run: &mut u32| {
        while *run > 0 {
            let n = (*run).min(u32::from(RUN));
            out.push(char::from(b'a' + n as u8 - 1));
            *run -= n;
        }
    };
    for &m in givens {
        match m {
            Mark::Unmarked => run += 1,
            given => {
                flush(&mut out, &mut run);
                out.push(given.to_char());
            }
        }
    }
    flush(&mut out, &mut run);
    out
}

/// Read an ID description, giving the 0-based position of an error.
fn read_description(desc: &str, shape: Shape) -> Result<Vec<Mark>, (usize, String)> {
    let cells = shape.cells();
    let mut givens = Vec::with_capacity(cells);
    let mut previous = b'0';
    for (at, b) in desc.bytes().enumerate() {
        let count = match b {
            b'0' | b'1' => 1,
            b'a'..=b'z' => {
                if previous.is_ascii_lowercase() && previous != b'z' {
                    return Err((at, "only a `z` run may be followed by another run".into()));
                }
                usize::from(b - b'a' + 1)
            }
            _ => return Err((at, "use 0, 1, or a-z for 1 to 26 empty cells".into())),
        };
        if givens.len() + count > cells {
            return Err((at, "the description has too many cells".into()));
        }
        match b {
            b'0' => givens.push(Mark::Zero),
            b'1' => givens.push(Mark::One),
            _ => givens.extend(std::iter::repeat_n(Mark::Unmarked, count)),
        }
        previous = b;
    }
    if givens.len() != cells {
        return Err((desc.len(), "the description has too few cells".into()));
    }
    Ok(givens)
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
    bounded_vec::<D, BoundedText<MAX_SIZE>, MAX_SIZE>(d)
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

/// A Binairo puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

/// Read Binairo documents, keeping their shared keys (F6-F8).
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
    if doc.genre().is_some_and(|g| g != "binairo") {
        return Err(ParseError::at_line(start, "expected a binairo puzzle"));
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
                    "the solution must give every cell `0` or `1`",
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
    let mut writer = Writer::new("binairo");
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
        let shape: Shape = "6x6".parse().unwrap();
        let mut givens = vec![Mark::Unmarked; 36];
        givens[0] = Mark::One;
        givens[35] = Mark::Zero;
        let d = describe(&givens);
        assert_eq!(d, "1zh0");
        assert_eq!(read_description(&d, shape).unwrap(), givens);
        let small: Shape = "2x2".parse().unwrap();
        assert_eq!(read_description("1c", small).unwrap(), {
            let mut g = vec![Mark::Unmarked; 4];
            g[0] = Mark::One;
            g
        });
        assert_eq!(read_description("1ab", small).unwrap_err().0, 2);
        assert_eq!(read_description("1d", small).unwrap_err().0, 1);
        assert_eq!(read_description("10", small).unwrap_err().0, 2);
        assert_eq!(read_description("102", small).unwrap_err().0, 2);
        assert_eq!(read_description("1010A", small).unwrap_err().0, 4);
    }
}
