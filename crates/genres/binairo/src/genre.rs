//! Binairo's shared contract and its view of givens and digits.
use crate::model::{LIMITS, read_document};
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, View, ViewCell,
    text::Document,
};

/// Binairo: a grid of 0s and 1s with balanced, different lines.
#[derive(Clone, Copy, Debug, Default)]
pub struct Binairo;

static METADATA: Metadata = {
    let mut m = Metadata::new(
        "binairo",
        "Binairo",
        "Fill the grid with 0s and 1s: no three alike in a row, as many 0s as 1s in each line, no two lines the same.",
    );
    m.aliases = &["takuzu"];
    m.rules = &[
        Rule::new(1, "no three in a row"),
        Rule::new(2, "as many 0s as 1s"),
        Rule::new(3, "different lines"),
        Rule::new(4, "givens"),
    ];
    m
};

impl Genre for Binairo {
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
    /// Given cells are labelled with their digit and cannot be changed. The
    /// other cells show the answer's marks: 1 shaded, 0 a dot.
    fn view(p: &Puzzle, answer: Option<&Answer>) -> View {
        let answer = answer.filter(|a| a.shape() == p.shape());
        let sh = p.shape();
        let mut v = View::default();
        v.rows = sh.rows() as u32;
        v.cols = sh.cols() as u32;
        let mut zero = MarkOption::new("0", "0", "\u{00b7}");
        zero.tone = "blue".into();
        v.marks = vec![
            MarkOption::new(".", "Unmarked", ""),
            MarkOption::shaded("1", "1"),
            zero,
        ];
        v.cells = p
            .givens()
            .iter()
            .enumerate()
            .map(|(i, &given)| match given {
                Mark::Unmarked => {
                    let mark =
                        answer.map_or(Mark::Unmarked, |a| a.get(i / sh.cols(), i % sh.cols()));
                    ViewCell::editable(mark.to_char().to_string())
                }
                _ => {
                    let mut cell = ViewCell::editable(given.to_char().to_string());
                    cell.editable = false;
                    cell.label = given.to_char().to_string();
                    cell
                }
            })
            .collect();
        v.finish_note = "Blanks count as 0s.".into();
        v.instructions = "Fill every cell with 1 (shaded) or 0 (dot).\nNo three equal digits may be next to each other in a row or column.\nEvery row and every column has as many 0s as 1s.\nNo two rows are the same, and no two columns are the same.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != p.shape().cells() {
            return Err(ParseError::new("marks must match the Binairo grid"));
        }
        let cols = p.shape().cols();
        let mut a = Answer::new(p.shape());
        for (i, s) in values.iter().enumerate() {
            let mut chars = s.chars();
            let mark = match (chars.next().and_then(Mark::from_char), chars.next()) {
                (Some(mark), None) => mark,
                _ => return Err(ParseError::new("use unmarked, 0 or 1 marks")),
            };
            a.set(i / cols, i % cols, mark);
        }
        Ok(a)
    }
    /// The fields are a digit (0 or 1) or nothing for every cell.
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> PuzzleEditor {
        let blank = Puzzle::new(shape, vec![Mark::Unmarked; shape.cells()]).expect("valid shape");
        let p = puzzle.unwrap_or(&blank);
        let sh = p.shape();
        let fields = (p.givens().iter().enumerate())
            .map(|(i, &given)| {
                let value = given.digit().map_or(String::new(), |d| d.to_string());
                ClueInput::new(crate::board::cell(i, sh.cols()), value, 0, 1, true)
            })
            .collect();
        PuzzleEditor::new(sh.to_string(), Self::view(p, None), fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.cells() {
            return Err(ParseError::new("the givens must match the Binairo grid"));
        }
        let givens = values
            .iter()
            .map(|s| match s.as_str() {
                "" => Ok(Mark::Unmarked),
                "0" => Ok(Mark::Zero),
                "1" => Ok(Mark::One),
                _ => Err(ParseError::new("each given must be 0 or 1")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Puzzle::new(shape, givens)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
