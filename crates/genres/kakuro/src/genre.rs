//! Kakuro's shared contract and digit-entry presentation.
use crate::model::{LIMITS, read_document};
use crate::*;
use pencil_puzzles_core::{
    BlockCell, ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, View,
    ViewCell, text::Document,
};

/// Kakuro: digits in white cells adding up to the sums in the blocks.
#[derive(Clone, Copy, Debug, Default)]
pub struct Kakuro;

static METADATA: Metadata = {
    let mut m = Metadata::new(
        "kakuro",
        "Kakuro",
        "Write a digit from 1 to 9 in every white cell so that each run adds up to the sum before it, with no digit repeated in a run.",
        GENERATOR_VERSION,
        RATER_VERSION,
    );
    m.aliases = &["cross-sums"];
    m.rules = &[Rule::new(1, "runs"), Rule::new(2, "every cell filled")];
    m
};

/// The editor's field pattern: empty for a white cell, `#`, or sums.
const PATTERN: &str = r"#|[1-9][0-9]?/([1-9][0-9]?)?|/[1-9][0-9]?";

fn block_view(cell: Cell) -> ViewCell {
    let Cell::Block { down, across } = cell else {
        return ViewCell::editable(".".into());
    };
    let label = |s: Option<u8>| s.map_or(String::new(), |s| s.to_string());
    let name = match (down, across) {
        (Some(d), Some(a)) => format!("down {d}, across {a}"),
        (Some(d), None) => format!("down {d}"),
        (None, Some(a)) => format!("across {a}"),
        (None, None) => "block".into(),
    };
    ViewCell::block("#".into(), BlockCell::new(label(across), label(down), name))
}

impl Genre for Kakuro {
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
        if !p.fits(&a) {
            return Err(ParseError::new(
                "the answer must have `#` exactly on the puzzle's blocks",
            ));
        }
        Ok(a)
    }
    fn empty_answer(p: &Puzzle) -> Answer {
        let mut a = Answer::new(p.shape());
        let cols = p.shape().cols();
        for (i, &c) in p.cells().iter().enumerate() {
            if c == Cell::White {
                a.set(i / cols, i % cols, Mark::Unmarked);
            }
        }
        a
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
        record.shared.generator_version = Some(g.generator_version);
        record.shared.rating = Some((g.rating.level, g.rating.score));
        record.shared.rater_version = Some(g.rating.rater_version);
        Ok(Generation::new(record, g.rating, g.attempts))
    }
    fn check(
        puzzle: &Puzzle,
        answer: &Answer,
        budget: &Budget,
    ) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
        check(puzzle, answer, budget)
    }
    fn hint(puzzle: &Puzzle, answer: &Answer, budget: &Budget) -> Result<Hint, HintError> {
        hint(puzzle, answer, budget)
    }
    fn view(p: &Puzzle, answer: Option<&Answer>) -> View {
        let answer = answer.filter(|a| p.fits(a));
        let sh = p.shape();
        let mut v = View::default();
        v.rows = sh.rows() as u32;
        v.cols = sh.cols() as u32;
        v.marks = vec![MarkOption::new(".", "Unmarked", "")];
        v.marks.extend((1..=9).map(|n| {
            let s = n.to_string();
            MarkOption::new(&s, &s, &s)
        }));
        v.cells = p
            .cells()
            .iter()
            .enumerate()
            .map(|(i, &c)| match c {
                Cell::White => {
                    let mark = answer.map_or(Mark::Unmarked, |a| a.marks()[i]);
                    ViewCell::editable(mark.symbol().to_string())
                }
                block => block_view(block),
            })
            .collect();
        v.finish_note = "Every white cell needs a digit.".into();
        v.instructions = "Write a digit from 1 to 9 in every white cell. A run is a line of white cells across or down; its digits add up to the sum in the block before it (down sums below the diagonal, across sums above it), and no digit repeats in a run. Type digits, or choose one below the grid.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != p.shape().cells() {
            return Err(ParseError::new("marks must match the Kakuro grid"));
        }
        let cols = p.shape().cols();
        let mut a = Answer::new(p.shape());
        for (i, s) in values.iter().enumerate() {
            let mut chars = s.chars();
            let mark = match (chars.next(), chars.next()) {
                (Some(c), None) => Mark::from_symbol(c),
                _ => None,
            }
            .ok_or_else(|| ParseError::new("use `#`, unmarked, empty or a digit"))?;
            if (p.cells()[i] == Cell::White) == (mark == Mark::Block) {
                return Err(ParseError::new(
                    "blocks are `#` and white cells hold digits",
                ));
            }
            a.set(i / cols, i % cols, mark);
        }
        Ok(a)
    }
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> PuzzleEditor {
        let sh = puzzle.map_or(shape, Puzzle::shape);
        let cols = sh.cols();
        let token = |i: usize| match puzzle {
            Some(p) => match p.cells()[i] {
                Cell::White => String::new(),
                c => c.token(),
            },
            None if i / cols == 0 || i.is_multiple_of(cols) => "#".into(),
            None => String::new(),
        };
        let mut view = View::default();
        view.rows = sh.rows() as u32;
        view.cols = cols as u32;
        view.marks = vec![MarkOption::new(".", "White", "")];
        view.cells = (0..sh.cells())
            .map(|_| ViewCell::editable(".".into()))
            .collect();
        let fields = (0..sh.cells())
            .map(|i| ClueInput::text(crate::board::cell(i, cols), token(i), PATTERN, true))
            .collect();
        PuzzleEditor::new(sh.to_string(), view, fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.cells() {
            return Err(ParseError::new("cells must match the Kakuro grid"));
        }
        let cols = shape.cols();
        let mut cells = Vec::with_capacity(values.len());
        for (i, s) in values.iter().enumerate() {
            let s = s.trim();
            cells.push(if s.is_empty() {
                Cell::White
            } else {
                Cell::from_token(s).map_err(|e| {
                    ParseError::new(format!(
                        "row {}, column {}: {}",
                        i / cols + 1,
                        i % cols + 1,
                        e.message()
                    ))
                })?
            });
        }
        Puzzle::new(shape, cells).map_err(|e| ParseError::new(e.message()))
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
