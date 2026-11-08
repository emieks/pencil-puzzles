//! Handles the web app's requests through the genre registry.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
use pencil_puzzles::{Budget, Difficulty, Input, RegistryError, Solutions, find, genres, read};
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    List,
    Generate {
        genre: String,
        params: String,
        difficulty: Option<Difficulty>,
        seed: String,
    },
    Editor {
        genre: String,
        shape: String,
        puzzle: Option<String>,
    },
    Compose {
        genre: String,
        shape: String,
        values: Vec<String>,
    },
    Load {
        genre: String,
        puzzle: String,
    },
    Solve {
        genre: String,
        puzzle: String,
    },
    Rate {
        genre: String,
        puzzle: String,
    },
    Check {
        genre: String,
        puzzle: String,
        marks: Vec<String>,
        finished: bool,
    },
}
fn input(genre: &str, puzzle: &str) -> Result<Input, RegistryError> {
    let mut inputs = read(puzzle)?;
    if inputs.len() != 1 {
        return Err("load one puzzle at a time".into());
    }
    let input = inputs.remove(0);
    let selected = find(genre).ok_or("unknown puzzle type")?;
    if input.genre.metadata().id != selected.metadata().id {
        return Err("the puzzle belongs to a different game".into());
    }
    Ok(input)
}
fn loaded(input: &Input) -> Result<Value, RegistryError> {
    let text = input.genre.with_solution(&input.text, None)?;
    let id = input.genre.id(&text)?;
    let (_, shape, _) = pencil_puzzles::id::split(&id)?;
    let params = input.genre.params_for_shape(shape)?;
    Ok(
        json!({"id":id,"params":params,"text":text,"view":input.genre.view(&text,None)?,"rating":input.rating}),
    )
}
/// Work allowed per request. A few 10x10 hard nurikabe puzzles need more
/// than 50 million operations; the page also stops a request after 15 s.
const WORK_BUDGET: u64 = 100_000_000;
fn execute(request: Request) -> Result<Value, RegistryError> {
    let budget = Budget::new(WORK_BUDGET);
    match request {
        Request::List => Ok(json!(
            genres()
                .iter()
                .map(|g| json!({"metadata":g.metadata(),"params":g.default_params()}))
                .collect::<Vec<_>>()
        )),
        Request::Generate {
            genre,
            params,
            difficulty,
            seed,
        } => {
            let genre = find(&genre).ok_or("unknown puzzle type")?;
            let generated = genre.generate(&params, difficulty, seed.parse()?, 10_000, &budget)?;
            loaded(&Input {
                genre,
                text: generated.text,
                rating: Some(generated.rating),
            })
        }
        Request::Editor {
            genre,
            shape,
            puzzle,
        } => {
            let g = find(&genre).ok_or("unknown puzzle type")?;
            let p = puzzle.map(|source| input(&genre, &source)).transpose()?;
            Ok(json!(
                g.editor(&shape, p.as_ref().map(|p| p.text.as_str()))?
            ))
        }
        Request::Compose {
            genre,
            shape,
            values,
        } => {
            let g = find(&genre).ok_or("unknown puzzle type")?;
            loaded(&Input {
                genre: g,
                text: g.puzzle_from_editor(&shape, &values)?,
                rating: None,
            })
        }
        Request::Load { genre, puzzle } => loaded(&input(&genre, &puzzle)?),
        Request::Solve { genre, puzzle } => {
            let input = input(&genre, &puzzle)?;
            let (status, answers) = match input.genre.solve(&input.text, &budget)? {
                Solutions::None => ("none", vec![]),
                Solutions::Unique(answer) => ("unique", vec![answer]),
                Solutions::Multiple(first, second) => ("multiple", vec![first, second]),
            };
            let views = answers
                .iter()
                .map(|a| input.genre.view(&input.text, Some(a)))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"puzzle":loaded(&input)?,"status":status,"answers":answers,"views":views}))
        }
        Request::Rate { genre, puzzle } => {
            let input = input(&genre, &puzzle)?;
            Ok(json!(input.genre.rate(&input.text, &budget)?))
        }
        Request::Check {
            genre,
            puzzle,
            marks,
            finished,
        } => {
            let input = input(&genre, &puzzle)?;
            let answer = input.genre.answer_from_values(&input.text, &marks)?;
            let violations = input
                .genre
                .check(&input.text, Some(&answer), finished, &budget)?;
            // Without `finished`, an answer is complete when every editable
            // cell is marked. Unmarked links are valid in a solution, so a
            // view with editable links is complete only when finished.
            let view = input.genre.view(&input.text, None)?;
            let complete = finished
                || (marks
                    .iter()
                    .zip(&view.cells)
                    .all(|(mark, cell)| !cell.editable || mark != &cell.value)
                    && view.links.iter().all(|link| !link.editable));
            Ok(json!({"violations":violations,"solved":complete && violations.is_empty()}))
        }
    }
}
/// Process a bounded JSON request. Errors are returned as data, never thrown.
#[wasm_bindgen]
pub fn request(source: &str) -> String {
    let result = if source.len() > 1_000_000 {
        Err("the puzzle input is too large".into())
    } else {
        serde_json::from_str::<Request>(source)
            .map_err(|e| -> RegistryError { Box::new(e) })
            .and_then(execute)
    };
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error.to_string()}),
    }
    .to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn call(value: Value) -> Value {
        serde_json::from_str(&request(&value.to_string())).unwrap()
    }
    #[test]
    fn browser_round_trip_and_solution_check() {
        let list = call(json!({"operation":"list"}));
        for game in list["value"].as_array().unwrap() {
            let genre = game["metadata"]["id"].as_str().unwrap();
            let generated = call(
                json!({"operation":"generate","genre":genre,"params":game["params"],"difficulty":"easy","seed":"42"}),
            );
            assert_eq!(generated["ok"], true);
            let puzzle = generated["value"]["id"].clone();
            let solved = call(json!({"operation":"solve","genre":genre,"puzzle":puzzle}));
            assert_eq!(solved["value"]["status"], "unique");
            // Cell values, then link values (I4).
            let view = &solved["value"]["views"][0];
            let marks: Vec<_> = ["cells", "links"]
                .iter()
                .flat_map(|layer| view[*layer].as_array().unwrap())
                .map(|c| c["value"].clone())
                .collect();
            let checked = call(
                json!({"operation":"check","genre":genre,"puzzle":puzzle,"marks":marks,"finished":true}),
            );
            assert_eq!(checked["value"]["solved"], true);
            let rated = call(json!({"operation":"rate","genre":genre,"puzzle":puzzle}));
            assert_eq!(
                rated["value"]["level"],
                generated["value"]["rating"]["level"]
            );
        }
    }
    #[test]
    fn medium_sudoku_fits_the_budget() {
        let generated = call(
            json!({"operation":"generate","genre":"sudoku","params":"9x9","difficulty":"medium","seed":"1"}),
        );
        assert_eq!(generated["ok"], true, "{generated}");
    }
    #[test]
    fn invalid_and_nonunique_inputs_are_reported() {
        assert_eq!(
            call(json!({"operation":"load","genre":"kakurasu","puzzle":"bad"}))["ok"],
            false
        );
        let result = call(
            json!({"operation":"solve","genre":"kakurasu","puzzle":"kakurasu:4x4:5.5.5.5/5.5.5.5"}),
        );
        assert_eq!(result["value"]["status"], "multiple");
        let result =
            call(json!({"operation":"solve","genre":"kakurasu","puzzle":"kakurasu:1x1:1/0"}));
        assert_eq!(result["value"]["status"], "none");
    }
}
