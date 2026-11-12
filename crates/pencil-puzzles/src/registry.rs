//! Object-safe text and JSON adaptation for the typed Genre interface (I1/I2).
use pencil_puzzles_core::{
    Budget, Difficulty, Hint, Metadata, ParseError, Rating, Solutions, Violation,
    text::{self, Document},
};
use std::error::Error;
/// Actionable failure from a type-erased operation.
pub type RegistryError = Box<dyn Error + Send + Sync>;
/// A generated text record and its full rating.
#[derive(Debug)]
#[non_exhaustive]
pub struct Generated {
    /// Canonical text including a complete solution and reproduction metadata.
    pub text: String,
    /// The complete L18 rating.
    pub rating: Rating,
    /// Candidates tried.
    pub attempts: u32,
}
/// An object-safe adapter. Text records carry shared metadata between operations.
/// All parsing is validated, and never invokes a solver or rater (F6).
pub trait DynGenre: Sync {
    /// Genre-specific limits for grid and solution text blocks.
    fn text_limits(&self) -> text::GridLimits;

    /// Metadata for listing and discovery.
    fn metadata(&self) -> &'static Metadata;
    /// Default parameters in their stable text encoding.
    fn default_params(&self) -> String;
    /// Validate shape text and supply default generator settings.
    fn params_for_shape(&self, shape: &str) -> Result<String, RegistryError>;
    /// Read one document and write canonical text, preserving shared keys.
    fn parse_document(&self, document: Document) -> Result<String, RegistryError>;
    /// Decode a puzzle ID to canonical text.
    fn read_id(&self, id: &str) -> Result<String, RegistryError>;
    /// Encode the puzzle's stable ID.
    fn id(&self, text: &str) -> Result<String, RegistryError>;
    /// Whether `generate` accepts these parameters and level.
    fn supports(&self, params: &str, difficulty: Option<Difficulty>)
    -> Result<bool, RegistryError>;
    /// Generate a unique puzzle; limits decide success, never which puzzle.
    fn generate(
        &self,
        params: &str,
        difficulty: Option<Difficulty>,
        seed: u64,
        max_attempts: u32,
        budget: &Budget,
    ) -> Result<Generated, RegistryError>;
    /// Solve with proof of uniqueness; witnesses are answer text.
    fn solve(&self, text: &str, budget: &Budget) -> Result<Solutions<String>, RegistryError>;
    /// Rate a unique puzzle.
    fn rate(&self, text: &str, budget: &Budget) -> Result<Rating, RegistryError>;
    /// Rate and list canonical deductions (P9, L19).
    fn rate_with_steps(
        &self,
        text: &str,
        budget: &Budget,
    ) -> Result<(Rating, Vec<pencil_puzzles_core::Step>), RegistryError>;
    /// Check explicit answer text, or the record's solution. Finished marks use P4 defaults.
    fn check(
        &self,
        text: &str,
        answer: Option<&str>,
        finished: bool,
        budget: &Budget,
    ) -> Result<Vec<Violation>, RegistryError>;
    /// Hint from explicit marks, the recorded solution, or an empty answer.
    fn hint(
        &self,
        text: &str,
        answer: Option<&str>,
        budget: &Budget,
    ) -> Result<Hint, RegistryError>;
    /// Replace or omit the solution while preserving the other shared keys.
    fn with_solution(&self, text: &str, answer: Option<&str>) -> Result<String, RegistryError>;
    /// The optional complete solution in a text record.
    fn solution(&self, text: &str) -> Result<Option<String>, RegistryError>;
    /// Set the record's level and score.
    fn with_rating(&self, text: &str, rating: &Rating) -> Result<String, RegistryError>;
    /// Build a blank editor or populate it from an existing puzzle.
    fn editor(
        &self,
        shape: &str,
        puzzle: Option<&str>,
    ) -> Result<pencil_puzzles_core::PuzzleEditor, RegistryError>;
    /// Turn validated visual clues into a canonical puzzle record.
    fn puzzle_from_editor(&self, shape: &str, values: &[String]) -> Result<String, RegistryError>;
    /// Build a browser cell view using the genre's presentation (I4).
    fn view(
        &self,
        text: &str,
        answer: Option<&str>,
    ) -> Result<pencil_puzzles_core::View, RegistryError>;
    /// Validate client mark values and write canonical answer text.
    fn answer_from_values(&self, text: &str, values: &[String]) -> Result<String, RegistryError>;
    /// Draw a validated puzzle, optionally with explicit answer text.
    fn render(&self, text: &str, answer: Option<&str>) -> Result<String, RegistryError>;
    /// Encode the F14 envelope; seeds are strings, and the full rating is optional.
    #[cfg(feature = "serde")]
    fn to_json(
        &self,
        text: &str,
        rating: Option<&Rating>,
    ) -> Result<serde_json::Value, RegistryError>;
    /// Decode a strict F14 envelope, validating the puzzle and solution without solving.
    #[cfg(feature = "serde")]
    fn read_json(&self, value: serde_json::Value) -> Result<Input, RegistryError>;
}
/// A validated input record and the registered genre that interprets it.
pub struct Input {
    /// The genre adapter.
    pub genre: &'static dyn DynGenre,
    /// Canonical text, preserving shared metadata.
    pub text: String,
    /// A complete rating when supplied in a JSON envelope.
    pub rating: Option<Rating>,
}
// The typed adapter is only needed when a genre is compiled in. Add future
// genre features here as they join the registry.
#[cfg(any(
    feature = "aquarium",
    feature = "binairo",
    feature = "fillomino",
    feature = "hashi",
    feature = "hitori",
    feature = "kakurasu",
    feature = "kakuro",
    feature = "sudoku",
    feature = "nurikabe"
))]
mod typed {
    use super::*;
    use pencil_puzzles_core::{Genre, Record};
    use std::marker::PhantomData;
    fn one<G: Genre>(source: &str) -> Result<Record<G::Puzzle, G::Answer>, RegistryError> {
        let mut docs = text::parse_with_limits(source, G::text_limits())?;
        if docs.len() != 1 {
            return Err(ParseError::new("expected exactly one puzzle").into());
        }
        let mut doc = docs.remove(0);
        if doc
            .genre()
            .is_some_and(|id| G::metadata().aliases.contains(&id))
        {
            doc.set_genre(G::metadata().id);
        }
        Ok(G::parse_document(doc)?)
    }
    fn answer<G: Genre>(
        record: &Record<G::Puzzle, G::Answer>,
        source: Option<&str>,
    ) -> Result<G::Answer, RegistryError> {
        match source {
            Some(source) => Ok(G::parse_answer(&record.puzzle, source)?),
            None => record.solution.clone().ok_or_else(|| {
                ParseError::new("provide an answer file or a solution block").into()
            }),
        }
    }
    #[cfg(feature = "serde")]
    type GenreRecord<G> = Record<<G as Genre>::Puzzle, <G as Genre>::Answer>;
    #[cfg(feature = "serde")]
    type Decoded<G> = (GenreRecord<G>, Option<Rating>);

    trait WireGenre: Genre {
        #[cfg(feature = "serde")]
        fn encode(
            record: &Record<Self::Puzzle, Self::Answer>,
            rating: Option<&Rating>,
        ) -> Result<serde_json::Value, RegistryError>;
        #[cfg(feature = "serde")]
        fn decode(value: serde_json::Value) -> Result<Decoded<Self>, RegistryError>;
    }
    #[cfg(not(feature = "serde"))]
    impl<G: Genre> WireGenre for G {}
    #[cfg(feature = "serde")]
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(
        deny_unknown_fields,
        bound(deserialize = "P: serde::Deserialize<'de>, A: serde::Deserialize<'de>")
    )]
    struct Envelope<P, A> {
        genre: String,
        puzzle: P,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        solution: Option<A>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        rating: Option<Rating>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        seed: Option<String>,
        /// Read from older envelopes and not used.
        #[serde(rename = "generator_version", skip_serializing, default)]
        _generator_version: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        params: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        difficulty: Option<Difficulty>,
    }
    #[cfg(feature = "serde")]
    impl<G> WireGenre for G
    where
        G: Genre,
        G::Puzzle: serde::Serialize + serde::de::DeserializeOwned,
        G::Answer: serde::Serialize + serde::de::DeserializeOwned,
    {
        fn encode(
            record: &Record<Self::Puzzle, Self::Answer>,
            rating: Option<&Rating>,
        ) -> Result<serde_json::Value, RegistryError> {
            Ok(serde_json::to_value(Envelope {
                genre: G::metadata().id.into(),
                puzzle: &record.puzzle,
                solution: record.solution.as_ref(),
                rating: rating.cloned(),
                seed: record.shared.seed.map(|s| s.to_string()),
                _generator_version: None,
                params: record.shared.params.clone(),
                difficulty: record.shared.difficulty,
            })?)
        }
        fn decode(value: serde_json::Value) -> Result<Decoded<Self>, RegistryError> {
            let envelope: Envelope<G::Puzzle, G::Answer> = serde_json::from_value(value)?;
            let m = G::metadata();
            if envelope.genre != m.id && !m.aliases.contains(&envelope.genre.as_str()) {
                return Err(ParseError::new("the envelope names a different genre").into());
            }
            let mut record = Record::new(envelope.puzzle);
            record.solution = envelope.solution;
            record.shared.seed = envelope.seed.map(|s| s.parse::<u64>()).transpose()?;
            record.shared.params = envelope.params;
            record.shared.difficulty = envelope.difficulty;
            if let Some(r) = &envelope.rating {
                record.shared.rating = Some((r.level, r.score));
            }
            // Reuse the genre's validation of solution shape/completeness, without solving.
            let record = one::<G>(&G::write_record(&record))?;
            Ok((record, envelope.rating))
        }
    }
    pub(super) struct Adapter<G>(PhantomData<fn() -> G>);
    impl<G: WireGenre> DynGenre for Adapter<G> {
        fn metadata(&self) -> &'static Metadata {
            G::metadata()
        }
        fn text_limits(&self) -> text::GridLimits {
            G::text_limits()
        }
        fn default_params(&self) -> String {
            G::Params::default().to_string()
        }
        fn params_for_shape(&self, shape: &str) -> Result<String, RegistryError> {
            Ok(G::params_for_shape(shape.parse()?).to_string())
        }
        fn parse_document(&self, mut document: Document) -> Result<String, RegistryError> {
            if let Some(id) = document.genre()
                && id != G::metadata().id
                && !G::metadata().aliases.contains(&id)
            {
                return Err(ParseError::at_line(
                    document.start_line(),
                    "the document names a different genre",
                )
                .into());
            }
            document.set_genre(G::metadata().id);
            Ok(G::write_record(&G::parse_document(document)?))
        }
        fn read_id(&self, id: &str) -> Result<String, RegistryError> {
            let (name, shape, description) = pencil_puzzles_core::id::split(id)?;
            if name != G::metadata().id && !G::metadata().aliases.contains(&name) {
                return Err(ParseError::at_char(1, "the ID names a different genre").into());
            }
            let canonical = pencil_puzzles_core::id::join(G::metadata().id, shape, description);
            Ok(G::write_record(&Record::new(G::from_id(&canonical)?)))
        }
        fn id(&self, text: &str) -> Result<String, RegistryError> {
            Ok(G::id(&one::<G>(text)?.puzzle))
        }
        fn supports(
            &self,
            params: &str,
            difficulty: Option<Difficulty>,
        ) -> Result<bool, RegistryError> {
            Ok(G::supports(&params.parse()?, difficulty))
        }
        fn generate(
            &self,
            params: &str,
            difficulty: Option<Difficulty>,
            seed: u64,
            max_attempts: u32,
            budget: &Budget,
        ) -> Result<Generated, RegistryError> {
            let g = G::generate(&params.parse()?, difficulty, seed, max_attempts, budget)?;
            Ok(Generated {
                text: G::write_record(&g.record),
                rating: g.rating,
                attempts: g.attempts,
            })
        }
        fn solve(&self, text: &str, budget: &Budget) -> Result<Solutions<String>, RegistryError> {
            Ok(G::solve(&one::<G>(text)?.puzzle, budget)?.map(|a| a.to_string()))
        }
        fn rate(&self, text: &str, budget: &Budget) -> Result<Rating, RegistryError> {
            Ok(G::rate(&one::<G>(text)?.puzzle, budget)?)
        }
        fn rate_with_steps(
            &self,
            text: &str,
            budget: &Budget,
        ) -> Result<(Rating, Vec<pencil_puzzles_core::Step>), RegistryError> {
            Ok(G::rate_with_steps(&one::<G>(text)?.puzzle, budget)?)
        }
        fn check(
            &self,
            text: &str,
            source: Option<&str>,
            finished: bool,
            budget: &Budget,
        ) -> Result<Vec<Violation>, RegistryError> {
            let r = one::<G>(text)?;
            let a = answer::<G>(&r, source)?;
            let a = if finished { G::finished(&a) } else { a };
            G::check(&r.puzzle, &a, budget)
        }
        fn hint(
            &self,
            text: &str,
            source: Option<&str>,
            budget: &Budget,
        ) -> Result<Hint, RegistryError> {
            let r = one::<G>(text)?;
            let a = if source.is_none() && r.solution.is_none() {
                G::empty_answer(&r.puzzle)
            } else {
                answer::<G>(&r, source)?
            };
            Ok(G::hint(&r.puzzle, &a, budget)?)
        }
        fn with_solution(&self, text: &str, source: Option<&str>) -> Result<String, RegistryError> {
            let mut r = one::<G>(text)?;
            r.solution = source.map(|s| G::parse_answer(&r.puzzle, s)).transpose()?;
            let text = G::write_record(&r);
            let _ = one::<G>(&text)?;
            Ok(text)
        }
        fn solution(&self, text: &str) -> Result<Option<String>, RegistryError> {
            Ok(one::<G>(text)?.solution.map(|a| a.to_string()))
        }
        fn with_rating(&self, text: &str, rating: &Rating) -> Result<String, RegistryError> {
            let mut r = one::<G>(text)?;
            r.shared.rating = Some((rating.level, rating.score));
            Ok(G::write_record(&r))
        }
        fn editor(
            &self,
            shape: &str,
            puzzle: Option<&str>,
        ) -> Result<pencil_puzzles_core::PuzzleEditor, RegistryError> {
            let p = puzzle.map(one::<G>).transpose()?;
            Ok(G::editor(shape.parse()?, p.as_ref().map(|r| &r.puzzle)))
        }
        fn puzzle_from_editor(
            &self,
            shape: &str,
            values: &[String],
        ) -> Result<String, RegistryError> {
            Ok(G::write_record(&Record::new(G::puzzle_from_editor(
                shape.parse()?,
                values,
            )?)))
        }
        fn view(
            &self,
            text: &str,
            source: Option<&str>,
        ) -> Result<pencil_puzzles_core::View, RegistryError> {
            let r = one::<G>(text)?;
            let answer = source.map(|s| G::parse_answer(&r.puzzle, s)).transpose()?;
            Ok(G::view(&r.puzzle, answer.as_ref()))
        }
        fn answer_from_values(
            &self,
            text: &str,
            values: &[String],
        ) -> Result<String, RegistryError> {
            let r = one::<G>(text)?;
            Ok(G::answer_from_values(&r.puzzle, values)?.to_string())
        }
        fn render(&self, text: &str, source: Option<&str>) -> Result<String, RegistryError> {
            let r = one::<G>(text)?;
            let a = source.map(|s| G::parse_answer(&r.puzzle, s)).transpose()?;
            Ok(G::render(&r.puzzle, a.as_ref()))
        }
        #[cfg(feature = "serde")]
        fn to_json(
            &self,
            text: &str,
            rating: Option<&Rating>,
        ) -> Result<serde_json::Value, RegistryError> {
            G::encode(&one::<G>(text)?, rating)
        }
        #[cfg(feature = "serde")]
        fn read_json(&self, value: serde_json::Value) -> Result<Input, RegistryError> {
            let (r, rating) = G::decode(value)?;
            Ok(Input {
                genre: find(G::metadata().id)
                    .ok_or_else(|| ParseError::new("genre is not registered"))?,
                text: G::write_record(&r),
                rating,
            })
        }
    }
    #[cfg(feature = "aquarium")]
    pub(super) static AQUARIUM: Adapter<crate::aquarium::Aquarium> = Adapter(PhantomData);
    #[cfg(feature = "binairo")]
    pub(super) static BINAIRO: Adapter<crate::binairo::Binairo> = Adapter(PhantomData);
    #[cfg(feature = "fillomino")]
    pub(super) static FILLOMINO: Adapter<crate::fillomino::Fillomino> = Adapter(PhantomData);
    #[cfg(feature = "hashi")]
    pub(super) static HASHI: Adapter<crate::hashi::Hashi> = Adapter(PhantomData);
    #[cfg(feature = "hitori")]
    pub(super) static HITORI: Adapter<crate::hitori::Hitori> = Adapter(PhantomData);
    #[cfg(feature = "kakurasu")]
    pub(super) static KAKURASU: Adapter<crate::kakurasu::Kakurasu> = Adapter(PhantomData);
    #[cfg(feature = "kakuro")]
    pub(super) static KAKURO: Adapter<crate::kakuro::Kakuro> = Adapter(PhantomData);
    #[cfg(feature = "nurikabe")]
    pub(super) static NURIKABE: Adapter<crate::nurikabe::Nurikabe> = Adapter(PhantomData);
    #[cfg(feature = "sudoku")]
    pub(super) static SUDOKU: Adapter<crate::sudoku::Sudoku> = Adapter(PhantomData);
}
/// Every compiled-in genre, in canonical id order (I1, K4).
static GENRES: &[&dyn DynGenre] = &[
    #[cfg(feature = "aquarium")]
    &typed::AQUARIUM,
    #[cfg(feature = "binairo")]
    &typed::BINAIRO,
    #[cfg(feature = "fillomino")]
    &typed::FILLOMINO,
    #[cfg(feature = "hashi")]
    &typed::HASHI,
    #[cfg(feature = "hitori")]
    &typed::HITORI,
    #[cfg(feature = "kakurasu")]
    &typed::KAKURASU,
    #[cfg(feature = "kakuro")]
    &typed::KAKURO,
    #[cfg(feature = "nurikabe")]
    &typed::NURIKABE,
    #[cfg(feature = "sudoku")]
    &typed::SUDOKU,
];
/// Every compiled-in genre, in canonical id order (I1, K4).
pub fn genres() -> &'static [&'static dyn DynGenre] {
    GENRES
}
/// Find a genre by its canonical id or any alias (N2).
pub fn find(name: &str) -> Option<&'static dyn DynGenre> {
    genres()
        .iter()
        .copied()
        .find(|g| g.metadata().id == name || g.metadata().aliases.contains(&name))
}
/// Parse IDs, text records (including mixed-genre files), or F14 JSON envelopes.
/// Text without a genre is rejected because the registry cannot infer it (I2).
pub fn read(source: &str) -> Result<Vec<Input>, RegistryError> {
    let trimmed = source.trim();
    #[cfg(feature = "serde")]
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let value: serde_json::Value = serde_json::from_str(trimmed)?;
        let values = match value {
            serde_json::Value::Array(values) => values,
            value => vec![value],
        };
        return values
            .into_iter()
            .map(|value| {
                let id = value
                    .get("genre")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ParseError::new("the JSON envelope needs a genre"))?;
                let g = find(id).ok_or_else(|| ParseError::new(format!("unknown genre {id:?}")))?;
                g.read_json(value)
            })
            .collect();
    }
    if !trimmed.contains('\n') && trimmed.matches(':').count() >= 2 {
        let (id, _, _) = pencil_puzzles_core::id::split(trimmed)?;
        let genre = find(id).ok_or_else(|| ParseError::new(format!("unknown genre {id:?}")))?;
        return Ok(vec![Input {
            genre,
            text: genre.read_id(trimmed)?,
            rating: None,
        }]);
    }
    let docs = text::parse_with_genre_limits(source, |id| {
        id.and_then(find)
            .map_or_else(text::GridLimits::default, |g| g.text_limits())
    })?;
    if docs.is_empty() {
        return Err(ParseError::new("there is no puzzle in the input").into());
    }
    docs.into_iter()
        .map(|d| {
            let id = d.genre().ok_or_else(|| {
                ParseError::at_line(d.start_line(), "each puzzle needs a genre: line")
            })?;
            let genre = find(id).ok_or_else(|| {
                ParseError::at_line(d.start_line(), format!("unknown genre {id:?}"))
            })?;
            Ok(Input {
                genre,
                text: genre.parse_document(d)?,
                rating: None,
            })
        })
        .collect()
}
