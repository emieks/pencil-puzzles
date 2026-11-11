//! Reproducible generation: a shuffled Latin square, a random shading whose
//! cells copy a number from their row or column, then edits until the human
//! techniques solve the puzzle at exactly the requested level.
use crate::human::{analyze_state, initial_masks, outcome};
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::grid::{flood, neighbours};
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::{error::Error, fmt};

/// Reproducible generator inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GenerateOptions {
    /// Shape and generator parameters.
    pub params: Params,
    /// Exact requested level, or any level from Easy to Expert.
    pub difficulty: Option<Difficulty>,
    /// Reproducible seed.
    pub seed: u64,
    /// Maximum candidates screened.
    pub max_attempts: u32,
}

impl GenerateOptions {
    /// Options accepting Easy through Expert, at most 10,000 candidates.
    pub fn new(params: Params, seed: u64) -> Self {
        Self {
            params,
            difficulty: None,
            seed,
            max_attempts: 10_000,
        }
    }
}

/// A generated puzzle and its solution.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Generated {
    /// The puzzle, with exactly one solution.
    pub puzzle: Puzzle,
    /// Its solution.
    pub solution: Answer,
    /// Its full rating.
    pub rating: Rating,
    /// The seed it was generated from.
    pub seed: u64,
    /// Candidates screened.
    pub attempts: u32,
}

/// Why no puzzle was generated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenerateError {
    /// The request cannot be met, such as an Extreme puzzle.
    UnsupportedRequest(&'static str),
    /// No puzzle was found within the attempt limit.
    AttemptsExhausted {
        /// Candidates screened.
        attempts: u32,
    },
    /// The work budget ran out.
    BudgetExceeded,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRequest(reason) => f.write_str(reason),
            Self::AttemptsExhausted { attempts } => write!(
                f,
                "no Hitori of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a Hitori was found")
            }
        }
    }
}

impl Error for GenerateError {}

impl From<BudgetExceeded> for GenerateError {
    fn from(_: BudgetExceeded) -> Self {
        Self::BudgetExceeded
    }
}

/// A candidate: the Latin square, the solution's shading and the puzzle's
/// numbers. Unshaded cells hold their Latin number; shaded cells copy a
/// number of an unshaded cell in their row or column.
#[derive(Clone)]
struct Candidate {
    latin: Vec<u32>,
    shaded: Vec<bool>,
    numbers: Vec<u32>,
    /// Changes proposed since the candidate was built.
    edits: u32,
}

struct Builder {
    shape: Shape,
    nb: Vec<Vec<usize>>,
}

impl Builder {
    /// Cells in the same row or column as `i`, other than `i`.
    fn lines(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (rows, cols) = (self.shape.rows(), self.shape.cols());
        let (r, c) = (i / cols, i % cols);
        (0..cols)
            .map(move |k| r * cols + k)
            .chain((0..rows).map(move |k| k * cols + c))
            .filter(move |&j| j != i)
    }

    /// Numbers a shaded cell may copy: those of the unshaded cells in its
    /// row and column, in increasing order without repeats.
    fn options(&self, c: &Candidate, i: usize) -> Vec<u32> {
        let mut out: Vec<u32> = self
            .lines(i)
            .filter(|&j| !c.shaded[j])
            .map(|j| c.latin[j])
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether shading `i` keeps shaded cells apart and the unshaded cells
    /// connected and nonempty.
    fn can_shade(&self, shaded: &[bool], i: usize) -> bool {
        if shaded[i] || self.nb[i].iter().any(|&j| shaded[j]) {
            return false;
        }
        let Some(start) = (0..shaded.len()).find(|&j| j != i && !shaded[j]) else {
            return false;
        };
        let reached = flood(start, |j| j != i && !shaded[j], &self.nb).len();
        reached == shaded.iter().filter(|&&s| !s).count() - 1
    }

    /// Give shaded cell `i` a random number to copy, other than its current
    /// one when there is a choice. Unshades it if there is nothing to copy.
    fn renumber(&self, c: &mut Candidate, i: usize, rng: &mut SplitMix64) {
        let mut options = self.options(c, i);
        if options.len() > 1 {
            options.retain(|&v| v != c.numbers[i]);
        }
        if options.is_empty() {
            c.shaded[i] = false;
            c.numbers[i] = c.latin[i];
        } else {
            c.numbers[i] = options[rng.below(options.len() as u64) as usize];
        }
    }

    /// Shade `i` and give every shaded cell in its lines that no longer
    /// copies an unshaded number a new one.
    fn shade(&self, c: &mut Candidate, i: usize, rng: &mut SplitMix64) {
        c.shaded[i] = true;
        self.renumber(c, i, rng);
        let lines: Vec<usize> = self.lines(i).collect();
        for j in lines {
            if c.shaded[j] && !self.options(c, j).contains(&c.numbers[j]) {
                self.renumber(c, j, rng);
            }
        }
    }
}

impl Construct for Builder {
    type Candidate = Candidate;
    type Puzzle = Puzzle;

    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        let cols = self.shape.cols();
        let n = self.shape.cells();
        let m = self.shape.largest() as usize;
        if !budget.spend((n * n) as u64) {
            return Err(BudgetExceeded);
        }
        // A cyclic Latin square with shuffled rows, columns and symbols.
        let mut row_order: Vec<usize> = (0..m).collect();
        let mut col_order: Vec<usize> = (0..m).collect();
        let mut symbols: Vec<u32> = (1..=m as u32).collect();
        rng.shuffle(&mut row_order);
        rng.shuffle(&mut col_order);
        rng.shuffle(&mut symbols);
        let latin: Vec<u32> = (0..n)
            .map(|i| symbols[(row_order[i / cols] + col_order[i % cols]) % m])
            .collect();
        let mut c = Candidate {
            numbers: latin.clone(),
            latin,
            shaded: vec![false; n],
            edits: 0,
        };
        // Shade a quarter to two fifths of the cells, in a random order.
        let target = n * (25 + rng.below(16) as usize) / 100;
        let mut order: Vec<usize> = (0..n).collect();
        rng.shuffle(&mut order);
        let mut count = 0;
        for &i in &order {
            if count >= target {
                break;
            }
            if self.can_shade(&c.shaded, i) {
                c.shaded[i] = true;
                count += 1;
            }
        }
        for i in 0..n {
            if c.shaded[i] {
                self.renumber(&mut c, i, rng);
            }
        }
        Ok(c)
    }

    fn puzzle(&self, c: &Candidate) -> Puzzle {
        Puzzle::new(self.shape, c.numbers.clone()).expect("generated numbers are in range")
    }

    /// The exact solver first: a puzzle with several solutions is stuck on
    /// the cells where two of them differ. Only unique puzzles are solved
    /// with the techniques.
    fn screen(
        &self,
        p: &Puzzle,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        let cols = p.shape().cols();
        match solve(p, budget)? {
            Solutions::Unique(_) => {}
            Solutions::Multiple(a, b) => {
                return Ok(Outcome::Stuck(
                    (0..p.shape().cells())
                        .filter(|&i| a.get(i / cols, i % cols) != b.get(i / cols, i % cols))
                        .collect(),
                ));
            }
            _ => return Ok(Outcome::Stuck((0..p.shape().cells()).collect())),
        }
        Ok(outcome(&analyze_state(
            p,
            initial_masks(p),
            budget,
            cap,
            false,
        )?))
    }

    /// Change the shading or a copied number near an unresolved cell, or
    /// anywhere when the puzzle solved below the requested level. A
    /// candidate is rebuilt after twice as many changes as it has cells.
    fn change(
        &self,
        c: &mut Candidate,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Candidate> {
        let n = self.shape.cells();
        let focus = match outcome {
            Outcome::Stuck(cells) if !cells.is_empty() => {
                cells[rng.below(cells.len() as u64) as usize]
            }
            _ => rng.below(n as u64) as usize,
        };
        c.edits += 1;
        if c.edits > 2 * n as u32 {
            return Move::Restart;
        }
        let mut next = c.clone();
        let choice = rng.below(3);
        if next.shaded[focus] {
            if choice == 0 {
                // Unshading never breaks the shading's rules.
                next.shaded[focus] = false;
                next.numbers[focus] = next.latin[focus];
            } else {
                self.renumber(&mut next, focus, rng);
            }
            return Move::Try(next);
        }
        // Shade the cell or a neighbour, which settles cells that could be
        // shaded or not; otherwise change a number copied in its lines.
        let target = if choice == 0 {
            Some(focus)
        } else {
            let nb = &self.nb[focus];
            nb.get(rng.below(nb.len().max(1) as u64) as usize).copied()
        };
        if choice < 2
            && let Some(t) = target
            && self.can_shade(&next.shaded, t)
        {
            self.shade(&mut next, t, rng);
            return Move::Try(next);
        }
        let mut shaded: Vec<usize> = self.lines(focus).filter(|&j| next.shaded[j]).collect();
        if shaded.is_empty() {
            shaded = (0..n).filter(|&j| next.shaded[j]).collect();
        }
        if shaded.is_empty() {
            if !self.can_shade(&next.shaded, focus) {
                return Move::Restart;
            }
            self.shade(&mut next, focus, rng);
        } else {
            let j = shaded[rng.below(shaded.len() as u64) as usize];
            self.renumber(&mut next, j, rng);
        }
        Move::Try(next)
    }

    /// Solved candidates climb: a change is kept if the level rises, or
    /// stays and the proof grows no smaller.
    fn keep(&self, before: &Outcome, after: &Outcome) -> bool {
        if let (Outcome::Solved(l1, w1), Outcome::Solved(l2, w2)) = (before, after) {
            return (l2, w2) >= (l1, w1);
        }
        after.rank() >= before.rank()
    }

    fn patience(&self) -> u32 {
        2 * (self.shape.rows() + self.shape.cols()) as u32
    }

    fn certify(&self, p: &Puzzle, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(p, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop. The solution is a shuffled Latin
/// square with a quarter to two fifths of the cells shaded; each shaded cell
/// copies the number of an unshaded cell in its row or column. Copied
/// numbers and shaded cells are changed until the techniques up to the
/// requested level solve the puzzle. The returned puzzle is rated in full
/// and checked for uniqueness. Extreme is not supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    if options.difficulty == Some(Difficulty::Extreme) {
        return Err(GenerateError::UnsupportedRequest(
            "Extreme Hitori generation is unavailable: no Extreme human technique profile is supported",
        ));
    }
    let shape = options.params.shape;
    let builder = Builder {
        shape,
        nb: neighbours(shape.rows(), shape.cols()),
    };
    match search(
        &builder,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => {
            let mut solution = Answer::new(shape);
            for (i, &s) in found.candidate.shaded.iter().enumerate() {
                let mark = if s { Mark::Shaded } else { Mark::Unshaded };
                solution.set(i / shape.cols(), i % shape.cols(), mark);
            }
            Ok(Generated {
                puzzle: builder.puzzle(&found.candidate),
                solution,
                rating: found.rating,
                seed: options.seed,
                attempts: found.attempts,
            })
        }
        Err(SearchError::BudgetExceeded) => Err(GenerateError::BudgetExceeded),
        Err(SearchError::AttemptsExhausted(attempts)) => {
            Err(GenerateError::AttemptsExhausted { attempts })
        }
    }
}
