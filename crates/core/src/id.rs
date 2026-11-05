//! Puzzle IDs: short, one-line forms of puzzles for links and sharing
//! (F9–F12 in the guidelines).
//!
//! ```
//! use pencil_puzzles_core::id;
//!
//! let parts = id::split("kakurasu:2x2:1.2/1.2").unwrap();
//! assert_eq!(parts, ("kakurasu", "2x2", "1.2/1.2"));
//! assert_eq!(id::join("kakurasu", "2x2", "1.2/1.2"), "kakurasu:2x2:1.2/1.2");
//! ```

use crate::ParseError;

/// The longest ID allowed (F9).
pub const MAX_LENGTH: usize = 4096;

/// Whether `c` may appear in an ID's shape or description (F9).
pub fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~' | ',' | '/')
}

/// Splits an ID into its genre id, shape and description, checking its
/// length and characters. Errors give the position of the first wrong
/// character, counted from 1 (F11).
pub fn split(id: &str) -> Result<(&str, &str, &str), ParseError> {
    if id.len() > MAX_LENGTH {
        return Err(ParseError::at_char(
            MAX_LENGTH + 1,
            format!("a puzzle ID can be at most {MAX_LENGTH} characters long"),
        ));
    }
    let mut parts = id.splitn(3, ':');
    let (Some(genre), Some(shape), Some(description)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(ParseError::at_char(
            id.chars().count() + 1,
            "a puzzle ID has the form `genre:shape:description`",
        ));
    };
    if genre.is_empty() {
        return Err(ParseError::at_char(1, "the ID is missing its genre"));
    }
    if let Some((index, _)) = genre
        .chars()
        .enumerate()
        .find(|&(i, c)| !(c.is_ascii_lowercase() || (i > 0 && (c.is_ascii_digit() || c == '-'))))
    {
        return Err(ParseError::at_char(
            index + 1,
            "a genre id uses lowercase ASCII words separated by hyphens",
        ));
    }
    let shape_start = genre.chars().count() + 2;
    let description_start = shape_start + shape.chars().count() + 1;
    for (start, part, name) in [
        (shape_start, shape, "shape"),
        (description_start, description, "description"),
    ] {
        if part.is_empty() {
            return Err(ParseError::at_char(
                start,
                format!("the ID is missing its {name}"),
            ));
        }
        if let Some((offset, c)) = part.chars().enumerate().find(|&(_, c)| !is_id_char(c)) {
            return Err(ParseError::at_char(
                start + offset,
                format!("{c:?} can't be used in a puzzle ID"),
            ));
        }
    }
    Ok((genre, shape, description))
}

/// Joins a genre id, shape and description into an ID.
pub fn join(genre: &str, shape: &str, description: &str) -> String {
    debug_assert!(shape.chars().chain(description.chars()).all(is_id_char));
    format!("{genre}:{shape}:{description}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Position;

    #[test]
    fn errors_point_at_the_character() {
        let position = |id: &str| split(id).unwrap_err().position();
        assert_eq!(position("k:5x5:1 2"), Position::Char(8));
        assert_eq!(position("k::1"), Position::Char(3));
        assert_eq!(position(":5x5:1"), Position::Char(1));
        assert_eq!(position("k:5x5"), Position::Char(6));
        assert_eq!(
            position(&"k:1:".repeat(2000)),
            Position::Char(MAX_LENGTH + 1)
        );
    }
}
