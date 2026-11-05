//! Typed Sudoku contract and browser presentation (I1, I4).
use crate::model::{EMPTY, read_document};
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, Status, View, ViewCell,
    text::Document,
};
/// Sudoku: digit placement in rows, columns and square boxes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sudoku;
static METADATA: Metadata = {
    let mut m = Metadata::new(
        "sudoku",
        "Sudoku",
        "Fill rows, columns and boxes with each digit exactly once.",
        Status::Stable,
        GENERATOR_VERSION,
        RATER_VERSION,
    );
    m.aliases = &["number-place", "nanpure"];
    m.rules = &[
        Rule::new(1, "rows"),
        Rule::new(2, "columns"),
        Rule::new(3, "boxes"),
        Rule::new(4, "givens"),
    ];
    m
};
impl Genre for Sudoku {
    fn text_limits() -> pencil_puzzles_core::text::GridLimits {
        pencil_puzzles_core::text::GridLimits::new(9, 9)
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
    fn parse_answer(puzzle: &Puzzle, text: &str) -> Result<Answer, ParseError> {
        let mut a: Answer = text.parse()?;
        if a.shape() != puzzle.shape() {
            return Err(ParseError::new("the answer has the wrong Sudoku shape"));
        }
        for (i, &g) in puzzle.givens().iter().enumerate() {
            if g != 0 && a.cells[i] == 0 {
                a.cells[i] = g;
            }
        }
        Ok(a)
    }
    fn empty_answer(puzzle: &Puzzle) -> Answer {
        Answer {
            shape: puzzle.shape(),
            cells: puzzle.givens().to_vec(),
        }
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
        if !budget.spend((3 * puzzle.shape().size()) as u64) {
            return Err(Box::new(BudgetExceeded));
        }
        Ok(check(puzzle, answer)?)
    }
    fn hint(puzzle: &Puzzle, answer: &Answer, budget: &Budget) -> Result<Hint, HintError> {
        hint(puzzle, answer, budget)
    }
    fn view(puzzle: &Puzzle, answer: Option<&Answer>) -> View {
        let shape = puzzle.shape();
        let answer = answer.filter(|a| a.shape() == shape);
        let n = shape.size();
        let b = shape.box_size();
        let mut view = View::default();
        view.rows = n as u32;
        view.cols = n as u32;
        view.marks = vec![MarkOption::new(".", "Unmarked", "")];
        view.marks.extend(
            (1..=n).map(|v| MarkOption::new(&v.to_string(), &v.to_string(), &v.to_string())),
        );
        view.cells = puzzle
            .givens()
            .iter()
            .enumerate()
            .map(|(i, &given)| {
                let v = if given != 0 {
                    given
                } else {
                    answer.map_or(0, |a| a.cells[i])
                };
                let value = match v {
                    0 => ".".into(),
                    EMPTY => "0".into(),
                    _ => v.to_string(),
                };
                let mut cell = ViewCell::editable(value);
                cell.editable = given == 0;
                cell.region = Some(((i / n / b) * b + i % n / b) as u32);
                cell
            })
            .collect();
        view.row_separators = (1..b).map(|k| (k * b) as u32).collect();
        view.col_separators = view.row_separators.clone();
        view.finish_note = "Empty cells must be filled.".into();
        view.instructions = format!(
            "Fill each row, column and {}×{} box with digits 1–{}, once each. Printed digits stay fixed. Type digits, or choose one below the grid.",
            b, b, n
        );
        view
    }
    fn answer_from_values(puzzle: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        let shape = puzzle.shape();
        if values.len() != shape.cells() {
            return Err(ParseError::new("the marks must match the Sudoku grid"));
        }
        let mut a = Answer::new(shape);
        for (i, s) in values.iter().enumerate() {
            let v = if s == "." {
                0
            } else if s == "0" {
                EMPTY
            } else {
                let v = s
                    .parse::<u8>()
                    .map_err(|_| ParseError::new("use digits within the Sudoku range"))?;
                if v == 0 || usize::from(v) > shape.size() {
                    return Err(ParseError::new("the digit is outside the Sudoku range"));
                }
                v
            };
            let given = puzzle.givens()[i];
            if given != 0 && v != 0 && v != given {
                return Err(ParseError::new("printed digits cannot be changed"));
            }
            a.cells[i] = if given != 0 { given } else { v };
        }
        Ok(a)
    }
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> PuzzleEditor {
        let blank = Puzzle::new(shape, vec![0; shape.cells()]).expect("validated shape");
        let p = puzzle.unwrap_or(&blank);
        let shape = p.shape();
        let n = shape.size();
        let fields = p
            .givens()
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                ClueInput::new(
                    Place::Cell {
                        row: (i / n) as u32,
                        col: (i % n) as u32,
                    },
                    if v == 0 { String::new() } else { v.to_string() },
                    1,
                    n as u32,
                    true,
                )
            })
            .collect();
        PuzzleEditor::new(shape.to_string(), Self::view(p, None), fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.cells() {
            return Err(ParseError::new("the clues must match the Sudoku grid"));
        }
        let givens = values
            .iter()
            .map(|s| {
                if s.is_empty() {
                    return Ok(0);
                }
                if s.len() != 1 || !s.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(ParseError::new("each given must be one digit"));
                }
                let v = s.as_bytes()[0] - b'0';
                if v == 0 || usize::from(v) > shape.size() {
                    return Err(ParseError::new("the given is outside the Sudoku range"));
                }
                Ok(v)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Puzzle::new(shape, givens)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
