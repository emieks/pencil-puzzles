//! Hitori's shared contract and shading presentation.
use crate::model::{LIMITS, read_document};
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, View, ViewCell,
    text::Document,
};

/// Hitori: shade repeated numbers away.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hitori;

static METADATA: Metadata = {
    let mut m = Metadata::new(
        "hitori",
        "Hitori",
        "Shade cells so no unshaded number repeats in a row or column; shaded cells never share a side and unshaded cells stay connected.",
        GENERATOR_VERSION,
        RATER_VERSION,
    );
    m.rules = &[
        Rule::new(1, "no repeats"),
        Rule::new(2, "shaded cells apart"),
        Rule::new(3, "unshaded cells connected"),
    ];
    m
};

impl Genre for Hitori {
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
        v.marks = vec![
            MarkOption::new(".", "Unmarked", ""),
            MarkOption::shaded("#", "Shaded"),
            MarkOption::new("-", "Unshaded", "\u{00b7}"),
        ];
        v.cells = p
            .numbers()
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                let mark = answer.map_or(Mark::Unmarked, |a| a.get(i / sh.cols(), i % sh.cols()));
                let mut cell = ViewCell::editable(mark.to_char().to_string());
                cell.label = n.to_string();
                cell
            })
            .collect();
        v.finish_note = "Blanks count as unshaded cells.".into();
        v.instructions = "Shade cells so that no number appears twice among the unshaded cells of a row or column.\nShaded cells may not share a side.\nThe unshaded cells must form one group joined by their sides.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != p.shape().cells() {
            return Err(ParseError::new("marks must match the Hitori grid"));
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
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> PuzzleEditor {
        let p = puzzle;
        let sh = p.map_or(shape, Puzzle::shape);
        let fields = (0..sh.cells())
            .map(|i| {
                ClueInput::new(
                    crate::board::cell(i, sh.cols()),
                    p.map_or(String::new(), |p| p.numbers()[i].to_string()),
                    1,
                    sh.largest(),
                    false,
                )
            })
            .collect();
        let mut view = match p {
            Some(p) => Self::view(p, None),
            None => {
                let ones = Puzzle::new(sh, vec![1; sh.cells()]).expect("validated shape");
                let mut view = Self::view(&ones, None);
                for cell in &mut view.cells {
                    cell.label.clear();
                }
                view
            }
        };
        view.finish_note.clear();
        PuzzleEditor::new(sh.to_string(), view, fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.cells() {
            return Err(ParseError::new("numbers must match the Hitori grid"));
        }
        let mut numbers = Vec::with_capacity(shape.cells());
        for s in values {
            if s.is_empty()
                || s.starts_with('0')
                || s.len() > 2
                || !s.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(ParseError::new(format!(
                    "every cell needs a number from 1 to {}",
                    shape.largest()
                )));
            }
            numbers.push(
                s.parse::<u32>()
                    .map_err(|_| ParseError::new("the number is too large"))?,
            );
        }
        Puzzle::new(shape, numbers)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
