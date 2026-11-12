//! A puzzle's shape and the generator's parameters.

use std::fmt;
use std::str::FromStr;

use pencil_puzzles_core::ParseError;

/// The largest number of rows or columns a puzzle may have.
pub const MAX_SIZE: usize = 15;

/// A puzzle's number of rows and columns, written like `5x7` (rows first).
///
/// ```
/// use pencil_puzzles_chocona::Shape;
///
/// let shape: Shape = "5x7".parse().unwrap();
/// assert_eq!((shape.rows(), shape.cols()), (5, 7));
/// assert_eq!(shape.to_string(), "5x7");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
pub struct Shape {
    rows: usize,
    cols: usize,
}

impl Shape {
    /// A shape, if both sizes are between 1 and [`MAX_SIZE`].
    pub fn new(rows: usize, cols: usize) -> Result<Self, ParseError> {
        if !(1..=MAX_SIZE).contains(&rows) || !(1..=MAX_SIZE).contains(&cols) {
            return Err(ParseError::new(format!(
                "a {rows}x{cols} grid isn't allowed; rows and columns must be between 1 and {MAX_SIZE}"
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
            |at| ParseError::at_char(at, format!("{s:?} is not a shape; write it like `5x5`"));
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
            if !(1..=MAX_SIZE).contains(&value) {
                return Err(ParseError::at_char(
                    offset,
                    format!("sizes must be between 1 and {MAX_SIZE}"),
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
/// Chocona has no generator settings beyond the shape, so the parameters
/// are written the same way, like `5x5`.
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
        for good in ["1x1", "5x7", "15x15"] {
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
            "0x5",
            "16x1",
            "5x5x5",
            "99999999999x1",
        ] {
            assert!(bad.parse::<Shape>().is_err(), "{bad:?}");
        }
    }
}
