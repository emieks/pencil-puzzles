//! Kakurasu's implementation of the common genre contract (I1).
use crate::*;
use pencil_puzzles_core::{Generation, Genre, Metadata, Rule, text::Document};
/// The Kakurasu genre (P1-P9, I1).
#[derive(Clone, Copy, Debug, Default)]
pub struct Kakurasu;
const RULES: &[Rule] = &[Rule::new(1, "row totals"), Rule::new(2, "column totals")];
static METADATA: Metadata = {
    let mut m = Metadata::new(
        "kakurasu",
        "Kakurasu",
        "Shade cells so every weighted row and column total matches its clue.",
        GENERATOR_VERSION,
        RATER_VERSION,
    );
    m.rules = RULES;
    m.aliases = &["index-sums"];
    m
};
impl Genre for Kakurasu {
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
    fn parse_document(document: Document) -> Result<Record, ParseError> {
        crate::puzzle::read_document(document)
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
        let answer: Answer = text.parse()?;
        if answer.shape() != puzzle.shape() {
            return Err(ParseError::new(format!(
                "the answer must be a {} grid",
                puzzle.shape()
            )));
        }
        Ok(answer)
    }
    fn empty_answer(puzzle: &Puzzle) -> Answer {
        Answer::new(puzzle.shape())
    }
    fn finished(answer: &Answer) -> Answer {
        answer.finished()
    }
    fn solve(puzzle: &Puzzle, budget: &Budget) -> Result<Solutions, BudgetExceeded> {
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
        record.shared.rater_version = Some(g.rating.rater_version);
        record.shared.rating = Some((g.rating.level, g.rating.score));
        Ok(Generation::new(record, g.rating, g.attempts))
    }
    fn check(
        puzzle: &Puzzle,
        answer: &Answer,
        budget: &Budget,
    ) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
        if !budget.spend((puzzle.rows() + puzzle.cols()) as u64) {
            return Err(Box::new(BudgetExceeded));
        }
        Ok(check(puzzle, answer)?)
    }
    fn hint(puzzle: &Puzzle, answer: &Answer, budget: &Budget) -> Result<Hint, HintError> {
        hint(puzzle, answer, budget)
    }
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> pencil_puzzles_core::PuzzleEditor {
        use pencil_puzzles_core::{ClueInput, PuzzleEditor};
        let blank =
            Puzzle::new(vec![0; shape.rows()], vec![0; shape.cols()]).expect("validated shape");
        let p = puzzle.unwrap_or(&blank);
        let shape = p.shape();
        let mut view = Self::view(p, None);
        view.right.fill(String::new());
        view.bottom.fill(String::new());
        let rows = p.row_clues().iter().enumerate().map(|(i, n)| {
            ClueInput::new(
                Place::Row(i as u32),
                if puzzle.is_some() {
                    n.to_string()
                } else {
                    String::new()
                },
                0,
                (shape.cols() * (shape.cols() + 1) / 2) as u32,
                false,
            )
        });
        let cols = p.col_clues().iter().enumerate().map(|(i, n)| {
            ClueInput::new(
                Place::Col(i as u32),
                if puzzle.is_some() {
                    n.to_string()
                } else {
                    String::new()
                },
                0,
                (shape.rows() * (shape.rows() + 1) / 2) as u32,
                false,
            )
        });
        PuzzleEditor::new(shape.to_string(), view, rows.chain(cols).collect())
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.rows() + shape.cols() {
            return Err(ParseError::new("enter a clue for every row and column"));
        }
        let clues = values
            .iter()
            .enumerate()
            .map(|(i, value)| {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(ParseError::new(format!(
                        "clue {} must be a nonnegative whole number",
                        i + 1
                    )));
                }
                value
                    .parse::<u32>()
                    .map_err(|_| ParseError::new(format!("clue {} is too large", i + 1)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Puzzle::new(
            clues[..shape.rows()].to_vec(),
            clues[shape.rows()..].to_vec(),
        )
    }
    fn view(puzzle: &Puzzle, answer: Option<&Answer>) -> pencil_puzzles_core::View {
        use pencil_puzzles_core::{MarkOption, View, ViewCell};
        let answer = answer.filter(|a| a.shape() == puzzle.shape());
        let mut view = View::default();
        view.rows = puzzle.rows() as u32;
        view.cols = puzzle.cols() as u32;
        view.top = (1..=puzzle.cols()).map(|n| n.to_string()).collect();
        view.left = (1..=puzzle.rows()).map(|n| n.to_string()).collect();
        view.right = puzzle.row_clues().iter().map(u32::to_string).collect();
        view.bottom = puzzle.col_clues().iter().map(u32::to_string).collect();
        view.cells = (0..puzzle.rows() * puzzle.cols())
            .map(|i| {
                ViewCell::editable(
                    answer
                        .map_or(Mark::Unmarked, |a| {
                            a.get(i / puzzle.cols(), i % puzzle.cols())
                        })
                        .to_char()
                        .to_string(),
                )
            })
            .collect();
        view.marks = vec![
            MarkOption::new(".", "Unmarked", ""),
            MarkOption::shaded("#", "Shaded"),
            MarkOption::new("-", "Unshaded", "×"),
        ];
        view.finish_note = "Blanks count as unshaded.".into();
        view.instructions="Shade cells so each row adds up to its clue using the column weights above. Each column adds up to its clue using the row weights on the left.".into();
        view
    }
    fn answer_from_values(puzzle: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != puzzle.rows() * puzzle.cols() {
            return Err(ParseError::new(
                "the number of marks must match the puzzle's cells",
            ));
        }
        let mut answer = Answer::new(puzzle.shape());
        for (i, value) in values.iter().enumerate() {
            let mark = match value.as_str() {
                "." => Mark::Unmarked,
                "#" => Mark::Shaded,
                "-" => Mark::Unshaded,
                _ => {
                    return Err(ParseError::new(
                        "use unmarked, shaded or unshaded cell values",
                    ));
                }
            };
            answer.set(i / puzzle.cols(), i % puzzle.cols(), mark);
        }
        Ok(answer)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
