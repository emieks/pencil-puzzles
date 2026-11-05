//! The error type for reading puzzles, answers and parameters.

use std::error::Error;
use std::fmt;

/// Where in the input a [`ParseError`] was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Position {
    /// No particular place.
    None,
    /// A line of text, counted from 1.
    Line(usize),
    /// A character of a one-line input such as a puzzle ID, counted from 1.
    Char(usize),
}

/// An error from reading text, a puzzle ID or JSON, saying what is wrong and
/// where (G7).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParseError {
    position: Position,
    message: String,
}

impl ParseError {
    /// An error with no particular position.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            position: Position::None,
            message: message.into(),
        }
    }

    /// An error on a line, counted from 1.
    pub fn at_line(line: usize, message: impl Into<String>) -> Self {
        Self {
            position: Position::Line(line),
            message: message.into(),
        }
    }

    /// An error at a character, counted from 1.
    pub fn at_char(position: usize, message: impl Into<String>) -> Self {
        Self {
            position: Position::Char(position),
            message: message.into(),
        }
    }

    /// Where the error is.
    pub fn position(&self) -> Position {
        self.position
    }

    /// What is wrong, without the position.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The same error, placed on `line` if it has no position yet.
    pub fn or_at_line(self, line: usize) -> Self {
        match self.position {
            Position::None => Self::at_line(line, self.message),
            _ => self,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.position {
            Position::None => f.write_str(&self.message),
            Position::Line(line) => write!(f, "line {line}: {}", self.message),
            Position::Char(position) => write!(f, "character {position}: {}", self.message),
        }
    }
}

impl Error for ParseError {}
