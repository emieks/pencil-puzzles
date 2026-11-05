//! Validated shapes, grids, answers and record formats (P2-P4, F1-F14).
use pencil_puzzles_core::{
    ParseError, Position, id,
    text::{self, Document, Writer},
};
use std::{fmt, str::FromStr};

/// Largest supported side length.
pub const MAX_SIZE: usize = 9;
/// Grid shape, written `1x1`, `4x4` or `9x9`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
pub struct Shape {
    size: usize,
}
impl Shape {
    /// Validate a square Sudoku size with square boxes.
    pub fn new(size: usize) -> Result<Self, ParseError> {
        if ![1, 4, 9].contains(&size) {
            return Err(ParseError::new("Sudoku sizes are 1x1, 4x4 and 9x9"));
        }
        Ok(Self { size })
    }
    /// Side length.
    pub fn size(self) -> usize {
        self.size
    }
    /// Box side length.
    pub fn box_size(self) -> usize {
        match self.size {
            1 => 1,
            4 => 2,
            _ => 3,
        }
    }
    /// Number of cells.
    pub fn cells(self) -> usize {
        self.size * self.size
    }
}
impl Default for Shape {
    fn default() -> Self {
        Self { size: 9 }
    }
}
impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.size, self.size)
    }
}
impl FromStr for Shape {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        match s {
            "1x1" => Self::new(1),
            "4x4" => Self::new(4),
            "9x9" => Self::new(9),
            _ => {
                let prefix = ["1x1", "4x4", "9x9"]
                    .iter()
                    .map(|valid| {
                        valid
                            .chars()
                            .zip(s.chars())
                            .take_while(|(a, b)| a == b)
                            .count()
                    })
                    .max()
                    .unwrap_or(0);
                Err(ParseError::at_char(
                    prefix + 1,
                    "Sudoku sizes are 1x1, 4x4 and 9x9",
                ))
            }
        }
    }
}
impl From<Shape> for String {
    fn from(s: Shape) -> Self {
        s.to_string()
    }
}
impl TryFrom<String> for Shape {
    type Error = ParseError;
    fn try_from(s: String) -> Result<Self, ParseError> {
        s.parse()
    }
}
/// Generation parameters. Sudoku has no settings beyond its shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
#[non_exhaustive]
pub struct Params {
    /// Shape to generate.
    pub shape: Shape,
}
impl Params {
    /// Parameters for a validated shape.
    pub fn new(shape: Shape) -> Self {
        Self { shape }
    }
}
impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.shape.fmt(f)
    }
}
impl FromStr for Params {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        s.parse().map(Self::new)
    }
}
impl From<Params> for String {
    fn from(p: Params) -> Self {
        p.to_string()
    }
}
impl TryFrom<String> for Params {
    type Error = ParseError;
    fn try_from(s: String) -> Result<Self, ParseError> {
        s.parse()
    }
}

// Explicitly empty is distinct from unmarked (P4): it has no legal digit.
pub(crate) const EMPTY: u8 = 10;
/// Player marks: `.` unmarked, `0` explicitly empty, or a digit.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(into = "Vec<String>"))]
pub struct Answer {
    pub(crate) shape: Shape,
    pub(crate) cells: Vec<u8>,
}
impl Answer {
    /// An unmarked grid of a validated shape.
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            cells: vec![0; shape.cells()],
        }
    }
    /// Read one row per string, validating dimensions before allocating the grid.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        let shape = Shape::new(rows.len()).map_err(|e| e.or_at_line(1))?;
        let mut cells = Vec::with_capacity(shape.cells());
        for (r, row) in rows.iter().enumerate() {
            let row = row.as_ref();
            if row.len() != shape.size {
                return Err(ParseError::at_line(
                    r + 1,
                    format!("row {} needs {} cells", r + 1, shape.size),
                ));
            }
            for c in row.bytes() {
                cells.push(match c {
                    b'.' => 0,
                    b'0' => EMPTY,
                    b'1'..=b'9' if usize::from(c - b'0') <= shape.size => c - b'0',
                    _ => {
                        return Err(ParseError::at_line(
                            r + 1,
                            format!("use '.', '0', or digits 1 through {}", shape.size),
                        ));
                    }
                });
            }
        }
        Ok(Self { shape, cells })
    }
    /// The grid shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }
    /// A digit, 0 for unmarked or explicitly empty.
    ///
    /// # Panics
    /// Panics if the row or column is outside the grid.
    pub fn get(&self, row: usize, col: usize) -> u8 {
        assert!(
            row < self.shape.size && col < self.shape.size,
            "cell outside the grid"
        );
        let v = self.cells[row * self.shape.size + col];
        if v == EMPTY { 0 } else { v }
    }
    /// Set a digit, or 0 to erase it. Invalid coordinates and values are errors.
    pub fn set(&mut self, row: usize, col: usize, value: u8) -> Result<(), ParseError> {
        if row >= self.shape.size || col >= self.shape.size || usize::from(value) > self.shape.size
        {
            return Err(ParseError::new(
                "the cell or digit is outside the grid's range",
            ));
        }
        self.cells[row * self.shape.size + col] = value;
        Ok(())
    }
    /// Whether every cell has a mark, including explicitly empty marks.
    pub fn is_complete(&self) -> bool {
        !self.cells.contains(&0)
    }
    /// Replace unmarked cells with explicitly empty marks (P4).
    pub fn finished(&self) -> Self {
        let mut a = self.clone();
        for v in &mut a.cells {
            if *v == 0 {
                *v = EMPTY;
            }
        }
        a
    }
    /// Rows in canonical text form.
    pub fn rows(&self) -> Vec<String> {
        self.cells
            .chunks(self.shape.size)
            .map(|row| {
                row.iter()
                    .map(|&v| match v {
                        0 => '.',
                        EMPTY => '0',
                        _ => char::from(b'0' + v),
                    })
                    .collect()
            })
            .collect()
    }
    pub(crate) fn digits_only(&self) -> bool {
        self.cells
            .iter()
            .all(|&v| v > 0 && usize::from(v) <= self.shape.size)
    }
}
impl fmt::Display for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.rows().join("\n"))
    }
}
impl FromStr for Answer {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        let rows: Vec<_> = s
            .lines()
            .enumerate()
            .map(|(i, s)| (i + 1, s.trim()))
            .filter(|(_, s)| !s.is_empty())
            .take(MAX_SIZE + 1)
            .collect();
        Self::from_rows(&rows.iter().map(|(_, s)| *s).collect::<Vec<_>>()).map_err(|e| {
            let line = match e.position() {
                Position::Line(n) => rows.get(n - 1).map_or(n, |(i, _)| *i),
                _ => 1,
            };
            ParseError::at_line(line, e.message())
        })
    }
}
impl From<Answer> for Vec<String> {
    fn from(a: Answer) -> Self {
        a.rows()
    }
}
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Answer {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use pencil_puzzles_core::serde_helpers::{BoundedText, bounded_vec};
        let rows = bounded_vec::<D, BoundedText<MAX_SIZE>, MAX_SIZE>(d)?;
        Self::from_rows(&rows.into_iter().map(|r| r.0).collect::<Vec<_>>())
            .map_err(serde::de::Error::custom)
    }
}
/// A validated grid of printed Sudoku clues; 0 means no given.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "PuzzleData"))]
pub struct Puzzle {
    grid: Answer,
}
impl Puzzle {
    /// Validate row-major givens. Contradictory clues are allowed and solve to no solution.
    pub fn new(shape: Shape, givens: Vec<u8>) -> Result<Self, ParseError> {
        if givens.len() != shape.cells() || givens.iter().any(|&v| usize::from(v) > shape.size) {
            return Err(ParseError::new(
                "the givens must match the shape and its digit range",
            ));
        }
        Ok(Self {
            grid: Answer {
                shape,
                cells: givens,
            },
        })
    }
    /// Read a grid, accepting `.` or `0` for an absent clue.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Result<Self, ParseError> {
        let mut a = Answer::from_rows(rows)?;
        for v in &mut a.cells {
            if *v == EMPTY {
                *v = 0;
            }
        }
        Ok(Self { grid: a })
    }
    /// Make a fully given puzzle from an answer containing digits only.
    pub fn from_solution(answer: &Answer) -> Result<Self, ParseError> {
        if !answer.digits_only() {
            return Err(ParseError::new("a solution contains digits only"));
        }
        Ok(Self {
            grid: answer.clone(),
        })
    }
    /// The grid shape.
    pub fn shape(&self) -> Shape {
        self.grid.shape
    }
    /// Row-major givens, with 0 for no clue.
    pub fn givens(&self) -> &[u8] {
        &self.grid.cells
    }
    /// Whether a complete answer obeys all units and the givens.
    pub fn is_solved_by(&self, answer: &Answer) -> bool {
        if answer.shape != self.shape()
            || !answer.digits_only()
            || self
                .givens()
                .iter()
                .zip(&answer.cells)
                .any(|(&g, &v)| g != 0 && g != v)
        {
            return false;
        }
        let n = self.shape().size;
        let b = self.shape().box_size();
        let full = (1u16 << n) - 1;
        for i in 0..n {
            let bits = |indices: Vec<usize>| {
                indices
                    .into_iter()
                    .fold(0u16, |mask, j| mask | 1 << (answer.cells[j] - 1))
            };
            if bits((0..n).map(|c| i * n + c).collect()) != full
                || bits((0..n).map(|r| r * n + i).collect()) != full
            {
                return false;
            }
            let (r, c) = ((i / b) * b, (i % b) * b);
            if bits((0..n).map(|k| (r + k / b) * n + c + k % b).collect()) != full {
                return false;
            }
        }
        true
    }
    /// Stable puzzle ID (F9-F12).
    pub fn id(&self) -> String {
        id::join(
            "sudoku",
            &self.shape().to_string(),
            &self.grid.rows().concat(),
        )
    }
    /// Decode a bounded puzzle ID with character positions in errors.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, description) = id::split(source)?;
        if genre != "sudoku" {
            return Err(ParseError::at_char(1, "expected a sudoku ID"));
        }
        let shape: Shape = shape.parse().map_err(|e: ParseError| {
            ParseError::at_char(
                8 + match e.position() {
                    Position::Char(n) => n - 1,
                    _ => 0,
                },
                e.message(),
            )
        })?;
        let start = 8 + shape.to_string().len() + 1;
        let mut cells = Vec::with_capacity(shape.cells());
        for (i, b) in description.bytes().enumerate() {
            if i >= shape.cells() {
                return Err(ParseError::at_char(start + i, "too many Sudoku clues"));
            }
            cells.push(match b {
                b'.' | b'0' => 0,
                b'1'..=b'9' if usize::from(b - b'0') <= shape.size => b - b'0',
                _ => return Err(ParseError::at_char(start + i, "invalid Sudoku clue")),
            });
        }
        if cells.len() != shape.cells() {
            return Err(ParseError::at_char(
                start + description.len(),
                format!("the description needs {} cells", shape.cells()),
            ));
        }
        Self::new(shape, cells)
    }
    /// Terminal rendering with box separators. Any answer must have this shape.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        let grid = answer.unwrap_or(&self.grid);
        if grid.shape != self.shape() {
            return "the answer has the wrong shape".into();
        }
        let b = self.shape().box_size();
        let mut result = Vec::new();
        for (r, row) in grid.rows().iter().enumerate() {
            if r > 0 && r % b == 0 {
                result.push("-".repeat(self.shape().size * 2 + 2 * (b - 1) - 1));
            }
            result.push(
                row.as_bytes()
                    .chunks(b)
                    .map(|part| {
                        part.iter()
                            .map(|&c| char::from(c).to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
        }
        result.join("\n")
    }
}
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PuzzleData {
    grid: Answer,
}
#[cfg(feature = "serde")]
impl TryFrom<PuzzleData> for Puzzle {
    type Error = ParseError;
    fn try_from(mut p: PuzzleData) -> Result<Self, ParseError> {
        for v in &mut p.grid.cells {
            if *v == EMPTY {
                *v = 0;
            }
        }
        Ok(Self { grid: p.grid })
    }
}
/// A parsed Sudoku record with shared metadata.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;
/// Read Sudoku documents with preserved metadata (F6-F8).
pub fn parse_records(source: &str) -> Result<Vec<Record>, ParseError> {
    let docs = text::parse_with_limits(source, text::GridLimits::new(9, 9))?;
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
    if doc.genre().is_some_and(|g| g != "sudoku") {
        return Err(ParseError::at_line(start, "expected a sudoku puzzle"));
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
            if a.shape != p.shape() || !a.digits_only() {
                return Err(ParseError::at_line(
                    block.line,
                    "the solution must contain a digit for every cell",
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
/// Write a canonical Sudoku record.
pub fn write_record(record: &Record) -> String {
    let mut writer = Writer::new("sudoku");
    writer.shared_info(&record.shared);
    writer.grid("grid", record.puzzle.grid.rows());
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
