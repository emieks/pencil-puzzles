//! Fillomino's shared contract and number-entry presentation.
use crate::model::{LIMITS, read_document};
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, View, ViewCell,
    text::Document,
};

/// Fillomino: regions numbered with their sizes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Fillomino;

static METADATA: Metadata = {
    let mut m = Metadata::new(
        "fillomino",
        "Fillomino",
        "Divide the grid into regions and write each region's size in its cells; regions of the same size never touch.",
        GENERATOR_VERSION,
        RATER_VERSION,
    );
    m.aliases = &["allied-occupation"];
    m.rules = &[
        Rule::new(1, "region size"),
        Rule::new(2, "every cell numbered"),
        Rule::new(3, "printed numbers"),
    ];
    m
};

/// The largest number offered for entry: 9, which covers every generated
/// puzzle and matches the digit keys, or the largest printed clue if that
/// is bigger, but never more than the number of cells.
fn largest(p: &Puzzle) -> u32 {
    let clue = p.clues().iter().copied().max().unwrap_or(0);
    clue.max(9).min(p.shape().cells() as u32)
}

impl Genre for Fillomino {
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
        let mut a: Answer = text.parse()?;
        if a.shape() != p.shape() {
            return Err(ParseError::new("the answer has the wrong shape"));
        }
        let cols = p.shape().cols();
        for (i, &n) in p.clues().iter().enumerate() {
            if n > 0 && a.get(i / cols, i % cols) == Mark::Unmarked {
                a.set(i / cols, i % cols, Mark::Number(n));
            }
        }
        Ok(a)
    }
    fn empty_answer(p: &Puzzle) -> Answer {
        let mut a = Answer::new(p.shape());
        let cols = p.shape().cols();
        for (i, &n) in p.clues().iter().enumerate() {
            if n > 0 {
                a.set(i / cols, i % cols, Mark::Number(n));
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
        let answer = answer.filter(|a| a.shape() == p.shape());
        let sh = p.shape();
        let mut v = View::default();
        v.rows = sh.rows() as u32;
        v.cols = sh.cols() as u32;
        v.marks = vec![MarkOption::new(".", "Unmarked", "")];
        v.marks.extend((1..=largest(p)).map(|n| {
            let s = n.to_string();
            MarkOption::new(&s, &s, &s)
        }));
        v.cells = p
            .clues()
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                let mark = if n > 0 {
                    Mark::Number(n)
                } else {
                    answer.map_or(Mark::Unmarked, |a| a.get(i / sh.cols(), i % sh.cols()))
                };
                let mut cell = ViewCell::editable(mark.token());
                cell.editable = n == 0;
                cell
            })
            .collect();
        v.finish_note = "Every cell needs a number.".into();
        v.instructions = "Write a number in every cell. Cells with the same number that touch form a region, and each region has exactly as many cells as its number, so regions of the same size never share an edge. Printed numbers stay fixed. Type digits, or choose one below the grid.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != p.shape().cells() {
            return Err(ParseError::new("marks must match the Fillomino grid"));
        }
        let cols = p.shape().cols();
        let mut a = Answer::new(p.shape());
        for (i, s) in values.iter().enumerate() {
            let mark = Mark::from_token(s, p.shape().cells()).ok_or_else(|| {
                ParseError::new("use unmarked, empty or a number up to the cell count")
            })?;
            let n = p.clues()[i];
            let mark = if n > 0 {
                if mark != Mark::Unmarked && mark != Mark::Number(n) {
                    return Err(ParseError::new("printed numbers cannot be changed"));
                }
                Mark::Number(n)
            } else {
                mark
            };
            a.set(i / cols, i % cols, mark);
        }
        Ok(a)
    }
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> PuzzleEditor {
        let empty = Puzzle::new(shape, vec![0; shape.cells()]).expect("validated shape");
        let p = puzzle.unwrap_or(&empty);
        let sh = p.shape();
        let fields = p
            .clues()
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                ClueInput::new(
                    crate::board::cell(i, sh.cols()),
                    if n == 0 { String::new() } else { n.to_string() },
                    1,
                    sh.cells() as u32,
                    true,
                )
            })
            .collect();
        PuzzleEditor::new(sh.to_string(), Self::view(p, None), fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.cells() {
            return Err(ParseError::new("clues must match the Fillomino grid"));
        }
        let mut clues = Vec::with_capacity(shape.cells());
        for s in values {
            if s.is_empty() {
                clues.push(0);
                continue;
            }
            if s.starts_with('0') || s.len() > 3 || !s.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseError::new("use positive whole-number clues"));
            }
            clues.push(
                s.parse::<u32>()
                    .map_err(|_| ParseError::new("the clue is too large"))?,
            );
        }
        Puzzle::new(shape, clues)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
