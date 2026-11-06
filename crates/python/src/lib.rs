//! The native module behind the Python package; the public API is in
//! `python/pencil_puzzles/__init__.py`.
#![forbid(unsafe_code)]
use pencil_puzzles::{fillomino, kakurasu, nurikabe, sudoku};
use pencil_puzzles_core::{Budget, Difficulty, Genre, RateError, Record, Solutions};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pyo3::create_exception!(_native, EngineError, pyo3::exceptions::PyException);

#[derive(Debug)]
struct Failure {
    kind: &'static str,
    message: String,
    attempts: Option<u32>,
}
impl Failure {
    fn new(kind: &'static str, message: impl ToString) -> Self {
        Self {
            kind,
            message: message.to_string(),
            attempts: None,
        }
    }
    fn py(self) -> PyErr {
        EngineError::new_err((self.kind, self.message, self.attempts))
    }
}
fn error(error: &(dyn Error + 'static)) -> Failure {
    if let Some(e) = error.downcast_ref::<RateError>() {
        return Failure::new(
            match e {
                RateError::NoSolution => "no_solution",
                RateError::MultipleSolutions => "multiple_solutions",
                RateError::BudgetExceeded => "budget",
                _ => "unrated",
            },
            e,
        );
    }
    if error.is::<pencil_puzzles_core::BudgetExceeded>() {
        return Failure::new("budget", error);
    }
    macro_rules! generation {
        ($ty:ty, $ex:path, $budget:path) => {
            if let Some(e) = error.downcast_ref::<$ty>() {
                return match e {
                    $ex { attempts } => Failure {
                        kind: "attempts",
                        message: e.to_string(),
                        attempts: Some(*attempts),
                    },
                    $budget => Failure::new("budget", e),
                    _ => Failure::new("unsupported", e),
                };
            }
        };
    }
    generation!(
        sudoku::GenerateError,
        sudoku::GenerateError::AttemptsExhausted,
        sudoku::GenerateError::BudgetExceeded
    );
    generation!(
        kakurasu::GenerateError,
        kakurasu::GenerateError::AttemptsExhausted,
        kakurasu::GenerateError::BudgetExceeded
    );
    generation!(
        fillomino::GenerateError,
        fillomino::GenerateError::AttemptsExhausted,
        fillomino::GenerateError::BudgetExceeded
    );
    generation!(
        nurikabe::GenerateError,
        nurikabe::GenerateError::AttemptsExhausted,
        nurikabe::GenerateError::BudgetExceeded
    );
    Failure::new("parse", error)
}

fn to_python(py: Python<'_>, value: Value) -> PyResult<Py<PyAny>> {
    Ok(match value {
        Value::Null => py.None(),
        Value::Bool(v) => v.into_pyobject(py)?.to_owned().into_any().unbind(),
        Value::String(v) => v.into_pyobject(py)?.into_any().unbind(),
        Value::Number(v) => {
            if let Some(n) = v.as_u64() {
                n.into_pyobject(py)?.into_any().unbind()
            } else if let Some(n) = v.as_i64() {
                n.into_pyobject(py)?.into_any().unbind()
            } else {
                v.as_f64().unwrap().into_pyobject(py)?.into_any().unbind()
            }
        }
        Value::Array(values) => {
            let list = PyList::empty(py);
            for value in values {
                list.append(to_python(py, value)?)?;
            }
            list.into_any().unbind()
        }
        Value::Object(values) => {
            let dict = PyDict::new(py);
            for (key, value) in values {
                dict.set_item(key, to_python(py, value)?)?;
            }
            dict.into_any().unbind()
        }
    })
}

#[pyclass(frozen, module = "pencil_puzzles._native")]
#[derive(Clone, Default)]
struct CancellationToken {
    flag: Arc<AtomicBool>,
}
#[pymethods]
impl CancellationToken {
    #[new]
    fn new() -> Self {
        Self::default()
    }
    fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }
    #[getter]
    fn cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}

// Work runs off the interpreter thread. Polling signals never changes selection:
// interruption sets the same cancellation token used at engine work checkpoints.
fn execute<T: Send + 'static>(
    py: Python<'_>,
    token: CancellationToken,
    work: impl FnOnce(Arc<AtomicBool>) -> Result<T, Failure> + Send + 'static,
) -> PyResult<T> {
    if token.cancelled() {
        return Err(Failure::new("cancelled", "operation cancelled").py());
    }
    let flag = token.flag.clone();
    let (send, receive) = mpsc::sync_channel(1);
    let receive = std::sync::Mutex::new(receive);
    let worker = std::thread::spawn(move || {
        let _ = send.send(work(flag));
    });
    loop {
        let received = py.detach(|| {
            receive
                .lock()
                .expect("single receiver")
                .recv_timeout(Duration::from_millis(20))
        });
        if let Err(signal) = py.check_signals() {
            token.cancel();
            let _ = py.detach(|| worker.join());
            return Err(signal);
        }
        match received {
            Ok(result) => {
                let _ = py.detach(|| worker.join());
                if token.cancelled() {
                    return Err(Failure::new("cancelled", "operation cancelled").py());
                }
                return result.map_err(Failure::py);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = worker.join();
                return Err(Failure::new("internal", "native worker failed").py());
            }
        }
    }
}

#[derive(Clone)]
enum PuzzleData {
    Sudoku(sudoku::Puzzle),
    Kakurasu(kakurasu::Puzzle),
    Nurikabe(nurikabe::Puzzle),
    Fillomino(fillomino::Puzzle),
}
#[pyclass(frozen, module = "pencil_puzzles._native")]
#[derive(Clone)]
struct NativePuzzle {
    data: PuzzleData,
}
macro_rules! dispatch {
    ($self:expr, $fun:ident $(, $arg:expr)*) => { match &$self.data {
        PuzzleData::Sudoku(p) => $fun::<sudoku::Sudoku>(p $(, $arg)*),
        PuzzleData::Kakurasu(p) => $fun::<kakurasu::Kakurasu>(p $(, $arg)*),
        PuzzleData::Nurikabe(p) => $fun::<nurikabe::Nurikabe>(p $(, $arg)*),
        PuzzleData::Fillomino(p) => $fun::<fillomino::Fillomino>(p $(, $arg)*),
    } };
}
fn puzzle_value<G: Genre>(p: &G::Puzzle) -> Value
where
    G::Puzzle: Serialize,
{
    json!({"genre": G::metadata().id, "id": G::id(p), "puzzle": p})
}
fn puzzle_text<G: Genre>(p: &G::Puzzle) -> String {
    G::write_record(&Record::new(p.clone()))
}
#[pymethods]
impl NativePuzzle {
    fn data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_python(py, dispatch!(self, puzzle_value))
    }
    fn to_text(&self) -> String {
        dispatch!(self, puzzle_text)
    }
    #[pyo3(signature=(operation, work, cancel, answer=None))]
    fn operate(
        &self,
        py: Python<'_>,
        operation: String,
        work: u64,
        cancel: CancellationToken,
        answer: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let puzzle = self.clone();
        let value = execute(py, cancel, move |flag| {
            let budget = Budget::with_cancellation(work, flag);
            if operation == "analyze"
                && let PuzzleData::Sudoku(p) = &puzzle.data
            {
                return sudoku::analyze(p, &budget)
                    .map(|a| {
                        json!({
                            "rating":a.rating,"steps":a.steps,"unresolved":a.unresolved
                        })
                    })
                    .map_err(|e| error(&e));
            }
            dispatch!(puzzle, operate, &operation, &budget, answer.as_deref())
        })?;
        to_python(py, value)
    }
}
fn operate<G: Genre>(
    p: &G::Puzzle,
    operation: &str,
    budget: &Budget,
    answer: Option<&str>,
) -> Result<Value, Failure>
where
    G::Answer: Serialize + DeserializeOwned,
{
    match operation {
        "rate" => G::rate(p, budget).map(|r| json!(r)).map_err(|e| error(&e)),
        "analyze" => G::rate_with_steps(p, budget)
            .map(|(r, s)| json!({"rating":r,"steps":s}))
            .map_err(|e| error(&e)),
        "solve" => match G::solve(p, budget).map_err(|e| error(&e))? {
            Solutions::None => Err(Failure::new("no_solution", "puzzle has no solution")),
            Solutions::Unique(a) => Ok(json!({"status":"unique", "solution":a})),
            Solutions::Multiple(a, b) => Ok(json!({"status":"multiple", "witnesses":[a,b]})),
        },
        "check" | "hint" => {
            let a: G::Answer = match answer {
                Some(source) => {
                    serde_json::from_str(source).map_err(|e| Failure::new("parse", e))?
                }
                None => G::empty_answer(p),
            };
            if operation == "check" {
                G::check(p, &a, budget)
                    .map(|v| json!(v))
                    .map_err(|e| error(e.as_ref()))
            } else {
                G::hint(p, &a, budget)
                    .map(|h| json!(h))
                    .map_err(|e| error(&e))
            }
        }
        _ => Err(Failure::new("unsupported", "unknown operation")),
    }
}

#[pyfunction]
fn from_id(id: &str) -> PyResult<NativePuzzle> {
    let name = id.split(':').next().unwrap_or_default();
    let genre = pencil_puzzles::find(name)
        .ok_or_else(|| Failure::new("unsupported", "unknown genre").py())?;
    let canonical = format!(
        "{}:{}",
        genre.metadata().id,
        id.split_once(':').map_or("", |p| p.1)
    );
    let data = match genre.metadata().id {
        "sudoku" => {
            PuzzleData::Sudoku(sudoku::Puzzle::from_id(&canonical).map_err(|e| error(&e).py())?)
        }
        "kakurasu" => {
            PuzzleData::Kakurasu(kakurasu::Puzzle::from_id(&canonical).map_err(|e| error(&e).py())?)
        }
        "nurikabe" => {
            PuzzleData::Nurikabe(nurikabe::Puzzle::from_id(&canonical).map_err(|e| error(&e).py())?)
        }
        "fillomino" => PuzzleData::Fillomino(
            fillomino::Puzzle::from_id(&canonical).map_err(|e| error(&e).py())?,
        ),
        _ => unreachable!(),
    };
    Ok(NativePuzzle { data })
}

#[pyfunction]
fn from_json(genre: &str, puzzle: &str) -> PyResult<NativePuzzle> {
    let data = match genre {
        "sudoku" => PuzzleData::Sudoku(
            serde_json::from_str(puzzle).map_err(|e| Failure::new("parse", e).py())?,
        ),
        "kakurasu" => PuzzleData::Kakurasu(
            serde_json::from_str(puzzle).map_err(|e| Failure::new("parse", e).py())?,
        ),
        "nurikabe" => PuzzleData::Nurikabe(
            serde_json::from_str(puzzle).map_err(|e| Failure::new("parse", e).py())?,
        ),
        "fillomino" => PuzzleData::Fillomino(
            serde_json::from_str(puzzle).map_err(|e| Failure::new("parse", e).py())?,
        ),
        _ => return Err(Failure::new("unsupported", "unknown genre").py()),
    };
    Ok(NativePuzzle { data })
}

#[pyfunction]
fn from_text(source: &str) -> PyResult<NativePuzzle> {
    let records = pencil_puzzles::read(source).map_err(|e| error(e.as_ref()).py())?;
    if records.len() != 1 {
        return Err(Failure::new("parse", "expected one puzzle").py());
    }
    let input = &records[0];
    from_id(
        &input
            .genre
            .id(&input.text)
            .map_err(|e| error(e.as_ref()).py())?,
    )
}

fn generate_typed<G: Genre>(
    shape: &str,
    difficulty: Option<Difficulty>,
    seed: u64,
    attempts: u32,
    budget: &Budget,
) -> Result<(G::Puzzle, Value), Failure>
where
    G::Answer: Serialize,
{
    let shape: G::Shape = shape.parse().map_err(|e| Failure::new("unsupported", e))?;
    let params = G::params_for_shape(shape);
    let g = G::generate(&params, difficulty, seed, attempts, budget).map_err(|e| error(&e))?;
    let value = json!({"solution":g.record.solution, "rating":g.rating,
        "attempts":g.attempts, "work":budget.spent(), "generator_version":G::metadata().generator_version});
    Ok((g.record.puzzle, value))
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
fn generate(
    py: Python<'_>,
    genre: String,
    shape: String,
    difficulty: Option<String>,
    seed: u64,
    attempts: u32,
    work: u64,
    version: Option<u32>,
    cancel: CancellationToken,
) -> PyResult<(NativePuzzle, Py<PyAny>)> {
    let g = pencil_puzzles::find(&genre)
        .ok_or_else(|| Failure::new("unsupported", "unknown genre").py())?;
    let name = g.metadata().id;
    if version.is_some_and(|v| v != g.metadata().generator_version) {
        return Err(
            Failure::new("unsupported", "generator version is retired or unsupported").py(),
        );
    }
    let level = difficulty
        .map(|d| d.parse::<Difficulty>())
        .transpose()
        .map_err(|e| Failure::new("unsupported", e).py())?;
    let (data, value) = execute(py, cancel, move |flag| {
        let budget = Budget::with_cancellation(work, flag);
        macro_rules! go {
            ($g:ty,$variant:path) => {{
                let (p, v) = generate_typed::<$g>(&shape, level, seed, attempts, &budget)?;
                Ok(($variant(p), v))
            }};
        }
        match name {
            "sudoku" => go!(sudoku::Sudoku, PuzzleData::Sudoku),
            "kakurasu" => go!(kakurasu::Kakurasu, PuzzleData::Kakurasu),
            "nurikabe" => go!(nurikabe::Nurikabe, PuzzleData::Nurikabe),
            "fillomino" => go!(fillomino::Fillomino, PuzzleData::Fillomino),
            _ => unreachable!(),
        }
    })?;
    Ok((NativePuzzle { data }, to_python(py, value)?))
}

#[pyfunction]
fn metadata(py: Python<'_>) -> PyResult<Py<PyAny>> {
    to_python(
        py,
        json!(
            pencil_puzzles::genres()
                .iter()
                .map(|g| g.metadata())
                .collect::<Vec<_>>()
        ),
    )
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("EngineError", m.py().get_type::<EngineError>())?;
    m.add_class::<CancellationToken>()?;
    m.add_class::<NativePuzzle>()?;
    m.add_function(wrap_pyfunction!(from_id, m)?)?;
    m.add_function(wrap_pyfunction!(from_text, m)?)?;
    m.add_function(wrap_pyfunction!(from_json, m)?)?;
    m.add_function(wrap_pyfunction!(generate, m)?)?;
    m.add_function(wrap_pyfunction!(metadata, m)?)?;
    Ok(())
}
