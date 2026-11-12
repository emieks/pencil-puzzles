//! Validated regions and clues, their formats, and strict shared records.
use crate::{Answer, MAX_SIZE, Mark, Params, Shape};
use pencil_puzzles_core::{
    ParseError, Position, grid, id,
    text::{self, Document, Writer},
};
use std::{collections::BTreeMap, fmt, str::FromStr};

/// The genre's text limits: rows, and bytes in a row of spaced 3-digit
/// region numbers.
pub(crate) const LIMITS: text::GridLimits = text::GridLimits::new(MAX_SIZE, 4 * MAX_SIZE);

/// Region letters in canonical order.
const LETTERS: &[u8; 52] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// The 64 characters that hold six wall bits each in an ID.
const WALL_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// A grid divided into regions, some of them with a number.
///
/// Regions are numbered from 0 in the order their first cells appear,
/// reading row by row. The clues are listed in the same order, `None` for a
/// region without a number.
///
/// ```
/// use pencil_puzzles_chocona::Puzzle;
///
/// let puzzle: Puzzle = "regions:\naab\nacc\nclues: 2 . 0".parse().unwrap();
/// assert_eq!(puzzle.regions(), [0, 0, 1, 0, 2, 2]);
/// assert_eq!(puzzle.clues(), [Some(2), None, Some(0)]);
/// assert_eq!(puzzle.id(), "chocona:2x3:Zg/2.0");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "PuzzleData", try_from = "PuzzleData"))]
pub struct Puzzle {
    shape: Shape,
    regions: Vec<u32>,
    clues: Vec<Option<u32>>,
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

/// Whether a token names a region: one letter, or a number from 1 to 999.
fn is_label(token: &str) -> bool {
    let b = token.as_bytes();
    match b {
        [c] if c.is_ascii_alphabetic() => true,
        _ => !b.is_empty() && b.len() <= 3 && b[0] != b'0' && b.iter().all(u8::is_ascii_digit),
    }
}

/// The number of the region a token names, numbering new names in order.
pub(crate) fn label<'a>(
    names: &mut BTreeMap<&'a str, u32>,
    token: &'a str,
) -> Result<u32, ParseError> {
    if !is_label(token) {
        return Err(ParseError::new(format!(
            "{token:?} is not a region; use a letter or a number from 1 to 999"
        )));
    }
    let next = names.len() as u32;
    Ok(*names.entry(token).or_insert(next))
}

/// A clue as written in text and the editor: `.` for none, or a number
/// from 0 to 999 without leading zeros.
pub(crate) fn read_clue(word: &str) -> Result<Option<u32>, ParseError> {
    if word == "." {
        return Ok(None);
    }
    (!word.is_empty()
        && word.len() <= 3
        && (word == "0" || !word.starts_with('0'))
        && word.bytes().all(|b| b.is_ascii_digit()))
    .then(|| word.parse::<u32>().ok())
    .flatten()
    .map(Some)
    .ok_or_else(|| ParseError::new(format!("{word:?} is not a clue; use a number or `.`")))
}

fn write_clue(clue: Option<u32>) -> String {
    clue.map_or(".".into(), |n| n.to_string())
}

/// Each cell's region, numbered by first appearance, and each region's
/// size. Fails if the labels do not fit the shape or a region is not
/// connected.
fn number(shape: Shape, labels: &[u32]) -> Result<(Vec<u32>, Vec<usize>), ParseError> {
    if labels.len() != shape.cells() {
        return Err(ParseError::new(format!(
            "a {shape} grid needs {} region labels",
            shape.cells()
        )));
    }
    let mut numbers = BTreeMap::new();
    let mut first = Vec::new();
    let regions: Vec<u32> = labels
        .iter()
        .enumerate()
        .map(|(i, &label)| {
            *numbers.entry(label).or_insert_with(|| {
                first.push(i);
                first.len() as u32 - 1
            })
        })
        .collect();
    let nb = grid::neighbours(shape.rows(), shape.cols());
    let sizes: Vec<usize> = (first.iter().enumerate())
        .map(|(t, &start)| grid::flood(start, |j| regions[j] == t as u32, &nb).len())
        .collect();
    if sizes.iter().sum::<usize>() != regions.len() {
        return Err(ParseError::new(
            "every region must be one group of cells joined by their sides",
        ));
    }
    Ok((regions, sizes))
}

/// Fails unless there is one clue per region, none larger than its region.
fn check_clues(sizes: &[usize], clues: &[Option<u32>]) -> Result<(), ParseError> {
    if clues.len() != sizes.len() {
        return Err(ParseError::new(format!(
            "the grid has {} regions but {} clues",
            sizes.len(),
            clues.len()
        )));
    }
    for (t, (&size, clue)) in sizes.iter().zip(clues).enumerate() {
        if let Some(clue) = clue.filter(|&n| n as usize > size) {
            return Err(ParseError::new(format!(
                "region {} has the clue {clue}, but it has only {size} cells",
                t + 1
            )));
        }
    }
    Ok(())
}

/// The shape and the cells' labels from region rows as written in text.
fn read_labels<S: AsRef<str>>(regions: &[S]) -> Result<(Shape, Vec<u32>), ParseError> {
    if regions.is_empty() || regions.len() > MAX_SIZE {
        return Err(ParseError::at_line(
            1,
            format!("Chocona needs 1 to {MAX_SIZE} rows"),
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
    Ok((shape, labels))
}

impl Puzzle {
    /// A puzzle from a region label for every cell (row by row) and a clue
    /// for every region, in the order the regions' first cells appear.
    /// Cells with the same label form a region, whatever the labels are.
    ///
    /// Fails if a list has the wrong length, a region is not connected, or
    /// a clue is larger than its region.
    pub fn new(shape: Shape, labels: &[u32], clues: Vec<Option<u32>>) -> Result<Self, ParseError> {
        let (regions, sizes) = number(shape, labels)?;
        check_clues(&sizes, &clues)?;
        Ok(Self {
            shape,
            regions,
            clues,
        })
    }

    /// Read the region rows as written in text, with a clue for every
    /// region. A cell holds a letter or a number from 1 to 999; rows with
    /// numbers above 9 separate their cells with spaces.
    pub fn from_rows<S: AsRef<str>>(
        regions: &[S],
        clues: Vec<Option<u32>>,
    ) -> Result<Self, ParseError> {
        let (shape, labels) = read_labels(regions)?;
        Self::new(shape, &labels, clues)
    }

    /// The puzzle with the same regions whose clues count the shaded cells
    /// of a complete answer. Regions without a clue keep none. It may have
    /// other solutions.
    ///
    /// # Panics
    ///
    /// Panics if the answer has another shape.
    pub fn with_clues_of(&self, answer: &Answer) -> Self {
        assert_eq!(answer.shape(), self.shape, "the answer has another shape");
        let cols = self.shape.cols();
        let mut counts = vec![0; self.clues.len()];
        for (i, &t) in self.regions.iter().enumerate() {
            if answer.is_shaded(i / cols, i % cols) {
                counts[t as usize] += 1;
            }
        }
        let clues = (self.clues.iter().zip(counts))
            .map(|(clue, count)| clue.map(|_| count))
            .collect();
        Self {
            clues,
            ..self.clone()
        }
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        self.shape
    }

    /// Each cell's region, row by row. Regions are numbered from 0 in the
    /// order their first cells appear.
    pub fn regions(&self) -> &[u32] {
        &self.regions
    }

    /// The number of regions.
    pub fn region_count(&self) -> usize {
        self.clues.len()
    }

    /// Each region's clue, `None` for a region without a number.
    pub fn clues(&self) -> &[Option<u32>] {
        &self.clues
    }

    /// The region rows, as written in text: letters for up to 52 regions,
    /// and numbers from 1 separated by spaces for more.
    pub fn region_rows(&self) -> Vec<String> {
        let letters = self.clues.len() <= LETTERS.len();
        self.regions
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

    /// The first cell of each region, reading row by row, where its number
    /// is drawn.
    pub fn clue_cells(&self) -> Vec<usize> {
        let mut first = vec![usize::MAX; self.clues.len()];
        for (i, &t) in self.regions.iter().enumerate() {
            if first[t as usize] == usize::MAX {
                first[t as usize] = i;
            }
        }
        first
    }

    /// Whether there is a wall between each pair of neighbouring cells:
    /// first between each cell and the one to its right, row by row, then
    /// between each cell and the one below it, row by row.
    fn walls(&self) -> Vec<bool> {
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let t = &self.regions;
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

    /// The puzzle's ID, such as `chocona:2x3:Zg/2.0`: the walls between
    /// regions, then the clues.
    pub fn id(&self) -> String {
        let walls: String = self
            .walls()
            .chunks(6)
            .map(|bits| {
                let value = (0..6).fold(0, |v, k| v << 1 | usize::from(bits.get(k) == Some(&true)));
                char::from(WALL_CHARS[value])
            })
            .collect();
        let digit = |n: u32| char::from_digit(n, 36).expect("below 36");
        let clues: String = (self.clues.iter())
            .map(|clue| match *clue {
                None => ".".into(),
                Some(n) if n < 36 => digit(n).to_string(),
                Some(n) => format!("-{}{}", digit(n / 36), digit(n % 36)),
            })
            .collect();
        id::join(
            "chocona",
            &self.shape.to_string(),
            &format!("{walls}/{clues}"),
        )
    }

    /// Read a puzzle ID, reporting the first wrong character.
    pub fn from_id(source: &str) -> Result<Self, ParseError> {
        let (genre, shape, desc) = id::split(source)?;
        if genre != "chocona" {
            return Err(ParseError::at_char(1, "expected a chocona ID"));
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

    /// Draw the regions for a terminal. Without an answer, each region's
    /// number is in its first cell; with one, shaded cells are `#` and cells
    /// marked unshaded are `-`.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if answer.is_some_and(|a| a.shape() != self.shape) {
            return "the answer has the wrong shape".into();
        }
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let t = &self.regions;
        let mut text = vec![String::new(); t.len()];
        match answer {
            Some(a) => {
                for (i, cell) in text.iter_mut().enumerate() {
                    *cell = match a.get(i / cols, i % cols) {
                        Mark::Shaded => "#".into(),
                        Mark::Unshaded => "-".into(),
                        Mark::Unmarked => String::new(),
                    };
                }
            }
            None => {
                for (region, i) in self.clue_cells().into_iter().enumerate() {
                    text[i] = self.clues[region].map_or(String::new(), |n| n.to_string());
                }
            }
        }
        let width = text.iter().map(String::len).max().unwrap_or(1).max(1);
        // The line of walls above row `r`; row `rows` is the bottom edge.
        let border = |r: usize| -> String {
            let mut line = String::new();
            for c in 0..cols {
                let wall = r == 0 || r == rows || t[(r - 1) * cols + c] != t[r * cols + c];
                line.push('+');
                line.push_str(&(if wall { "-" } else { " " }).repeat(width));
            }
            line.push('+');
            line
        };
        let mut lines = Vec::new();
        for r in 0..rows {
            lines.push(border(r));
            let mut line = String::new();
            for c in 0..cols {
                let wall = c == 0 || t[r * cols + c - 1] != t[r * cols + c];
                line.push(if wall { '|' } else { ' ' });
                line.push_str(&format!("{:<width$}", text[r * cols + c]));
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
    let Some((wall_part, clue_part)) = desc.split_once('/') else {
        return Err((
            desc.len(),
            "the description needs walls and clues, separated by `/`".into(),
        ));
    };
    let across = rows * (cols - 1);
    let count = across + (rows - 1) * cols;
    let mut walls = Vec::with_capacity(count + 5);
    for (at, b) in wall_part.bytes().enumerate() {
        if walls.len() >= count {
            return Err((at, "there are too many walls".into()));
        }
        let Some(value) = WALL_CHARS.iter().position(|&w| w == b) else {
            return Err((at, "use A-Z, a-z, 0-9, `-` or `_`".into()));
        };
        walls.extend((0..6).rev().map(|k| value >> k & 1 == 1));
        if walls[count.min(walls.len())..].contains(&true) {
            return Err((at, "the unused wall bits must be 0".into()));
        }
    }
    if walls.len() < count {
        return Err((wall_part.len(), "there are too few walls".into()));
    }
    // Cells without a wall between them share a region.
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
                    wall(i, j) / 6,
                    "a wall may not stand inside a region".into(),
                ));
            }
        }
    }
    let regions = next as usize;
    let start = wall_part.len() + 1;
    let digit = |b: u8, at: usize| match b {
        b'0'..=b'9' | b'a'..=b'z' => Ok(char::from(b).to_digit(36).expect("base 36 digit")),
        _ => Err((
            start + at,
            "use `.`, 0-9, a-z, or `-` and two of them".into(),
        )),
    };
    let bytes = clue_part.as_bytes();
    let mut clues = Vec::with_capacity(regions);
    let mut at = 0;
    while at < bytes.len() {
        if clues.len() == regions {
            return Err((start + at, "there are more clues than regions".into()));
        }
        match bytes[at] {
            b'.' => {
                clues.push(None);
                at += 1;
            }
            b'-' => {
                let (Some(&high), Some(&low)) = (bytes.get(at + 1), bytes.get(at + 2)) else {
                    return Err((start + bytes.len(), "`-` needs two digits after it".into()));
                };
                let n = digit(high, at + 1)? * 36 + digit(low, at + 2)?;
                if n < 36 {
                    return Err((start + at, "clues below 36 take one character".into()));
                }
                clues.push(Some(n));
                at += 3;
            }
            b => {
                clues.push(Some(digit(b, at)?));
                at += 1;
            }
        }
    }
    if clues.len() != regions {
        return Err((
            start + bytes.len(),
            format!("there are {regions} regions but only {} clues", clues.len()),
        ));
    }
    Puzzle::new(shape, &labels, clues).map_err(|e| (start, e.message().to_string()))
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PuzzleData {
    #[serde(deserialize_with = "read_rows")]
    regions: Vec<String>,
    #[serde(deserialize_with = "read_clue_list")]
    clues: Vec<Option<u32>>,
}

#[cfg(feature = "serde")]
fn read_rows<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    use pencil_puzzles_core::serde_helpers::{BoundedText, bounded_vec};
    bounded_vec::<D, BoundedText<{ 4 * MAX_SIZE }>, MAX_SIZE>(d)
        .map(|v| v.into_iter().map(|s| s.0).collect())
}

#[cfg(feature = "serde")]
fn read_clue_list<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Option<u32>>, D::Error> {
    pencil_puzzles_core::serde_helpers::bounded_vec::<D, Option<u32>, { MAX_SIZE * MAX_SIZE }>(d)
}

#[cfg(feature = "serde")]
impl From<Puzzle> for PuzzleData {
    fn from(p: Puzzle) -> Self {
        Self {
            regions: p.region_rows(),
            clues: p.clues,
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<PuzzleData> for Puzzle {
    type Error = ParseError;
    fn try_from(d: PuzzleData) -> Result<Self, ParseError> {
        Self::from_rows(&d.regions, d.clues)
    }
}

/// A Chocona puzzle as read from text, with its solution and shared keys.
pub type Record = pencil_puzzles_core::Record<Puzzle, Answer>;

/// Read Chocona documents, keeping their shared keys (F6-F8).
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
    if doc.genre().is_some_and(|g| g != "chocona") {
        return Err(ParseError::at_line(start, "expected a chocona puzzle"));
    }
    let params_line = doc.line_of("params");
    let mut shared = doc.take_shared()?;
    let regions = doc
        .take_grid("regions")?
        .ok_or_else(|| ParseError::at_line(start, "missing regions block"))?;
    let (clue_text, clue_line) = doc.require_text("clues")?;
    let clues = clue_text
        .split(' ')
        .take(MAX_SIZE * MAX_SIZE + 1)
        .map(read_clue)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.or_at_line(clue_line))?;
    let in_block = |e: ParseError| ParseError::at_line(block_line(regions.line, &e), e.message());
    let (shape, labels) = read_labels(&regions.rows).map_err(in_block)?;
    let (cells, sizes) = number(shape, &labels).map_err(in_block)?;
    check_clues(&sizes, &clues).map_err(|e| e.or_at_line(clue_line))?;
    let p = Puzzle {
        shape,
        regions: cells,
        clues,
    };
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
    let mut writer = Writer::new("chocona");
    writer.shared_info(&record.shared);
    writer.grid("regions", record.puzzle.region_rows());
    let clues: Vec<String> = record.puzzle.clues.iter().map(|&c| write_clue(c)).collect();
    writer.text("clues", &clues.join(" "));
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
