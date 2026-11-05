//! Nurikabe's common contract and binary cell presentation.
use crate::model::read_document;
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, Status, View, ViewCell,
    text::Document,
};
/// Numbered islands surrounded by a connected shaded sea.
#[derive(Clone, Copy, Debug, Default)]
pub struct Nurikabe;
static METADATA: Metadata = {
    let mut m = Metadata::new(
        "nurikabe",
        "Nurikabe",
        "Shade a connected sea around numbered islands, without shaded 2x2 blocks.",
        Status::Stable,
        GENERATOR_VERSION,
        RATER_VERSION,
    );
    m.rules = &[
        Rule::new(1, "island size"),
        Rule::new(2, "one number per island"),
        Rule::new(3, "connected sea"),
        Rule::new(4, "no pools"),
        Rule::new(5, "printed cells"),
    ];
    m
};
impl Genre for Nurikabe {
    fn text_limits() -> pencil_puzzles_core::text::GridLimits {
        pencil_puzzles_core::text::GridLimits::new(8, 24)
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
        for (i, &n) in p.clues().iter().enumerate() {
            if n > 0 && a.get(i / p.shape().cols(), i % p.shape().cols()) == Mark::Unmarked {
                a.set(i / p.shape().cols(), i % p.shape().cols(), Mark::Unshaded);
            }
        }
        Ok(a)
    }
    fn empty_answer(p: &Puzzle) -> Answer {
        let mut a = Answer::new(p.shape());
        for (i, &n) in p.clues().iter().enumerate() {
            if n > 0 {
                a.set(i / p.shape().cols(), i % p.shape().cols(), Mark::Unshaded);
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
        let mut v = View::default();
        let sh = p.shape();
        v.rows = sh.rows() as u32;
        v.cols = sh.cols() as u32;
        v.marks = vec![
            MarkOption::new(".", "Unmarked", ""),
            MarkOption::shaded("#", "Sea"),
            MarkOption::new("-", "Island", "\u{00b7}"),
        ];
        v.cells = p
            .clues()
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                let mut cell = ViewCell::editable(if n > 0 {
                    n.to_string()
                } else {
                    answer
                        .map_or(Mark::Unmarked, |a| a.get(i / sh.cols(), i % sh.cols()))
                        .to_char()
                        .to_string()
                });
                cell.editable = n == 0;
                cell
            })
            .collect();
        v.finish_note = "Blanks count as island cells.".into();
        v.instructions="Each island contains one number and exactly that many cells. Shade one connected sea. No shaded 2x2 blocks. Islands can touch at corners.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != p.shape().cells() {
            return Err(ParseError::new("marks must match the Nurikabe grid"));
        }
        let mut a = Answer::new(p.shape());
        for (i, s) in values.iter().enumerate() {
            let n = p.clues()[i];
            let mark = if n > 0 {
                if s != "." && s != "-" && s != &n.to_string() {
                    return Err(ParseError::new(
                        "numbered cells cannot be shaded or changed",
                    ));
                }
                Mark::Unshaded
            } else {
                match s.as_str() {
                    "." => Mark::Unmarked,
                    "#" => Mark::Shaded,
                    "-" => Mark::Unshaded,
                    _ => return Err(ParseError::new("use unmarked, sea or island marks")),
                }
            };
            a.set(i / p.shape().cols(), i % p.shape().cols(), mark);
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
                    Place::Cell {
                        row: (i / sh.cols()) as u32,
                        col: (i % sh.cols()) as u32,
                    },
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
            return Err(ParseError::new("clues must match the Nurikabe grid"));
        }
        let mut clues = Vec::with_capacity(shape.cells());
        for s in values {
            if s.is_empty() {
                clues.push(0);
            } else {
                if s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(ParseError::new("use positive whole-number clues"));
                }
                clues.push(
                    s.parse::<u32>()
                        .map_err(|_| ParseError::new("clue is too large"))?,
                );
            }
        }
        Puzzle::new(shape, clues)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
