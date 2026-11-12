//! Aquarium's shared contract and its view with tanks and clues.
use crate::model::{LIMITS, read_document};
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, View, ViewCell,
    text::Document,
};

/// Aquarium: tanks filled with water to one level each.
#[derive(Clone, Copy, Debug, Default)]
pub struct Aquarium;

static METADATA: Metadata = {
    let mut m = Metadata::new(
        "aquarium",
        "Aquarium",
        "Fill tanks with water to match the row and column clues; the water in a tank is level.",
    );
    m.rules = &[
        Rule::new(1, "row counts"),
        Rule::new(2, "column counts"),
        Rule::new(3, "one level"),
    ];
    m
};

/// What an editor cell may hold: a tank letter or number.
const TANK_PATTERN: &str = "[A-Za-z]|[1-9][0-9]{0,2}";

impl Genre for Aquarium {
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
        v.right = p.row_clues().iter().map(u32::to_string).collect();
        v.bottom = p.col_clues().iter().map(u32::to_string).collect();
        let mut water = MarkOption::shaded("#", "Water");
        water.tone = "water".into();
        v.marks = vec![
            MarkOption::new(".", "Unmarked", ""),
            water,
            MarkOption::new("-", "Air", "\u{00d7}"),
        ];
        v.cells = p
            .tanks()
            .iter()
            .enumerate()
            .map(|(i, &tank)| {
                let mark = answer.map_or(Mark::Unmarked, |a| a.get(i / sh.cols(), i % sh.cols()));
                let mut cell = ViewCell::editable(mark.to_char().to_string());
                cell.region = Some(tank);
                cell
            })
            .collect();
        v.finish_note = "Blanks count as air.".into();
        v.instructions = "Fill cells with water.\nThe clue to the right of a row and the clue below a column count its water cells.\nThe outlined areas are tanks. The water in a tank has one level: under and beside a water cell, every cell of the same tank is water.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        if values.len() != p.shape().cells() {
            return Err(ParseError::new("marks must match the Aquarium grid"));
        }
        let cols = p.shape().cols();
        let mut a = Answer::new(p.shape());
        for (i, s) in values.iter().enumerate() {
            let mark = match s.as_str() {
                "." => Mark::Unmarked,
                "#" => Mark::Shaded,
                "-" => Mark::Unshaded,
                _ => return Err(ParseError::new("use unmarked, water or air marks")),
            };
            a.set(i / cols, i % cols, mark);
        }
        Ok(a)
    }
    /// The fields are the row clues, the column clues and then a tank
    /// letter or number for every cell.
    fn editor(shape: Shape, puzzle: Option<&Puzzle>) -> PuzzleEditor {
        let sh = puzzle.map_or(shape, Puzzle::shape);
        let blank = Puzzle::new(
            sh,
            &vec![0; sh.cells()],
            vec![0; sh.rows()],
            vec![0; sh.cols()],
        )
        .expect("validated shape");
        let p = puzzle.unwrap_or(&blank);
        let clue = |n: &u32| puzzle.map_or(String::new(), |_| n.to_string());
        let regions = p.regions();
        let spaced = regions[0].contains(' ');
        let labels = regions.iter().flat_map(|row| -> Vec<String> {
            if spaced {
                row.split(' ').map(String::from).collect()
            } else {
                row.chars().map(String::from).collect()
            }
        });
        let fields = (p.row_clues().iter().enumerate())
            .map(|(r, n)| ClueInput::new(Place::Row(r as u32), clue(n), 0, sh.cols() as u32, false))
            .chain(p.col_clues().iter().enumerate().map(|(c, n)| {
                ClueInput::new(Place::Col(c as u32), clue(n), 0, sh.rows() as u32, false)
            }))
            .chain(labels.enumerate().map(|(i, label)| {
                let value = puzzle.map_or(String::new(), |_| label);
                ClueInput::text(crate::board::cell(i, sh.cols()), value, TANK_PATTERN, false)
            }))
            .collect();
        let mut view = Self::view(p, None);
        view.right.fill(String::new());
        view.bottom.fill(String::new());
        view.finish_note.clear();
        PuzzleEditor::new(sh.to_string(), view, fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        let (rows, cols) = (shape.rows(), shape.cols());
        if values.len() != rows + cols + shape.cells() {
            return Err(ParseError::new(
                "enter a clue for every row and column and a tank for every cell",
            ));
        }
        let clues = |values: &[String]| {
            values
                .iter()
                .map(|s| {
                    (!s.is_empty() && s.len() <= 2 && s.bytes().all(|b| b.is_ascii_digit()))
                        .then(|| s.parse::<u32>().ok())
                        .flatten()
                        .ok_or_else(|| ParseError::new("every clue must be a whole number"))
                })
                .collect::<Result<Vec<u32>, _>>()
        };
        let row_clues = clues(&values[..rows])?;
        let col_clues = clues(&values[rows..rows + cols])?;
        let mut names = std::collections::BTreeMap::new();
        let labels = values[rows + cols..]
            .iter()
            .map(|token| crate::model::label(&mut names, token))
            .collect::<Result<Vec<u32>, _>>()?;
        Puzzle::new(shape, &labels, row_clues, col_clues)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
