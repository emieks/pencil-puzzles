//! Chocona's shared contract and its view with regions and numbers.
use crate::model::{LIMITS, read_document};
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, View, ViewCell,
    text::Document,
};

/// Chocona: shaded rectangles counted region by region.
#[derive(Clone, Copy, Debug, Default)]
pub struct Chocona;

static METADATA: Metadata = {
    let mut m = Metadata::new(
        "chocona",
        "Chocona",
        "Shade rectangles so each region has as many shaded cells as its number.",
    );
    m.rules = &[Rule::new(1, "region counts"), Rule::new(2, "rectangles")];
    m
};

/// What an editor cell may hold: a region letter, optionally followed by
/// the region's number, or a region number from 1 to 999, optionally
/// followed by `:` and the region's number.
const CELL_PATTERN: &str = "[A-Za-z](0|[1-9][0-9]{0,2})?|[1-9][0-9]{0,2}(:(0|[1-9][0-9]{0,2}))?";

/// An editor cell's region label and number, if it has one.
fn read_cell(value: &str) -> Result<(&str, Option<u32>), ParseError> {
    let split = match value.split_once(':') {
        Some((label, clue)) if !label.starts_with(|c: char| c.is_ascii_alphabetic()) => {
            Some((label, clue))
        }
        Some(_) => None,
        None => match value.char_indices().nth(1) {
            Some((at, _)) if value.starts_with(|c: char| c.is_ascii_alphabetic()) => {
                Some((&value[..at], &value[at..]))
            }
            _ => Some((value, ".")),
        },
    };
    let bad = || {
        ParseError::new(format!(
            "{value:?} is not a cell; write a region letter and its number, such as `a` or `a3`"
        ))
    };
    let (label, clue) = split.ok_or_else(bad)?;
    if clue == "." {
        return Ok((label, None));
    }
    match crate::model::read_clue(clue) {
        Ok(Some(n)) => Ok((label, Some(n))),
        _ => Err(bad()),
    }
}

impl Genre for Chocona {
    fn text_limits() -> pencil_puzzles_core::text::GridLimits {
        LIMITS
    }
    type Shape = Shape;
    type Params = Params;
    type Puzzle = Puzzle;
    type Answer = Answer;
    type GenerateError = GenerateError;
    type HintError = HintError;
    fn metadata() -> &'static Metadata {
        &METADATA
    }
    fn params_for_shape(shape: Shape) -> Params {
        Params::new(shape)
    }
    fn parse_document(doc: Document) -> Result<Record, ParseError> {
        read_document(doc)
    }
    fn write_record(record: &Record) -> String {
        write_record(record)
    }
    fn id(puzzle: &Puzzle) -> String {
        puzzle.id()
    }
    fn from_id(id: &str) -> Result<Puzzle, ParseError> {
        Puzzle::from_id(id)
    }
    fn parse_answer(p: &Puzzle, text: &str) -> Result<Answer, ParseError> {
        let a: Answer = text.parse()?;
        if a.shape() != p.shape() {
            return Err(ParseError::new("the answer has the wrong shape"));
        }
        Ok(a)
    }
    fn empty_answer(p: &Puzzle) -> Answer {
        Answer::new(p.shape())
    }
    fn finished(answer: &Answer) -> Answer {
        answer.finished()
    }
    fn solve(puzzle: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
        solve(puzzle, budget)
    }
    fn rate(puzzle: &Puzzle, budget: &Budget) -> Result<Rating, RateError> {
        rate(puzzle, budget)
    }
    fn rate_with_steps(puzzle: &Puzzle, budget: &Budget) -> Result<(Rating, Vec<Step>), RateError> {
        rate_with_steps(puzzle, budget)
    }
    fn supports(params: &Params, difficulty: Option<Difficulty>) -> bool {
        crate::generate::unsupported(params.shape, difficulty).is_none()
    }
    fn generate(
        params: &Params,
        difficulty: Option<Difficulty>,
        seed: u64,
        max_attempts: u32,
        budget: &Budget,
    ) -> Result<Generation<Puzzle, Answer>, GenerateError> {
        let mut options = GenerateOptions::new(*params, seed);
        options.difficulty = difficulty;
        options.max_attempts = max_attempts;
        let g = generate(&options, budget)?;
        let mut record = Record::new(g.puzzle);
        record.solution = Some(g.solution);
        record.shared.params = Some(params.to_string());
        record.shared.difficulty = difficulty;
        record.shared.seed = Some(seed);
        record.shared.rating = Some((g.rating.level, g.rating.score));
        Ok(Generation::new(record, g.rating, g.attempts))
    }
    fn check(
        puzzle: &Puzzle,
        answer: &Answer,
        _: &Budget,
    ) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(check(puzzle, answer)?)
    }
    fn hint(puzzle: &Puzzle, answer: &Answer, budget: &Budget) -> Result<Hint, HintError> {
        hint(puzzle, answer, budget)
    }
    fn view(p: &Puzzle, answer: Option<&Answer>) -> View {
        let answer = answer.filter(|a| a.shape() == p.shape());
        let sh = p.shape();
        let mut v = View::default();
        v.rows = sh.rows() as u32;
        v.cols = sh.cols() as u32;
        let mut shaded = MarkOption::shaded("#", "Shaded");
        shaded.tone = "gray".into();
        let mut unshaded = MarkOption::new("-", "Unshaded", "\u{00b7}");
        unshaded.tone = "blue".into();
        v.marks = vec![MarkOption::new(".", "Unmarked", ""), shaded, unshaded];
        let mut labels = vec![String::new(); sh.cells()];
        for (t, i) in p.clue_cells().into_iter().enumerate() {
            labels[i] = p.clues()[t].map_or(String::new(), |n| n.to_string());
        }
        v.cells = (p.regions().iter().zip(labels).enumerate())
            .map(|(i, (&region, label))| {
                let mark = answer.map_or(Mark::Unmarked, |a| a.get(i / sh.cols(), i % sh.cols()));
                let mut cell = ViewCell::editable(mark.to_char().to_string());
                cell.region = Some(region);
                cell.label = label;
                cell
            })
            .collect();
        v.finish_note = "Blanks count as unshaded cells.".into();
        v.instructions = "Shade some cells. A number in an outlined region tells how many of its cells are shaded; a region without a number may have any number of shaded cells.\nEach group of shaded cells joined by their sides fills a rectangle. Rectangles may cross region borders.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != p.shape().cells() {
            return Err(ParseError::new("marks must match the Chocona grid"));
        }
        let cols = p.shape().cols();
        let mut a = Answer::new(p.shape());
        for (i, s) in values.iter().enumerate() {
            let mark = match s.as_str() {
                "." => Mark::Unmarked,
                "#" => Mark::Shaded,
                "-" => Mark::Unshaded,
                _ => return Err(ParseError::new("use unmarked, shaded or unshaded marks")),
            };
            a.set(i / cols, i % cols, mark);
        }
        Ok(a)
    }
    /// One field per cell: its region's letter or number, followed by the
    /// region's number in one cell of the region, such as `a3`, or `12:3`
    /// for a region numbered 12.
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> PuzzleEditor {
        let sh = puzzle.map_or(shape, Puzzle::shape);
        let blank = Puzzle::new(sh, &vec![0; sh.cells()], vec![None]).expect("validated shape");
        let p = puzzle.unwrap_or(&blank);
        let rows = p.region_rows();
        let spaced = rows[0].contains(' ');
        let mut labels: Vec<String> = rows
            .iter()
            .flat_map(|row| -> Vec<String> {
                if spaced {
                    row.split(' ').map(String::from).collect()
                } else {
                    row.chars().map(String::from).collect()
                }
            })
            .collect();
        for (t, i) in p.clue_cells().into_iter().enumerate() {
            if let Some(n) = p.clues()[t] {
                let sep = if spaced { ":" } else { "" };
                labels[i] = format!("{}{sep}{n}", labels[i]);
            }
        }
        let fields = labels
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                let value = puzzle.map_or(String::new(), |_| label);
                ClueInput::text(crate::board::cell(i, sh.cols()), value, CELL_PATTERN, false)
            })
            .collect();
        let mut view = Self::view(p, None);
        for cell in &mut view.cells {
            cell.label.clear();
        }
        view.finish_note.clear();
        PuzzleEditor::new(sh.to_string(), view, fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.cells() {
            return Err(ParseError::new("enter a region for every cell"));
        }
        let mut names = std::collections::BTreeMap::new();
        let mut labels = Vec::with_capacity(values.len());
        let mut clues: Vec<Option<u32>> = Vec::new();
        for value in values {
            let (label, clue) = read_cell(value)?;
            let t = crate::model::label(&mut names, label)? as usize;
            labels.push(t as u32);
            if t == clues.len() {
                clues.push(None);
            }
            if let Some(n) = clue {
                if clues[t].is_some() {
                    return Err(ParseError::new(format!(
                        "region {label} has more than one number"
                    )));
                }
                clues[t] = Some(n);
            }
        }
        Puzzle::new(shape, &labels, clues)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
