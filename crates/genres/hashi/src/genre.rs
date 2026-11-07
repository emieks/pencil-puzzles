//! Hashi's shared contract and bridge presentation.
use crate::board::Layout;
use crate::model::{LIMITS, MAX_NUMBER, read_document};
use crate::*;
use pencil_puzzles_core::{
    ClueInput, Generation, Genre, MarkOption, Metadata, PuzzleEditor, Rule, View, ViewCell,
    ViewLink, text::Document,
};

/// Hashi: islands joined by bridges.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hashi;

static METADATA: Metadata = {
    let mut m = Metadata::new(
        "hashi",
        "Hashi",
        "Connect all islands with straight bridges that never cross, giving each island as many bridges as its number.",
        GENERATOR_VERSION,
        RATER_VERSION,
    );
    m.aliases = &["hashiwokakero", "bridges"];
    m.rules = &[
        Rule::new(1, "island numbers"),
        Rule::new(2, "no crossings"),
        Rule::new(3, "connected"),
    ];
    m
};

/// A slot's count as a link mark.
fn link_value(count: u8) -> String {
    if count == 0 {
        ".".into()
    } else {
        count.to_string()
    }
}

impl Genre for Hashi {
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
        text.parse::<Answer>()?.for_puzzle(p)
    }
    fn empty_answer(p: &Puzzle) -> Answer {
        let layout = Layout::new(p);
        Answer::from_counts(p, &layout, &vec![0; layout.slots.len()], false)
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
        let layout = Layout::new(p);
        let counts = answer
            .filter(|a| a.shape() == p.shape())
            .and_then(|a| a.counts(&layout).ok())
            .unwrap_or_else(|| vec![0; layout.slots.len()]);
        let sh = p.shape();
        let cols = sh.cols();
        let mut v = View::default();
        v.rows = sh.rows() as u32;
        v.cols = cols as u32;
        v.marks = vec![MarkOption::new(".", "Empty", "")];
        v.cells = p
            .clues()
            .iter()
            .map(|&n| {
                let mut cell = ViewCell::editable(if n == 0 { ".".into() } else { n.to_string() });
                cell.editable = false;
                cell.circled = n > 0;
                cell
            })
            .collect();
        v.link_marks = vec![
            MarkOption::link(".", "No bridge", 0),
            MarkOption::link("1", "One bridge", 1),
            MarkOption::link("2", "Two bridges", 2),
        ];
        v.links = layout
            .slots
            .iter()
            .zip(&counts)
            .map(|(slot, &count)| {
                let [a, b] = slot.ends.map(|k| layout.cells[k]);
                let at = |i: usize| ((i / cols) as u32, (i % cols) as u32);
                ViewLink::editable(at(a), at(b), link_value(count))
            })
            .collect();
        v.finish_note = "Bridges not drawn count as none.".into();
        v.instructions = "Join the circled islands with bridges.\nBridges run straight across or down between two islands and may not cross a bridge or another island. At most two bridges join the same pair.\nEach island has as many bridges as its number.\nThe bridges must join all islands into one group.".into();
        v
    }
    fn answer_from_values(p: &Puzzle, values: &[String]) -> Result<Answer, ParseError> {
        let layout = Layout::new(p);
        let cells = p.shape().cells();
        if values.len() != cells + layout.slots.len() {
            return Err(ParseError::new(
                "marks must match the Hashi grid and its bridges",
            ));
        }
        for (i, value) in values[..cells].iter().enumerate() {
            let n = p.clues()[i];
            let expected = if n == 0 { ".".into() } else { n.to_string() };
            if *value != expected {
                return Err(ParseError::new("islands and empty cells cannot be changed"));
            }
        }
        let counts = values[cells..]
            .iter()
            .map(|v| match v.as_str() {
                "." => Ok(0),
                "1" => Ok(1),
                "2" => Ok(2),
                _ => Err(ParseError::new("bridges are `.`, `1` or `2`")),
            })
            .collect::<Result<Vec<u8>, _>>()?;
        Ok(Answer::from_counts(p, &layout, &counts, false))
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
                    u32::from(MAX_NUMBER),
                    true,
                )
            })
            .collect();
        let mut view = Self::view(p, None);
        view.links.clear();
        PuzzleEditor::new(sh.to_string(), view, fields)
    }
    fn puzzle_from_editor(shape: Shape, values: &[String]) -> Result<Puzzle, ParseError> {
        if values.len() != shape.cells() {
            return Err(ParseError::new("clues must match the Hashi grid"));
        }
        let clues = values
            .iter()
            .map(|s| match s.as_str() {
                "" => Ok(0),
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" => Ok(s.as_bytes()[0] - b'0'),
                _ => Err(ParseError::new("island numbers go from 1 to 8")),
            })
            .collect::<Result<Vec<u8>, _>>()?;
        Puzzle::new(shape, clues)
    }
    fn render(puzzle: &Puzzle, answer: Option<&Answer>) -> String {
        puzzle.render(answer)
    }
}
