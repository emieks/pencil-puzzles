//! The [`Puzzle`] type: the clues of a Kakurasu puzzle, and its formats.

use std::fmt;
use std::str::FromStr;

use pencil_puzzles_core::text::{self, Document, Writer};
use pencil_puzzles_core::{ParseError, Record, id};

use crate::{Answer, MAX_SIZE, Mark, Shape};

/// The genre id.
pub(crate) const ID: &str = "kakurasu";

/// The clues of a Kakurasu puzzle.
///
/// A row's clue is the total of the column numbers of its shaded cells, and
/// a column's clue is the total of the row numbers of its shaded cells, with
/// rows and columns counted from 1.
///
/// The text form (see `docs/genres/kakurasu.md`):
///
/// ```
/// use pencil_puzzles_kakurasu::Puzzle;
///
/// let puzzle: Puzzle = "genre: kakurasu\nrows: 4 5 4 10\ncols: 5 6 7 7".parse().unwrap();
/// assert_eq!(puzzle.row_clues(), [4, 5, 4, 10]);
/// assert_eq!(puzzle.id(), "kakurasu:4x4:4.5.4.10/5.6.7.7");
/// ```
///
/// With the `serde` feature, a puzzle is an object with `row_clues` and
/// `col_clues` lists.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "PuzzleClues"))]
pub struct Puzzle {
    row_clues: Vec<u32>,
    col_clues: Vec<u32>,
}

impl Puzzle {
    /// A puzzle from its row clues (top to bottom) and column clues (left to
    /// right).
    ///
    /// Fails if there are too few or too many rows or columns, or a clue is
    /// larger than its row or column could ever add up to.
    pub fn new(row_clues: Vec<u32>, col_clues: Vec<u32>) -> Result<Self, ParseError> {
        let shape = Shape::new(row_clues.len(), col_clues.len())?;
        let lines = (row_clues.iter().enumerate())
            .map(|(i, &clue)| (format!("row {}", i + 1), clue, triangle(shape.cols())))
            .chain(
                (col_clues.iter().enumerate())
                    .map(|(i, &clue)| (format!("column {}", i + 1), clue, triangle(shape.rows()))),
            );
        for (line, clue, max) in lines {
            if clue > max {
                return Err(ParseError::new(format!(
                    "{line} has the clue {clue}, but the largest possible there is {max}"
                )));
            }
        }
        Ok(Self {
            row_clues,
            col_clues,
        })
    }

    /// The puzzle whose clues match a complete answer. It may have other
    /// solutions too; [`solve`](crate::solve) tells.
    ///
    /// # Panics
    ///
    /// Panics if the answer is not complete.
    pub fn from_solution(solution: &Answer) -> Self {
        assert!(solution.is_complete(), "the answer must be complete");
        let shape = solution.shape();
        Self {
            row_clues: (0..shape.rows())
                .map(|row| solution.row_total(row))
                .collect(),
            col_clues: (0..shape.cols())
                .map(|col| solution.col_total(col))
                .collect(),
        }
    }

    /// The puzzle's shape.
    pub fn shape(&self) -> Shape {
        Shape::new(self.rows(), self.cols()).expect("checked when the puzzle was made")
    }

    /// The number of rows.
    pub fn rows(&self) -> usize {
        self.row_clues.len()
    }

    /// The number of columns.
    pub fn cols(&self) -> usize {
        self.col_clues.len()
    }

    /// The row clues, from top to bottom.
    pub fn row_clues(&self) -> &[u32] {
        &self.row_clues
    }

    /// The column clues, from left to right.
    pub fn col_clues(&self) -> &[u32] {
        &self.col_clues
    }

    /// Whether `answer` is complete, has the right shape and matches every
    /// clue.
    pub fn is_solved_by(&self, answer: &Answer) -> bool {
        answer.shape() == self.shape()
            && answer.is_complete()
            && (0..self.rows()).all(|row| answer.row_total(row) == self.row_clues[row])
            && (0..self.cols()).all(|col| answer.col_total(col) == self.col_clues[col])
    }

    /// The puzzle ID, like `kakurasu:5x5:10.11.7.5.4/4.3.4.14.3`: the shape,
    /// then the row clues and the column clues (F9).
    pub fn id(&self) -> String {
        let join = |clues: &[u32]| {
            (clues.iter().map(u32::to_string))
                .collect::<Vec<_>>()
                .join(".")
        };
        id::join(
            ID,
            &self.shape().to_string(),
            &format!("{}/{}", join(&self.row_clues), join(&self.col_clues)),
        )
    }

    /// Reads a puzzle ID. Errors give the position of the first wrong
    /// character (F11).
    pub fn from_id(puzzle_id: &str) -> Result<Self, ParseError> {
        let (genre, shape_text, description) = id::split(puzzle_id)?;
        if genre != ID {
            return Err(ParseError::at_char(
                1,
                format!("this is a {genre} ID, not a {ID} ID"),
            ));
        }
        let shape_start = genre.len() + 2;
        let shape: Shape = shape_text.parse().map_err(|error: ParseError| {
            ParseError::at_char(
                shape_start
                    + match error.position() {
                        pencil_puzzles_core::Position::Char(n) => n - 1,
                        _ => 0,
                    },
                error.message().to_owned(),
            )
        })?;
        let description_start = shape_start + shape_text.len() + 1;
        let Some((rows, cols)) = description.split_once('/') else {
            return Err(ParseError::at_char(
                description_start,
                "the description needs the row clues and the column clues, separated by `/`",
            ));
        };
        let clues = |part: &str, start: usize, count: usize, name: &str, max: u32| {
            let mut clues = Vec::with_capacity(count);
            let mut position = start;
            for word in part.split('.') {
                if let Some(bad) = word.bytes().position(|b| !b.is_ascii_digit()) {
                    return Err(ParseError::at_char(
                        position + bad,
                        "a clue contains only decimal digits",
                    ));
                }

                let clue = (!word.is_empty()
                    && word.len() <= 4
                    && word.bytes().all(|b| b.is_ascii_digit()))
                .then(|| word.parse::<u32>().ok())
                .flatten()
                .ok_or_else(|| ParseError::at_char(position, format!("{word:?} is not a clue")))?;
                if clue > max {
                    return Err(ParseError::at_char(
                        position,
                        format!("a {name} clue must be at most {max}"),
                    ));
                }
                clues.push(clue);
                position += word.len() + 1;
            }
            if clues.len() != count {
                return Err(ParseError::at_char(
                    start,
                    format!(
                        "a {shape} puzzle needs {count} {name} clues, not {}",
                        clues.len()
                    ),
                ));
            }
            Ok(clues)
        };
        let row_clues = clues(
            rows,
            description_start,
            shape.rows(),
            "row",
            triangle(shape.cols()),
        )?;
        let col_clues = clues(
            cols,
            description_start + rows.len() + 1,
            shape.cols(),
            "column",
            triangle(shape.rows()),
        )?;
        Self::new(row_clues, col_clues)
            .map_err(|error| ParseError::at_char(description_start, error.message().to_owned()))
    }

    /// Draws the puzzle for a terminal: the cell values along the top and
    /// left, the clues along the right and bottom, and the answer's marks if
    /// one is given.
    ///
    /// ```
    /// use pencil_puzzles_kakurasu::Puzzle;
    ///
    /// let puzzle: Puzzle = "rows: 1 2\ncols: 1 2".parse().unwrap();
    /// let solution = "#-\n-#".parse().unwrap();
    /// assert_eq!(
    ///     puzzle.render(Some(&solution)),
    ///     "   1 2\n1  # -  1\n2  - #  2\n   1 2"
    /// );
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the answer is not the same shape as the puzzle.
    pub fn render(&self, answer: Option<&Answer>) -> String {
        if let Some(answer) = answer {
            assert_eq!(
                answer.shape(),
                self.shape(),
                "the answer's shape differs from the puzzle's"
            );
        }
        let width = |n: usize| n.to_string().len();
        let cell_width = (self.col_clues.iter().map(|&clue| width(clue as usize)))
            .chain([width(self.cols())])
            .max()
            .unwrap_or(1);
        let label_width = width(self.rows());
        let numbers = |values: &mut dyn Iterator<Item = usize>| {
            values
                .map(|value| format!("{value:>cell_width$}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut lines = vec![format!(
            "{:label_width$}  {}",
            "",
            numbers(&mut (1..=self.cols()))
        )];
        for row in 0..self.rows() {
            let cells = (0..self.cols())
                .map(|col| {
                    let mark = answer.map_or(Mark::Unmarked, |answer| answer.get(row, col));
                    format!("{:>cell_width$}", mark.to_char())
                })
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(format!(
                "{:>label_width$}  {cells}  {}",
                row + 1,
                self.row_clues[row]
            ));
        }
        lines.push(format!(
            "{:label_width$}  {}",
            "",
            numbers(&mut self.col_clues.iter().map(|&clue| clue as usize))
        ));
        (lines.iter().map(|line| line.trim_end()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The largest possible clue for a line of `len` cells: 1 + 2 + … + len.
fn triangle(len: usize) -> u32 {
    (len * (len + 1) / 2) as u32
}

/// Reads every puzzle in a text, with the shared keys of each (F2, F6).
///
/// A puzzle may leave out its `genre:` line only if it is the only puzzle in
/// the text.
pub fn parse_records(text: &str) -> Result<Vec<Record<Puzzle, Answer>>, ParseError> {
    let documents = text::parse(text)?;
    if documents.is_empty() {
        return Err(ParseError::new("there is no puzzle in the text"));
    }
    let several = documents.len() > 1;
    documents
        .into_iter()
        .map(|document| {
            if several && document.genre().is_none() {
                return Err(ParseError::at_line(
                    document.start_line(),
                    "each puzzle in a file with several puzzles starts with its `genre:` line",
                ));
            }
            read_document(document)
        })
        .collect()
}

/// Reads one puzzle's document.
pub(crate) fn read_document(mut document: Document) -> Result<Record<Puzzle, Answer>, ParseError> {
    let start = document.start_line();
    if let Some(genre) = document.genre()
        && genre != ID
    {
        return Err(ParseError::at_line(
            start,
            format!("this is a {genre} puzzle, not a {ID} puzzle"),
        ));
    }
    let params_line = document.line_of("params");
    let mut shared = document.take_shared()?;
    let clues = |document: &mut Document, key: &str| -> Result<(Vec<u32>, usize), ParseError> {
        let (value, line) = document.require_text(key)?;
        let words: Vec<&str> = value.split_whitespace().take(MAX_SIZE + 1).collect();
        if words.len() > MAX_SIZE {
            return Err(ParseError::at_line(
                line,
                format!("a puzzle can have at most {MAX_SIZE} {key}"),
            ));
        }
        let clues = words
            .iter()
            .map(|word| {
                (word.len() <= 4 && word.bytes().all(|b| b.is_ascii_digit()))
                    .then(|| word.parse().ok())
                    .flatten()
                    .ok_or_else(|| ParseError::at_line(line, format!("{word:?} is not a clue")))
            })
            .collect::<Result<Vec<u32>, ParseError>>()?;
        Ok((clues, line))
    };
    let (row_clues, row_line) = clues(&mut document, "rows")?;
    let (col_clues, col_line) = clues(&mut document, "cols")?;
    document.finish()?;
    let invalid_line = if row_clues.iter().any(|&n| n > triangle(col_clues.len())) {
        row_line
    } else {
        col_line
    };
    let puzzle =
        Puzzle::new(row_clues, col_clues).map_err(|error| error.or_at_line(invalid_line))?;

    let solution = match shared.solution.take() {
        None => None,
        Some(block) => {
            let answer = Answer::from_rows(&block.rows).map_err(|error| {
                ParseError::at_line(
                    block.line
                        + match error.position() {
                            pencil_puzzles_core::Position::Line(n) => n,
                            _ => 1,
                        },
                    error.message(),
                )
            })?;
            if answer.shape() != puzzle.shape() || !answer.is_complete() {
                return Err(ParseError::at_line(
                    block.line,
                    format!("the solution must be a complete {} grid", puzzle.shape()),
                ));
            }
            Some(answer)
        }
    };
    if let Some(params) = &shared.params {
        params
            .parse::<crate::Params>()
            .map_err(|error| ParseError::at_line(params_line.unwrap_or(start), error.message()))?;
    }
    let mut record = Record::new(puzzle);
    record.solution = solution;
    record.shared = shared;
    Ok(record)
}

/// Writes a puzzle with its shared keys, in canonical form (F7, F8).
pub fn write_record(record: &Record<Puzzle, Answer>) -> String {
    let mut writer = Writer::new(ID);
    writer.shared_info(&record.shared);
    let join = |clues: &[u32]| {
        (clues.iter().map(u32::to_string))
            .collect::<Vec<_>>()
            .join(" ")
    };
    writer.text("rows", &join(&record.puzzle.row_clues));
    writer.text("cols", &join(&record.puzzle.col_clues));
    let mut shared = record.shared.clone();
    shared.solution = record.solution.as_ref().map(|solution| text::GridBlock {
        rows: solution.rows(),
        line: 0,
    });
    writer.shared_record(&shared);
    writer.finish()
}

impl fmt::Display for Puzzle {
    /// The canonical text, without shared keys.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&write_record(&Record::new(self.clone())))
    }
}

impl FromStr for Puzzle {
    type Err = ParseError;

    /// Reads one puzzle, ignoring any shared keys. Use [`parse_records`] to
    /// keep them.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut records = parse_records(s)?;
        if records.len() > 1 {
            return Err(ParseError::new("the text has more than one puzzle"));
        }
        Ok(records.remove(0).puzzle)
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PuzzleClues {
    #[serde(deserialize_with = "crate::json::bounded")]
    row_clues: Vec<u32>,
    #[serde(deserialize_with = "crate::json::bounded")]
    col_clues: Vec<u32>,
}

#[cfg(feature = "serde")]
impl TryFrom<PuzzleClues> for Puzzle {
    type Error = ParseError;

    fn try_from(clues: PuzzleClues) -> Result<Self, Self::Error> {
        Self::new(clues.row_clues, clues.col_clues)
    }
}
