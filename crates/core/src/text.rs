//! The text format shared by every genre (F1–F8 in the guidelines).
//!
//! ```text
//! genre: kakurasu
//! # A comment.
//! rows: 10 11 7 5 4
//! cols: 4 3 4 14 3
//! solution:
//! -##-#
//! -#-##
//! ```
//!
//! [`parse`] splits text into [`Document`]s, one per puzzle, and each genre
//! reads its own keys from them. [`Writer`] writes the same format.

use crate::{Difficulty, ParseError};

/// One puzzle's lines, read but not yet interpreted by its genre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    genre: Option<String>,
    start_line: usize,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    key: String,
    value: Value,
    line: usize,
    taken: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Text(String),
    Grid(Vec<String>),
}

/// A grid block, as read from text: its rows and the line of its `key:`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GridBlock {
    /// The rows, top to bottom.
    pub rows: Vec<String>,
    /// The line of the block's `key:`, counted from 1.
    pub line: usize,
}

/// Allocation limits for each grid block, applied before copying a row.
#[derive(Clone, Copy, Debug)]
pub struct GridLimits {
    /// Maximum rows per block (including solution blocks).
    pub rows: usize,
    /// Maximum bytes in one row, including token separators.
    pub row_bytes: usize,
}
impl GridLimits {
    /// Limits chosen by the genre before parsing untrusted text.
    pub const fn new(rows: usize, row_bytes: usize) -> Self {
        Self { rows, row_bytes }
    }
}
impl Default for GridLimits {
    fn default() -> Self {
        Self::new(32, 128)
    }
}
/// Parse with one genre's grid limits, including records without a genre line.
pub fn parse_with_limits(text: &str, limits: GridLimits) -> Result<Vec<Document>, ParseError> {
    parse_with_genre_limits(text, |_| limits)
}
/// Parse mixed-genre records, selecting limits as each genre header is read.
/// Unknown headers use the caller's fallback. Metadata lines are capped at
/// 64 KiB and keys at 64 bytes; each document allows at most 128 keys.
pub fn parse_with_genre_limits(
    text: &str,
    mut limits_for: impl FnMut(Option<&str>) -> GridLimits,
) -> Result<Vec<Document>, ParseError> {
    parse_inner(text, &mut limits_for)
}
/// Splits text into documents, one per puzzle (F1–F4).
///
/// Each document starts at a `genre:` line. Text before the first `genre:`
/// line, if there is any, forms a document without a genre, which a reader
/// may accept when it already knows the genre (F2).
pub fn parse(text: &str) -> Result<Vec<Document>, ParseError> {
    parse_with_limits(text, GridLimits::default())
}
fn parse_inner(
    text: &str,
    limits_for: &mut impl FnMut(Option<&str>) -> GridLimits,
) -> Result<Vec<Document>, ParseError> {
    let mut limits = limits_for(None);
    let mut documents = Vec::new();
    let mut current: Option<Document> = None;
    let mut block: Option<(String, Vec<String>, usize)> = None;

    for (index, raw) in text.split('\n').enumerate() {
        let number = index + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);

        if line.len() > 65536 {
            return Err(ParseError::at_line(number, "a text line exceeds 64 KiB"));
        }
        if let Some((_, rows, _)) = &mut block {
            if !line.trim().is_empty() && !line.contains(':') {
                if line != line.trim() {
                    return Err(ParseError::at_line(
                        number,
                        "grid rows can't start or end with spaces",
                    ));
                }
                if rows.len() >= limits.rows || line.len() > limits.row_bytes {
                    return Err(ParseError::at_line(
                        number,
                        "grid block exceeds this genre's row or width limit",
                    ));
                }
                rows.push(line.to_owned());
                continue;
            }
            let (key, rows, start) = block.take().expect("checked above");
            add(&mut current, key, Value::Grid(rows), start)?;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            return Err(ParseError::at_line(
                number,
                format!("expected `key: value`, found {trimmed:?}"),
            ));
        };
        if key.len() > 64 || !is_key(key) {
            return Err(ParseError::at_line(
                number,
                format!(
                    "{key:?} is not a valid key; keys are lowercase letters, digits and `_`, starting with a letter"
                ),
            ));
        }
        let value = value.trim();
        if key == "genre" {
            if value.is_empty() {
                return Err(ParseError::at_line(number, "`genre:` needs a genre id"));
            }
            if value.len() > 64 {
                return Err(ParseError::at_line(number, "genre id exceeds 64 bytes"));
            }
            limits = limits_for(Some(value));
            documents.extend(current.take());
            current = Some(Document {
                genre: Some(value.to_owned()),
                start_line: number,
                entries: Vec::new(),
            });
        } else if value.is_empty() {
            block = Some((key.to_owned(), Vec::new(), number));
        } else {
            add(
                &mut current,
                key.to_owned(),
                Value::Text(value.to_owned()),
                number,
            )?;
        }
    }
    if let Some((key, rows, start)) = block {
        add(&mut current, key, Value::Grid(rows), start)?;
    }
    documents.extend(current);
    Ok(documents)
}

fn add(
    current: &mut Option<Document>,
    key: String,
    value: Value,
    line: usize,
) -> Result<(), ParseError> {
    let document = current.get_or_insert_with(|| Document {
        genre: None,
        start_line: line,
        entries: Vec::new(),
    });
    if document.entries.iter().any(|entry| entry.key == key) {
        return Err(ParseError::at_line(
            line,
            format!("more than one `{key}:` line"),
        ));
    }
    if document.entries.len() >= 128 {
        return Err(ParseError::at_line(line, "too many keys in one puzzle"));
    }
    document.entries.push(Entry {
        key,
        value,
        line,
        taken: false,
    });
    Ok(())
}

/// Whether `key` is a valid key: `[a-z][a-z0-9_]*` (F3).
pub fn is_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl Document {
    /// The genre named on the `genre:` line, if there was one.
    pub fn genre(&self) -> Option<&str> {
        self.genre.as_deref()
    }

    /// Normalize a known id or alias before passing the document to its genre.
    pub fn set_genre(&mut self, genre: &str) {
        self.genre = Some(genre.to_owned());
    }

    /// The document's first line, counted from 1.
    pub fn start_line(&self) -> usize {
        self.start_line
    }

    /// The source line of a key, if present, without taking its value.
    pub fn line_of(&self, key: &str) -> Option<usize> {
        self.entries.iter().find(|e| e.key == key).map(|e| e.line)
    }

    /// Takes the value of `key: value`, with its line, if the key is there.
    pub fn take_text(&mut self, key: &str) -> Result<Option<(String, usize)>, ParseError> {
        let Some(entry) = self.entry(key) else {
            return Ok(None);
        };
        match std::mem::replace(&mut entry.value, Value::Text(String::new())) {
            Value::Text(text) => Ok(Some((text, entry.line))),
            Value::Grid(_) => Err(ParseError::at_line(
                entry.line,
                format!("`{key}:` needs a value on the same line"),
            )),
        }
    }

    /// Takes a grid block, if the key is there.
    pub fn take_grid(&mut self, key: &str) -> Result<Option<GridBlock>, ParseError> {
        let Some(entry) = self.entry(key) else {
            return Ok(None);
        };
        match std::mem::replace(&mut entry.value, Value::Text(String::new())) {
            Value::Grid(rows) => Ok(Some(GridBlock {
                rows,
                line: entry.line,
            })),
            Value::Text(_) => Err(ParseError::at_line(
                entry.line,
                format!("`{key}:` starts a grid, so the grid's rows go on the following lines"),
            )),
        }
    }

    /// Like [`take_text`](Self::take_text), but the key must be there.
    pub fn require_text(&mut self, key: &str) -> Result<(String, usize), ParseError> {
        let start = self.start_line;
        self.take_text(key)?
            .ok_or_else(|| ParseError::at_line(start, format!("missing `{key}:` line")))
    }

    /// Takes the shared keys (F6), checking their format.
    pub fn take_shared(&mut self) -> Result<SharedKeys, ParseError> {
        let mut shared = SharedKeys::default();
        for (key, slot) in [
            ("title", &mut shared.title),
            ("author", &mut shared.author),
            ("source", &mut shared.source),
            ("license", &mut shared.license),
            ("params", &mut shared.params),
        ] {
            *slot = self.take_text(key)?.map(|(value, _)| value);
        }
        shared.solution = self.take_grid("solution")?;
        if let Some((value, line)) = self.take_text("rating")? {
            let mut words = value.split_whitespace();
            let level = words
                .next()
                .and_then(|word| word.parse::<Difficulty>().ok());
            let score = words.next().and_then(|word| word.parse::<u64>().ok());
            match (level, score, words.next()) {
                (Some(level), Some(score), None) => shared.rating = Some((level, score)),
                _ => {
                    return Err(ParseError::at_line(
                        line,
                        format!(
                            "`rating:` needs a level and a score, such as `hard 42`, not {value:?}"
                        ),
                    ));
                }
            }
        }
        shared.rater_version = self.take_number("rater_version")?;
        shared.generator_version = self.take_number("generator_version")?;
        shared.seed = self.take_number("seed")?;
        if let Some((value, line)) = self.take_text("difficulty")? {
            shared.difficulty = Some(
                value
                    .parse()
                    .map_err(|error| ParseError::at_line(line, format!("{error}")))?,
            );
        }
        Ok(shared)
    }

    fn take_number<T: std::str::FromStr>(&mut self, key: &str) -> Result<Option<T>, ParseError> {
        match self.take_text(key)? {
            None => Ok(None),
            Some((value, line)) => value.parse().map(Some).map_err(|_| {
                ParseError::at_line(
                    line,
                    format!("`{key}:` needs a whole number, not {value:?}"),
                )
            }),
        }
    }

    /// Checks that every key has been taken, and fails on the first unknown
    /// one (F7).
    pub fn finish(self) -> Result<(), ParseError> {
        match self.entries.iter().find(|entry| !entry.taken) {
            Some(entry) => Err(ParseError::at_line(
                entry.line,
                format!("unknown key `{}`", entry.key),
            )),
            None => Ok(()),
        }
    }

    fn entry(&mut self, key: &str) -> Option<&mut Entry> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| !entry.taken && entry.key == key)?;
        entry.taken = true;
        Some(entry)
    }
}

/// The keys that mean the same in every genre (F6).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SharedKeys {
    /// The puzzle's title.
    pub title: Option<String>,
    /// Who made the puzzle.
    pub author: Option<String>,
    /// Where the puzzle came from.
    pub source: Option<String>,
    /// The puzzle's license.
    pub license: Option<String>,
    /// The solution grid, as written. The genre checks it.
    pub solution: Option<GridBlock>,
    /// The recorded level and score.
    pub rating: Option<(Difficulty, u64)>,
    /// The rater version that produced `rating`.
    pub rater_version: Option<u32>,
    /// The generator version that made the puzzle.
    pub generator_version: Option<u32>,
    /// The parameters the puzzle was generated with, as written. The genre
    /// checks them.
    pub params: Option<String>,
    /// The difficulty the puzzle was generated for.
    pub difficulty: Option<Difficulty>,
    /// The seed the puzzle was generated with.
    pub seed: Option<u64>,
}

/// Writes the shared text format.
///
/// ```
/// use pencil_puzzles_core::text::Writer;
///
/// let mut writer = Writer::new("kakurasu");
/// writer.text("rows", "1 2");
/// writer.grid("solution", ["#-", "-#"]);
/// assert_eq!(writer.finish(), "genre: kakurasu\nrows: 1 2\nsolution:\n#-\n-#");
/// ```
#[derive(Clone, Debug)]
pub struct Writer {
    out: String,
}

impl Writer {
    /// Starts a puzzle of `genre`.
    pub fn new(genre: &str) -> Self {
        Self {
            out: format!("genre: {genre}\n"),
        }
    }

    /// Writes `key: value`.
    pub fn text(&mut self, key: &str, value: &str) {
        debug_assert!(is_key(key) && !value.is_empty());
        self.out.push_str(key);
        self.out.push_str(": ");
        self.out.push_str(value);
        self.out.push('\n');
    }

    /// Writes a grid block.
    pub fn grid<I: IntoIterator<Item = S>, S: AsRef<str>>(&mut self, key: &str, rows: I) {
        debug_assert!(is_key(key));
        self.out.push_str(key);
        self.out.push_str(":\n");
        for row in rows {
            self.out.push_str(row.as_ref());
            self.out.push('\n');
        }
    }

    /// Writes the shared keys that describe a puzzle (title, author, source
    /// and license). Canonical text puts these right after the `genre:` line.
    pub fn shared_info(&mut self, shared: &SharedKeys) {
        for (key, value) in [
            ("title", &shared.title),
            ("author", &shared.author),
            ("source", &shared.source),
            ("license", &shared.license),
        ] {
            if let Some(value) = value {
                self.text(key, value);
            }
        }
    }

    /// Writes the remaining shared keys: how the puzzle was made, its
    /// rating, and its solution. Canonical text puts these after the genre's
    /// own keys.
    pub fn shared_record(&mut self, shared: &SharedKeys) {
        if let Some(version) = shared.generator_version {
            self.text("generator_version", &version.to_string());
        }
        if let Some(params) = &shared.params {
            self.text("params", params);
        }
        if let Some(difficulty) = shared.difficulty {
            self.text("difficulty", difficulty.as_str());
        }
        if let Some(seed) = shared.seed {
            self.text("seed", &seed.to_string());
        }
        if let Some((level, score)) = shared.rating {
            self.text("rating", &format!("{level} {score}"));
        }
        if let Some(version) = shared.rater_version {
            self.text("rater_version", &version.to_string());
        }
        if let Some(solution) = &shared.solution {
            self.grid("solution", &solution.rows);
        }
    }

    /// The text, without a final newline.
    pub fn finish(mut self) -> String {
        while self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_documents_and_reads_blocks() {
        let text = "# intro\ngenre: a\nrows: 1 2\ngrid:\n#.\n.#\ncols: 3\n\ngenre: b\r\nx: y\r\n";
        let mut documents = parse(text).unwrap();
        assert_eq!(documents.len(), 2);
        let mut a = documents.remove(0);
        assert_eq!(a.genre(), Some("a"));
        assert_eq!(a.take_text("rows").unwrap(), Some(("1 2".into(), 3)));
        assert_eq!(a.take_grid("grid").unwrap().unwrap().rows, ["#.", ".#"]);
        assert_eq!(a.take_text("cols").unwrap(), Some(("3".into(), 7)));
        a.finish().unwrap();
        let mut b = documents.remove(0);
        assert_eq!(b.start_line(), 9);
        assert_eq!(b.take_text("x").unwrap(), Some(("y".into(), 10)));
    }

    #[test]
    fn rows_starting_with_hash_are_grid_rows_not_comments() {
        let mut document = parse("genre: a\ngrid:\n#-\n##").unwrap().remove(0);
        assert_eq!(
            document.take_grid("grid").unwrap().unwrap().rows,
            ["#-", "##"]
        );
    }

    #[test]
    fn errors_name_the_line() {
        let line = |text: &str| match parse(text) {
            Err(error) => error.position(),
            Ok(_) => panic!("{text:?} should not parse"),
        };
        use crate::Position::Line;
        assert_eq!(line("genre: a\nRows: 1"), Line(2));
        assert_eq!(line("genre: a\nno colon"), Line(2));
        assert_eq!(line("genre: a\nx: 1\nx: 2"), Line(3));
        assert_eq!(line("genre: a\ng:\n #"), Line(3));
        assert_eq!(line("genre:"), Line(1));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let document = parse("genre: a\nmystery: 1").unwrap().remove(0);
        assert_eq!(
            document.finish().unwrap_err().position(),
            crate::Position::Line(2)
        );
    }

    #[test]
    fn shared_keys_round_trip() {
        let text = "genre: a\ntitle: T\nlicense: CC0\nx: 1\ngenerator_version: 1\nparams: 5x5\ndifficulty: hard\nseed: 7\nrating: hard 42\nrater_version: 3\nsolution:\n#-";
        let mut document = parse(text).unwrap().remove(0);
        let shared = document.take_shared().unwrap();
        let (x, _) = document.require_text("x").unwrap();
        document.finish().unwrap();
        assert_eq!(shared.rating, Some((Difficulty::Hard, 42)));
        let mut writer = Writer::new("a");
        writer.shared_info(&shared);
        writer.text("x", &x);
        writer.shared_record(&shared);
        assert_eq!(writer.finish(), text);
    }

    #[test]
    fn bad_shared_values_are_rejected() {
        for text in [
            "genre: a\nrating: hard",
            "genre: a\nseed: -1",
            "genre: a\ndifficulty: tough",
        ] {
            let mut document = parse(text).unwrap().remove(0);
            assert!(document.take_shared().is_err(), "{text:?}");
        }
    }
}
