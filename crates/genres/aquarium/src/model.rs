//! Validated tanks and clues, their formats, and strict shared records.
use crate::{Answer, MAX_SIZE, Mark, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, grid, id,
    text::{self, Document, Writer},
};
use std::{collections::BTreeMap, fmt, str::FromStr};

/// The genre's text limits: rows, and bytes in a row of spaced 3-digit
/// tank numbers.
pub(crate) const LIMITS: text::GridLimits = text::GridLimits::new(MAX_SIZE, 4 * MAX_SIZE);

/// Tank letters in canonical order.
const LETTERS: &[u8; 52] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// The 64 characters that hold six wall bits each in an ID.
const WALL_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// A grid divided into tanks, with a clue for every row and column.
///
/// Tanks are numbered from 0 in the order their first cells appear, reading
/// row by row.
///
/// ```
/// use pencil_puzzles_aquarium::Puzzle;
///
/// let puzzle: Puzzle = "rows: 1 2\ncols: 2 1\nregions:\nab\naa".parse().unwrap();
/// assert_eq!(puzzle.tanks(), [0, 1, 0, 0]);
/// assert_eq!(puzzle.id(), "aquarium:2x2:12/21/k");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    tanks: Vec<u32>,
    tank_count: usize,
    row_clues: Vec<u32>,
    col_clues: Vec<u32>,
}

/// The tanks' cells by row, shared by the solver, the checker and the
/// techniques. A tank's cells in one row form a group. A tank is connected,
/// so its groups are in consecutive rows.
pub(crate) struct Layout {
    pub rows: usize,
    pub cols: usize,
    /// Each group's cells, left to right. Groups are in the order of their
    /// first cells, reading row by row.
    pub groups: Vec<Vec<usize>>,
    /// Each cell's group.
    pub group_of: Vec<usize>,
    /// Each tank's groups, top to bottom.
    pub tank_groups: Vec<Vec<usize>>,
}

impl Layout {
    pub(crate) fn new(p: &Puzzle) -> Self {
        let (rows, cols) = (p.shape.rows(), p.shape.cols());
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut group_of = vec![0; p.tanks.len()];
        let mut tank_groups: Vec<Vec<usize>> = vec![Vec::new(); p.tank_count];
        // The row of each tank's latest group.
        let mut latest = vec![usize::MAX; p.tank_count];
        for (i, &t) in p.tanks.iter().enumerate() {
            let t = t as usize;
            if latest[t] != i / cols {
                latest[t] = i / cols;
                tank_groups[t].push(groups.len());
                groups.push(Vec::new());
            }
            let g = *tank_groups[t].last().expect("pushed above");
            groups[g].push(i);
            group_of[i] = g;
        }
        Self {
            rows,
            cols,
            groups,
            group_of,
            tank_groups,
        }
    }
}

/// Split a row into tokens: single characters, or space-separated tokens
/// when any row of the grid has a space. At most `MAX_SIZE + 1` tokens are
/// kept, so that oversized rows fail before allocating more.
fn tokens(row: &str, spaced: bool) -> Vec<&str> {
    if spaced {
        row.split(' ').take(MAX_SIZE + 1).collect()
    } else {
        row.char_indices()
            .take(MAX_SIZE + 1)
            .map(|(at, c)| &row[at..at + c.len_utf8()])
            .collect()
    }
}

/// Whether a token names a tank: one letter, or a number from 1 to 999.
fn is_label(token: &str) -> bool {
    let b = token.as_bytes();
    match b {
        [c] if c.is_ascii_alphabetic() => true,
        _ => !b.is_empty() && b.len() <= 3 && b[0] != b'0' && b.iter().all(u8::is_ascii_digit),
    }
}

/// The number of the tank a token names, numbering new names in order.
pub(crate) fn label<'a>(
    names: &mut BTreeMap<&'a str, u32>,
    token: &'a str,
) -> Result<u32, ParseError> {
    if !is_label(token) {
        return Err(ParseError::new(format!(
            "{token:?} is not a tank; use a letter or a number from 1 to 999"
        )));
    }
    let next = names.len() as u32;
    Ok(*names.entry(token).or_insert(next))
}

/// Read space-separated clues, each at most `max`.
fn read_clues(text: &str, count: usize, max: usize, name: &str) -> Result<Vec<u32>, ParseError> {
    let mut clues = Vec::with_capacity(count);
    for word in text.split(' ').take(count + 1) {
        let clue = (!word.is_empty()
            && word.len() <= 2
            && (word == "0" || !word.starts_with('0'))
            && word.bytes().all(|b| b.is_ascii_digit()))
        .then(|| word.parse::<u32>().ok())
        .flatten()
        .ok_or_else(|| ParseError::new(format!("{word:?} is not a clue")))?;
        if clue as usize > max {
            return Err(ParseError::new(format!(
                "a {name} clue can be at most {max}"
            )));
        }
        clues.push(clue);
    }
    if clues.len() != count {
        return Err(ParseError::new(format!(
            "the grid needs {count} {name} clues"
        )));
    }
    Ok(clues)
}

fn write_clues(clues: &[u32]) -> String {
    clues
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

impl Puzzle {
    /// A puzzle from a tank label for every cell (row by row), the row clues
    /// (top to bottom) and the column clues (left to right). Cells with the
    /// same label form a tank, whatever the labels are.
    ///
    /// Fails if a list has the wrong length, a clue is larger than its line,
    /// or a tank is not connected.
    pub fn new(
        shape: Shape,
        labels: &[u32],
        row_clues: Vec<u32>,
        col_clues: Vec<u32>,
    ) -> Result<Self, ParseError> {
        let (rows, cols) = (shape.rows(), shape.cols());
        if labels.len() != shape.cells() || row_clues.len() != rows || col_clues.len() != cols {
            return Err(ParseError::new(format!(
                "a {shape} grid needs {} tank labels, {rows} row clues and {cols} column clues",
                shape.cells()
            )));
        }
        for (clues, max, name) in [(&row_clues, cols, "row"), (&col_clues, rows, "column")] {
            if let Some(at) = clues.iter().position(|&clue| clue as usize > max) {
                return Err(ParseError::new(format!(
                    "{name} {} has the clue {}, but it has only {max} cells",
                    at + 1,
                    clues[at]
                )));
            }
        }
        // Number the tanks by first appearance.
        let mut numbers = BTreeMap::new();
        let mut first = Vec::new();
        let tanks: Vec<u32> = labels
            .iter()
            .enumerate()
            .map(|(i, &label)| {
                *numbers.entry(label).or_insert_with(|| {
                    first.push(i);
                    first.len() as u32 - 1
                })
            })
            .collect();
        let nb = grid::neighbours(rows, cols);
        let mut seen = 0;
        for (t, &start) in first.iter().enumerate() {
            seen += grid::flood(start, |j| tanks[j] == t as u32, &nb).len();
        }
        if seen != tanks.len() {
            return Err(ParseError::new(
                "every tank must be one group of cells joined by their sides",
            ));
        }
        Ok(Self {
            shape,
            tanks,
            tank_count: first.len(),
            row_clues,
            col_clues,
        })
    }

    /// Read the tank rows as written in text, with the clues. A cell holds
    /// a letter or a number from 1 to 999; rows with numbers above 9 separate
    /// their cells with spaces.
    pub fn from_rows<S: AsRef<str>>(
        regions: &[S],
        row_clues: Vec<u32>,
        col_clues: Vec<u32>,
    ) -> Result<Self, ParseError> {
        if regions.is_empty() || regions.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                1,
                format!("Aquarium needs 1 to {MAX_SIZE} rows"),
            ));
        }
        let spaced = regions.iter().any(|r| r.as_ref().contains(' '));
        let width = tokens(regions[0].as_ref(), spaced).len();
        let shape = Shape::new(regions.len(), width).map_err(|e| e.or_at_line(1))?;
        let mut names: BTreeMap<&str, u32> = BTreeMap::new();
        let mut labels = Vec::with_capacity(shape.cells());
        for (r, row) in regions.iter().enumerate() {
            let cells = tokens(row.as_ref(), spaced);
            if cells.len() != width {
                return Err(ParseError::at_line(
                    r + 1,
                    "each row must have the same number of cells",
                ));
            }
            for token in cells {
                labels.push(label(&mut names, token).map_err(|e| e.or_at_line(r + 1))?);
            }
        }
        Self::new(shape, &labels, row_clues, col_clues)
    }

    /// The puzzle with the same tanks whose clues count the water of a
    /// complete answer. It may have other solutions.
    ///
    /// # Panics
    ///
    /// Panics if the answer has another shape.
    pub fn with_clues_of(&self, answer: &Answer) -> Self {
        assert_eq!(answer.shape(), self.shape, "the answer has another shape");
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let count = |cells: &mut dyn Iterator<Item = (usize, usize)>| {
            cells.filter(|&(r, c)| answer.is_shaded(r, c)).count() as u32
        };
        Self {
            row_clues: (0..rows)
                .map(|r| count(&mut (0..cols).map(|c| (r, c))))
                .collect(),
            col_clues: (0..cols)
                .map(|c| count(&mut (0..rows).map(|r| (r, c))))
                .collect(),
            ..self.clone()
        }
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// Each cell's tank, row by row. Tanks are numbered from 0 in the order
    /// their first cells appear.
    pub fn tanks(&self) -> &[u32] {
        &self.tanks
    }

    /// The number of tanks.
    pub fn tank_count(&self) -> usize {
        self.tank_count
    }

    /// The row clues, top to bottom.
    pub fn row_clues(&self) -> &[u32] {
        &self.row_clues
    }

    /// The column clues, left to right.
    pub fn col_clues(&self) -> &[u32] {
        &self.col_clues
    }

    /// The tank rows, as written in text: letters for up to 52 tanks, and
    /// numbers from 1 separated by spaces for more.
    pub fn regions(&self) -> Vec<String> {
        let letters = self.tank_count <= LETTERS.len();
        self.tanks
            .chunks(self.shape.cols())
            .map(|row| {
                if letters {
                    row.iter()
                        .map(|&t| char::from(LETTERS[t as usize]))
                        .collect()
                } else {
                    row.iter()
                        .map(|t| (t + 1).to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                }
            })
            .collect()
    }

    /// Whether there is a wall between each pair of neighbouring cells:
    /// first between each cell and the one to its right, row by row, then
    /// between each cell and the one below it, row by row.
    fn walls(&self) -> Vec<bool> {
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let t = &self.tanks;
        let mut walls = Vec::with_capacity(2 * rows * cols);
        for r in 0..rows {
            for c in 0..cols - 1 {
                walls.push(t[r * cols + c] != t[r * cols + c + 1]);
            }
        }
        for i in 0..(rows - 1) * cols {
            walls.push(t[i] != t[i + cols]);
        }
        walls
    }

    /// The puzzle's ID, such as `aquarium:2x2:12/21/k`: the row clues, the
    /// column clues and the walls between tanks.
    pub fn id(&self) -> String {
        let clues = |clues: &[u32]| -> String {
            clues
                .iter()
                .map(|&n| char::from_digit(n, 36).expect("clues are below 36"))
                .collect()
        };
        let walls: String = self
            .walls()
            .chunks(6)
            .map(|bits| {
                let value = (0..6).fold(0, |v, k| v << 1 | usize::from(bits.get(k) == Some(&true)));
                char::from(WALL_CHARS[value])
            })
            .collect();
        id::join(
            "aquarium",
            &self.shape.to_string(),
            &format!(
                "{}/{}/{walls}",
                clues(&self.row_clues),
                clues(&self.col_clues)
            ),
        )
    }

    /// Read a puzzle ID, reporting the first wrong character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "aquarium" {
            return Err(ParseError::at_char(1, "expected an aquarium ID"));
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
        read_description(desc, parsed)
            .map_err(|(at, message)| ParseError::at_char(start + at, message))
    }

    /// Whether a complete answer obeys every rule.
    pub fn is_solved_by(&self, a: &Answer) -> bool {
        a.is_complete() && crate::check(self, a).is_ok_and(|v| v.is_empty())
    }

    /// Draw the tanks for a terminal, with the column clues above and the
    /// row clues to the left. Water is `#` and cells marked as air are `-`.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| a.shape() != self.shape) {
            return "the answer has the wrong shape".into();
        }
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let t = &self.tanks;
        let pad = if rows > 9 || cols > 9 { 2 } else { 1 };
        let margin = " ".repeat(pad + 1);
        let mut lines = vec![
            format!(
                "{margin} {}",
                self.col_clues
                    .iter()
                    .map(|n| format!("{n:<2}"))
                    .collect::<String>()
            )
            .trim_end()
            .to_string(),
        ];
        // The line of walls above row `r`; row `rows` is the bottom edge.
        let border = |r: usize| -> String {
            let mut line = margin.clone();
            for c in 0..cols {
                let wall = r == 0 || r == rows || t[(r - 1) * cols + c] != t[r * cols + c];
                line.push('+');
                line.push(if wall { '-' } else { ' ' });
            }
            line.push('+');
            line
        };
        for r in 0..rows {
            lines.push(border(r));
            let mut line = format!("{:>pad$} ", self.row_clues[r]);
            for c in 0..cols {
                let wall = c == 0 || t[r * cols + c - 1] != t[r * cols + c];
                line.push(if wall { '|' } else { ' ' });
                line.push(match answer.map(|a| a.get(r, c)) {
                    Some(Mark::Shaded) => '#',
                    Some(Mark::Unshaded) => '-',
                    _ => ' ',
                });
            }
            line.push('|');
            lines.push(line);
        }
        lines.push(border(rows));
        lines.join("\n")
    }
}

/// Read an ID description, giving the 0-based position of an error.
fn read_description(desc: &str, shape: Shape) -> Result<Puzzle, (usize, String)> {
    let (rows, cols) = (shape.rows(), shape.cols());
    let mut parts = desc.splitn(3, '/');
    let (Some(row_part), Some(col_part), Some(wall_part)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return Err((
            desc.len(),
            "the description needs row clues, column clues and walls, separated by `/`".into(),
        ));
    };
    let clues = |part: &str, start: usize, count: usize, max: usize, name: &str| {
        let mut clues = Vec::with_capacity(count);
        for (at, b) in part.bytes().enumerate() {
            if at == count {
                return Err((start + at, format!("there are too many {name} clues")));
            }
            let clue = match b {
                b'0'..=b'9' | b'a'..=b'z' => char::from(b).to_digit(36).expect("base 36 digit"),
                _ => return Err((start + at, "use 0-9, or a-z for 10 to 35".into())),
            };
            if clue as usize > max {
                return Err((start + at, format!("a {name} clue can be at most {max}")));
            }
            clues.push(clue);
        }
        if clues.len() != count {
            return Err((
                start + part.len(),
                format!("there are too few {name} clues"),
            ));
        }
        Ok(clues)
    };
    let row_clues = clues(row_part, 0, rows, cols, "row")?;
    let col_start = row_part.len() + 1;
    let col_clues = clues(col_part, col_start, cols, rows, "column")?;
    let wall_start = col_start + col_part.len() + 1;
    let across = rows * (cols - 1);
    let count = across + (rows - 1) * cols;
    let mut walls = Vec::with_capacity(count + 5);
    for (at, b) in wall_part.bytes().enumerate() {
        if walls.len() >= count {
            return Err((wall_start + at, "there are too many walls".into()));
        }
        let Some(value) = WALL_CHARS.iter().position(|&w| w == b) else {
            return Err((wall_start + at, "use A-Z, a-z, 0-9, `-` or `_`".into()));
        };
        walls.extend((0..6).rev().map(|k| value >> k & 1 == 1));
        if walls[count.min(walls.len())..].contains(&true) {
            return Err((wall_start + at, "the unused wall bits must be 0".into()));
        }
    }
    if walls.len() < count {
        return Err((desc.len(), "there are too few walls".into()));
    }
    // Cells without a wall between them share a tank.
    let nb = grid::neighbours(rows, cols);
    let wall = |i: usize, j: usize| {
        let (i, j) = (i.min(j), i.max(j));
        if j == i + 1 {
            (i / cols) * (cols - 1) + i % cols
        } else {
            across + i
        }
    };
    let mut labels = vec![u32::MAX; shape.cells()];
    let mut next = 0;
    for start in 0..labels.len() {
        if labels[start] != u32::MAX {
            continue;
        }
        labels[start] = next;
        let mut todo = vec![start];
        while let Some(i) = todo.pop() {
            for &j in &nb[i] {
                if labels[j] == u32::MAX && !walls[wall(i, j)] {
                    labels[j] = next;
                    todo.push(j);
                }
            }
        }
        next += 1;
    }
    for i in 0..labels.len() {
        for &j in &nb[i] {
            if j > i && walls[wall(i, j)] && labels[i] == labels[j] {
                return Err((
                    wall_start + wall(i, j) / 6,
                    "a wall may not stand inside a tank".into(),
                ));
            }
        }
    }
    Puzzle::new(shape, &labels, row_clues, col_clues).map_err(|e| (0, e.message().to_string()))
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PuzzleData {
    #[serde(deserialize_with = "read_clue_list")]
    row_clues: Vec<u32>,
    #[serde(deserialize_with = "read_clue_list")]
    col_clues: Vec<u32>,
    #[serde(deserialize_with = "read_rows")]
    regions: Vec<String>,
}

#[cfg(feature = "serde")]
fn read_rows<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    use pencil_puzzles_core::serde_helpers::{BoundedText, bounded_vec};
    bounded_vec::<D, BoundedText<{ 4 * MAX_SIZE }>, MAX_SIZE>(d)
        .map(|v| v.into_iter().map(|s| s.0).collect())
}

#[cfg(feature = "serde")]
fn read_clue_list<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u32>, D::Error> {
    pencil_puzzles_core::serde_helpers::bounded_vec::<D, u32, MAX_SIZE>(d)
}

#[cfg(feature = "serde")]
impl From<Puzzle> for PuzzleData {
    fn from(p: Puzzle) -> Self {
        Self {
            regions: p.regions(),
            row_clues: p.row_clues,
            col_clues: p.col_clues,
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<PuzzleData> for Puzzle {
    type Error = ParseError;
    fn try_from(d: PuzzleData) -> Result<Self, ParseError> {
        Self::from_rows(&d.regions, d.row_clues, d.col_clues)
    }
}

/// An Aquarium puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

/// Read Aquarium documents, keeping their shared keys (F6-F8).
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
    if doc.genre().is_some_and(|g| g != "aquarium") {
        return Err(ParseError::at_line(start, "expected an aquarium puzzle"));
    }
    let params_line = doc.line_of("params");
    let mut shared = doc.take_shared()?;
    let (row_text, row_line) = doc.require_text("rows")?;
    let (col_text, col_line) = doc.require_text("cols")?;
    let regions = doc
        .take_grid("regions")?
        .ok_or_else(|| ParseError::at_line(start, "missing regions block"))?;
    // The clue counts give the grid's size; the regions must match it.
    let count = |text: &str| text.split(' ').take(MAX_SIZE + 1).count();
    let (rows, cols) = (count(&row_text), count(&col_text));
    let row_clues = read_clues(&row_text, rows, cols, "row").map_err(|e| e.or_at_line(row_line))?;
    let col_clues =
        read_clues(&col_text, cols, rows, "column").map_err(|e| e.or_at_line(col_line))?;
    let p = Puzzle::from_rows(&regions.rows, row_clues, col_clues)
        .map_err(|e| ParseError::at_line(block_line(regions.line, &e), e.message()))?;
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
    let mut writer = Writer::new("aquarium");
    writer.shared_info(&record.shared);
    writer.text("rows", &write_clues(&record.puzzle.row_clues));
    writer.text("cols", &write_clues(&record.puzzle.col_clues));
    writer.grid("regions", record.puzzle.regions());
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
