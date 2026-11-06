//! Validated grids of white cells and blocks with sums, and strict shared
//! records.
use crate::{Answer, MAX_SIZE, Mark, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, id,
    text::{self, Document, Writer},
};
use std::{fmt, str::FromStr};

/// The genre's text limits: rows, and bytes in a row of `45/45` tokens
/// with spaces.
pub(crate) const LIMITS: text::GridLimits = text::GridLimits::new(MAX_SIZE, 6 * MAX_SIZE);

/// The most cells a run may have.
pub const MAX_RUN: usize = 9;
/// The largest sum: 1 + 2 + ... + 9.
pub const MAX_SUM: u8 = 45;

/// One cell of the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    /// A white cell, to hold a digit.
    White,
    /// A block, holding the sum of the down run below it, the sum of the
    /// across run to its right, both or neither.
    Block {
        /// The sum of the run below, if one starts there.
        down: Option<u8>,
        /// The sum of the run to the right, if one starts there.
        across: Option<u8>,
    },
}

impl Cell {
    /// A block without sums.
    pub const BLOCK: Self = Self::Block {
        down: None,
        across: None,
    };

    /// The text token: `.`, `#`, or the sums around a `/`, such as `4/6`,
    /// `4/` (down only) and `/6` (across only).
    pub fn token(self) -> String {
        match self {
            Self::White => ".".into(),
            Self::Block {
                down: None,
                across: None,
            } => "#".into(),
            Self::Block { down, across } => {
                let part = |s: Option<u8>| s.map_or(String::new(), |s| s.to_string());
                format!("{}/{}", part(down), part(across))
            }
        }
    }

    /// Read a text token.
    pub fn from_token(s: &str) -> Result<Self, ParseError> {
        match s {
            "." => return Ok(Self::White),
            "#" => return Ok(Self::BLOCK),
            _ => {}
        }
        let Some((down, across)) = s.split_once('/') else {
            return Err(ParseError::new(format!(
                "{s:?} is not a cell; use `.`, `#` or sums such as `4/6`, `4/` or `/6`"
            )));
        };
        let sum = |part: &str| -> Result<Option<u8>, ParseError> {
            if part.is_empty() {
                return Ok(None);
            }
            if part.len() > 2 || part.starts_with('0') || !part.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(ParseError::new(format!(
                    "{part:?} is not a sum; sums are numbers from 1 to {MAX_SUM}"
                )));
            }
            let v: u8 = part.parse().expect("two digits");
            if v > MAX_SUM {
                return Err(ParseError::new(format!("sums are at most {MAX_SUM}")));
            }
            Ok(Some(v))
        };
        let (down, across) = (sum(down)?, sum(across)?);
        if down.is_none() && across.is_none() {
            return Err(ParseError::new("use `#` for a block without sums"));
        }
        Ok(Self::Block { down, across })
    }
}

/// A run: its cells in order, its sum, the block holding the sum and
/// whether it runs down.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Run {
    pub cells: Vec<usize>,
    pub sum: u32,
    pub clue: usize,
    pub down: bool,
}

/// A Kakuro grid: white cells and blocks, some holding the sums of the
/// runs next to them. Every white cell is in an across run and a down run
/// of 2 to 9 cells, and every run has its sum.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    cells: Vec<Cell>,
    runs: Vec<Run>,
}

impl Puzzle {
    /// A puzzle from row-major cells. Every white cell must be in an
    /// across run and a down run of 2 to 9 cells, the block before each run
    /// must hold its sum, a block may hold only those sums, and sums are 1
    /// to 45. Sums that no digits can reach are accepted; such a puzzle has
    /// no solution. Errors give the grid row as their line.
    pub fn new(shape: Shape, cells: Vec<Cell>) -> Result<Self, ParseError> {
        let runs = runs(shape, &cells)?;
        Ok(Self { shape, cells, runs })
    }

    /// Read rows of space-separated tokens, or of one character per cell
    /// when no cell holds a sum.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        if rows.is_empty() || rows.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("Kakuro needs 1 to {MAX_SIZE} rows"),
            ));
        }
        let spaced = rows.iter().any(|r| r.as_ref().contains(' '));
        let tokens = |row: &str| -> Vec<String> {
            if spaced {
                row.split(' ')
                    .take(MAX_SIZE + 1)
                    .map(str::to_owned)
                    .collect()
            } else {
                row.chars().take(MAX_SIZE + 1).map(String::from).collect()
            }
        };
        let first = tokens(rows[0].as_ref());
        let shape = Shape::new(rows.len(), first.len()).map_err(|e| e.or_at_line(1))?;
        let mut cells = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let row = tokens(row.as_ref());
            if row.len() != shape.cols() {
                return Err(ParseError::at_line(
                    r + 1,
                    "each grid row must have the same number of cells",
                ));
            }
            for t in row {
                cells.push(Cell::from_token(&t).map_err(|e| e.or_at_line(r + 1))?);
            }
        }
        Self::new(shape, cells)
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// The cells in row-major order.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// The cell at `row`, `col` (both counted from 0).
    ///
    /// # Panics
    ///
    /// Panics if the cell is outside the grid.
    pub fn cell(&self, row: usize, col: usize) -> Cell {
        assert!(row < self.shape.rows() && col < self.shape.cols());
        self.cells[row * self.shape.cols() + col]
    }

    /// The runs, in the order of their sums in the ID: blocks row-major,
    /// the down run before the across run.
    pub(crate) fn runs(&self) -> &[Run] {
        &self.runs
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

    /// The puzzle's ID, such as `kakuro:3x3:DbAb/4.6.3.7`.
    pub fn id(&self) -> String {
        id::join("kakuro", &self.shape.to_string(), &self.describe())
    }

    /// Read a puzzle ID, reporting the first wrong character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "kakuro" {
            return Err(ParseError::at_char(1, "expected a kakuro ID"));
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
        let cells = read_description(desc, parsed)
            .map_err(|(at, message)| ParseError::at_char(start + at, message))?;
        Self::new(parsed, cells).map_err(|e| ParseError::at_char(start, e.message()))
    }

    /// The ID description: the layout row-major, as runs of 1 to 26 blocks
    /// (`A` to `Z`) and white cells (`a` to `z`), then `/` and the sums
    /// separated by `.` in the order of [`Puzzle::runs`].
    fn describe(&self) -> String {
        let mut out = String::new();
        let mut kind = None;
        let mut count = 0usize;
        let flush = |out: &mut String, white: Option<bool>, count: &mut usize| {
            while *count > 0 {
                let k = (*count).min(26);
                let base = if white == Some(true) { b'a' } else { b'A' };
                out.push(char::from(base + k as u8 - 1));
                *count -= k;
            }
        };
        for &c in &self.cells {
            let white = Some(c == Cell::White);
            if white != kind {
                flush(&mut out, kind, &mut count);
                kind = white;
            }
            count += 1;
        }
        flush(&mut out, kind, &mut count);
        out.push('/');
        let sums: Vec<String> = self.runs.iter().map(|r| r.sum.to_string()).collect();
        out.push_str(&sums.join("."));
        out
    }

    /// Whether a complete answer obeys every rule.
    pub fn is_solved_by(&self, a: &Answer) -> bool {
        a.is_complete()
            && crate::check(self, a, &pencil_puzzles_core::Budget::unlimited())
                .is_ok_and(|v| v.is_empty())
    }

    /// Whether an answer has blocks exactly where the puzzle does.
    pub(crate) fn fits(&self, a: &Answer) -> bool {
        a.shape() == self.shape
            && self
                .cells
                .iter()
                .zip(a.marks())
                .all(|(&c, &m)| (c == Cell::White) == (m != Mark::Block))
    }

    /// Draw the grid, and optionally an answer's digits, for a terminal.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| !self.fits(a)) {
            return "the answer does not fit the puzzle".into();
        }
        let tokens: Vec<String> = self
            .cells
            .iter()
            .enumerate()
            .map(|(i, &c)| match (c, answer) {
                (Cell::White, Some(a)) => a.marks()[i].symbol().to_string(),
                _ => c.token(),
            })
            .collect();
        let width = tokens.iter().map(String::len).max().unwrap_or(1);
        tokens
            .chunks(self.shape.cols())
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

/// The runs of a grid, checked as [`Puzzle::new`] describes.
fn runs(shape: Shape, cells: &[Cell]) -> Result<Vec<Run>, ParseError> {
    let (rows, cols) = (shape.rows(), shape.cols());
    if cells.len() != shape.cells() {
        return Err(ParseError::new("the cells must match the grid"));
    }
    let n = cells.len();
    for (i, &c) in cells.iter().enumerate() {
        if let Cell::Block { down, across } = c
            && [down, across]
                .iter()
                .any(|s| s.is_some_and(|s| s == 0 || s > MAX_SUM))
        {
            return Err(ParseError::at_line(
                i / cols + 1,
                format!("sums are from 1 to {MAX_SUM}"),
            ));
        }
    }
    let white = |i: usize| cells[i] == Cell::White;
    let place = |i: usize| format!("row {}, column {}", i / cols + 1, i % cols + 1);
    // For each block, the run it starts in each direction.
    let mut starts: [Vec<Option<Vec<usize>>>; 2] = [vec![None; n], vec![None; n]];
    for down in [false, true] {
        let (lines, len, step) = if down {
            (cols, rows, cols)
        } else {
            (rows, cols, 1)
        };
        let name = if down { "down" } else { "across" };
        for line in 0..lines {
            let first = if down { line } else { line * cols };
            let mut k = 0;
            while k < len {
                if !white(first + k * step) {
                    k += 1;
                    continue;
                }
                let begin = k;
                while k < len && white(first + k * step) {
                    k += 1;
                }
                let run: Vec<usize> = (begin..k).map(|j| first + j * step).collect();
                let at = run[0];
                if begin == 0 {
                    return Err(ParseError::at_line(
                        at / cols + 1,
                        format!(
                            "the white cell in {} has no block before it for its {name} sum",
                            place(at)
                        ),
                    ));
                }
                if run.len() == 1 {
                    return Err(ParseError::at_line(
                        at / cols + 1,
                        format!(
                            "the white cell in {} is in a {name} run of one cell; runs have 2 to {MAX_RUN} cells",
                            place(at)
                        ),
                    ));
                }
                if run.len() > MAX_RUN {
                    return Err(ParseError::at_line(
                        at / cols + 1,
                        format!(
                            "the {name} run starting in {} has more than {MAX_RUN} cells",
                            place(at)
                        ),
                    ));
                }
                let clue = at - step;
                let has_sum = match cells[clue] {
                    Cell::Block { down: d, across: a } => if down { d } else { a }.is_some(),
                    Cell::White => false,
                };
                if !has_sum {
                    return Err(ParseError::at_line(
                        clue / cols + 1,
                        format!(
                            "the {name} run starting in {} has no sum in the block before it",
                            place(at)
                        ),
                    ));
                }
                starts[usize::from(down)][clue] = Some(run);
            }
        }
    }
    let mut out = Vec::new();
    for (i, &c) in cells.iter().enumerate() {
        let Cell::Block { down, across } = c else {
            continue;
        };
        for (is_down, sum) in [(true, down), (false, across)] {
            let Some(sum) = sum else { continue };
            let Some(run) = starts[usize::from(is_down)][i].take() else {
                return Err(ParseError::at_line(
                    i / cols + 1,
                    format!(
                        "the block in {} has a {} sum but no run {}",
                        place(i),
                        if is_down { "down" } else { "across" },
                        if is_down { "below it" } else { "to its right" }
                    ),
                ));
            };
            out.push(Run {
                cells: run,
                sum: u32::from(sum),
                clue: i,
                down: is_down,
            });
        }
    }
    Ok(out)
}

/// Read an ID description, giving the 0-based position of an error.
fn read_description(desc: &str, shape: Shape) -> Result<Vec<Cell>, (usize, String)> {
    let bytes = desc.as_bytes();
    let n = shape.cells();
    let mut white = Vec::with_capacity(n);
    let mut at = 0;
    while at < bytes.len() && bytes[at] != b'/' {
        let (is_white, k) = match bytes[at] {
            b @ b'a'..=b'z' => (true, usize::from(b - b'a') + 1),
            b @ b'A'..=b'Z' => (false, usize::from(b - b'A') + 1),
            _ => {
                return Err((
                    at,
                    "the layout uses lowercase letters for white cells and uppercase letters for blocks"
                        .into(),
                ));
            }
        };
        if white.len() + k > n {
            return Err((at, "the layout has too many cells".into()));
        }
        white.extend(std::iter::repeat_n(is_white, k));
        at += 1;
    }
    if white.len() < n {
        return Err((at, "the layout has too few cells".into()));
    }
    if at == bytes.len() {
        return Err((at, "the layout needs `/` and the sums after it".into()));
    }
    at += 1;
    let cols = shape.cols();
    // The sums each block needs, in ID order: down before across.
    let mut needs = Vec::new();
    for i in 0..n {
        if white[i] {
            continue;
        }
        if i + cols < n && white[i + cols] {
            needs.push((i, true));
        }
        if i % cols + 1 < cols && white[i + 1] {
            needs.push((i, false));
        }
    }
    let mut sums = Vec::new();
    if at < bytes.len() {
        loop {
            let start = at;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            let digits = &desc[start..at];
            if digits.is_empty() || digits.len() > 2 || digits.starts_with('0') {
                return Err((
                    start,
                    format!("sums are numbers from 1 to {MAX_SUM}, separated by `.`"),
                ));
            }
            let v: u8 = digits.parse().expect("two digits");
            if v > MAX_SUM {
                return Err((start, format!("sums are at most {MAX_SUM}")));
            }
            if sums.len() == needs.len() {
                return Err((start, "there are more sums than runs".into()));
            }
            sums.push(v);
            if at == bytes.len() {
                break;
            }
            if bytes[at] != b'.' {
                return Err((at, "separate sums with `.`".into()));
            }
            at += 1;
            if at == bytes.len() {
                return Err((at, "a sum is missing after `.`".into()));
            }
        }
    }
    if sums.len() < needs.len() {
        return Err((bytes.len(), "there are fewer sums than runs".into()));
    }
    let mut cells: Vec<Cell> = white
        .iter()
        .map(|&w| if w { Cell::White } else { Cell::BLOCK })
        .collect();
    for (&(i, down), &sum) in needs.iter().zip(&sums) {
        if let Cell::Block { down: d, across: a } = &mut cells[i] {
            *if down { d } else { a } = Some(sum);
        }
    }
    Ok(cells)
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
    bounded_vec::<D, BoundedText<{ 6 * MAX_SIZE }>, MAX_SIZE>(d)
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

/// A Kakuro puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

/// Read Kakuro documents, keeping their shared keys (F6-F8).
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
    if doc.genre().is_some_and(|g| g != "kakuro") {
        return Err(ParseError::at_line(start, "expected a kakuro puzzle"));
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
            if !p.fits(&a) || !a.is_complete() {
                return Err(ParseError::at_line(
                    block.line,
                    "the solution must have the puzzle's blocks and a digit in every white cell",
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
    let mut writer = Writer::new("kakuro");
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
        let p = Puzzle::from_rows(&["# 4/ 6/", "/3 . .", "/7 . ."]).unwrap();
        assert_eq!(p.describe(), "DbAb/4.6.3.7");
        let shape = p.shape();
        assert_eq!(read_description("DbAb/4.6.3.7", shape).unwrap(), p.cells);
        for (desc, at) in [
            ("DbAb/4.6.3", 10),
            ("DbAb/4.6.3.7.1", 13),
            ("DbAb/4.6.3.", 11),
            ("DbAb/4.6.3x7", 10),
            ("DbAb/4.6.46.7", 9),
            ("DbAb/04.6.3.7", 5),
            ("DbA", 3),
            ("DbAbA/1", 4),
            ("DbAb", 4),
            ("D1", 1),
        ] {
            assert_eq!(read_description(desc, shape).unwrap_err().0, at, "{desc}");
        }
        let blocks = Puzzle::new(shape, vec![Cell::BLOCK; 9]).unwrap();
        assert_eq!(blocks.id(), "kakuro:3x3:I/");
        assert_eq!(Puzzle::from_id("kakuro:3x3:I/").unwrap(), blocks);
    }
}
