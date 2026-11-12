//! A puzzle's shape and the generator's parameters.

use std::fmt;
use std::str::FromStr;

use pencil_puzzles_core::ParseError;

/// The largest number of rows or columns a puzzle may have.
pub const MAX_SIZE: usize = 16;

/// A puzzle's number of rows and columns, written like `6x8` (rows first).
/// Both are even, so that every line can hold as many 0s as 1s.
///
/// ```
/// use pencil_puzzles_binairo::Shape;
///
/// let shape: Shape = "6x8".parse().unwrap();
/// assert_eq!((shape.rows(), shape.cols()), (6, 8));
/// assert_eq!(shape.to_string(), "6x8");
/// assert!("5x6".parse::<Shape>().is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
pub struct Shape {
    rows: usize,
    cols: usize,
}

impl Shape {
    /// A shape, if both sizes are even and between 2 and [`MAX_SIZE`], and
    /// the grid has enough different lines for R3: a side of 2 needs the
    /// other side 2, a side of 4 at most 6, and a side of 6 at most 14.
    pub fn new(rows: usize, cols: usize) -> Result<Self, ParseError> {
        let fits = |n: usize| (2..=MAX_SIZE).contains(&n) && n.is_multiple_of(2);
        if !fits(rows) || !fits(cols) {
            return Err(ParseError::new(format!(
                "a {rows}x{cols} grid isn't allowed; rows and columns must be even numbers from 2 to {MAX_SIZE}"
            )));
        }
        // R3 needs as many different valid lines as there are parallel
        // lines: 2, 6 and 14 lines of 2, 4 and 6 cells, and more for longer
        // lines than any grid has.
        let different = |n: usize| match n {
            2 => 2,
            4 => 6,
            6 => 14,
            _ => usize::MAX,
        };
        if rows > different(cols) || cols > different(rows) {
            return Err(ParseError::new(format!(
                "a {rows}x{cols} grid isn't allowed: its lines can't all be different"
            )));
        }
        Ok(Self { rows, cols })
    }

    /// The number of rows.
    pub fn rows(self) -> usize {
        self.rows
    }

    /// The number of columns.
    pub fn cols(self) -> usize {
        self.cols
    }

    /// The number of cells.
    pub fn cells(self) -> usize {
        self.rows * self.cols
    }
}

impl Default for Shape {
    /// 10×10.
    fn default() -> Self {
        Self { rows: 10, cols: 10 }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.rows, self.cols)
    }
}

impl FromStr for Shape {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid =
            |at| ParseError::at_char(at, format!("{s:?} is not a shape; write it like `6x6`"));
        let Some((rows, cols)) = s.split_once('x') else {
            let at = s
                .chars()
                .position(|c| !c.is_ascii_digit())
                .unwrap_or(s.len());
            return Err(invalid(at + 1));
        };
        let size = |part: &str, offset: usize| -> Result<usize, ParseError> {
            if part.is_empty() {
                return Err(invalid(offset));
            }
            if let Some(bad) = part.chars().position(|c| !c.is_ascii_digit()) {
                return Err(invalid(offset + bad));
            }
            if part.len() > 1 && part.starts_with('0') {
                return Err(invalid(offset));
            }
            if part.len() > 3 {
                return Err(invalid(offset));
            }
            let value: usize = part.parse().map_err(|_| invalid(offset))?;
            if !(2..=MAX_SIZE).contains(&value) || !value.is_multiple_of(2) {
                return Err(ParseError::at_char(
                    offset,
                    format!("sizes must be even numbers from 2 to {MAX_SIZE}"),
                ));
            }
            Ok(value)
        };
        Self::new(size(rows, 1)?, size(cols, rows.len() + 2)?)
    }
}

impl From<Shape> for String {
    fn from(shape: Shape) -> Self {
        shape.to_string()
    }
}

impl TryFrom<String> for Shape {
    type Error = ParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// What the generator is given besides the difficulty and seed: the shape.
///
/// Binairo has no generator settings beyond the shape, so the parameters
/// are written the same way, like `6x6`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
#[non_exhaustive]
pub struct Params {
    /// The shape of the puzzles to generate.
    pub shape: Shape,
}

impl Params {
    /// Parameters for puzzles of the given shape.
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self::new)
    }
}

impl From<Params> for String {
    fn from(params: Params) -> Self {
        params.to_string()
    }
}

impl TryFrom<String> for Params {
    type Error = ParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_strict() {
        for good in ["2x2", "6x8", "16x16"] {
            assert_eq!(good.parse::<Shape>().unwrap().to_string(), good);
        }
        for bad in [
            "",
            "5",
            "5x",
            "x5",
            "05x5",
            "5X5",
            " 5x5",
            "0x6",
            "1x1",
            "5x6",
            "6x7",
            "18x2",
            "2x4",
            "8x4",
            "6x16",
            "5x5x5",
            "99999999999x1",
        ] {
            assert!(bad.parse::<Shape>().is_err(), "{bad:?}");
        }
    }
}
