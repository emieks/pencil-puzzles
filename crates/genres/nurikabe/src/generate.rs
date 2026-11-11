//! Reproducible generation: islands grown from seeds inside a connected sea,
//! a uniqueness check before any contradiction chain, and clue moves guided
//! by the second solution or by the cells the techniques could not decide.
use crate::bits::{self, Bits, bit, cells, count, first};
use crate::board::{Board, State};
use crate::human::{analyze_state, initial_masks};
use crate::*;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_core::search::{Construct, Move, Outcome, SearchError, search};
use std::cell::RefCell;
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
/// A generated puzzle and its proof witness.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Generated {
    /// Unique puzzle.
    pub puzzle: Puzzle,
    /// Its complete solution.
    pub solution: Answer,
    /// Exact canonical rating.
    pub rating: Rating,
    /// Reproduction seed.
    pub seed: u64,
    /// Candidates tried.
    pub attempts: u32,
}
/// Bounded generation failure.
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
    /// The deterministic work limit ran out.
    BudgetExceeded,
}
impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRequest(reason) => f.write_str(reason),
            Self::AttemptsExhausted { attempts } => write!(
                f,
                "no Nurikabe of the requested level found in {attempts} attempts; try another size or level"
            ),
            Self::BudgetExceeded => {
                f.write_str("the work budget ran out before a Nurikabe was found")
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

/// A candidate: the solution's sea, the numbered cell of each island, the
/// best outcome rank so far and the changes since it.
#[derive(Clone)]
struct Candidate {
    sea: Bits,
    clues: Bits,
    best: (bool, i64),
    stale: u32,
}

/// A candidate's puzzle with its solution's sea. The solution only tells a
/// second solution apart; it never decides a step.
struct Screened {
    puzzle: Puzzle,
    sea: Bits,
}

struct Grower {
    shape: Shape,
    g: bits::Grid,
    /// The requested level, if any.
    level: Option<Difficulty>,
    /// The last screened puzzle that had a second solution, with that
    /// solution's sea, so the next change can rule it out.
    alternative: RefCell<Option<(Puzzle, Bits)>>,
}

/// Tries to find a valid change before starting over.
const CHANGE_TRIES: usize = 8;
/// How many times [`Construct::patience`] a candidate is changed without
/// improving before starting over.
const STALE: u32 = 3;

impl Grower {
    /// Whether a sea is nonempty and connected. A sea covering more than
    /// one cell must leave some land, so that there is an island to number.
    fn connected(&self, sea: Bits) -> bool {
        sea != 0
            && (sea != self.g.all || count(sea) == 1)
            && self.g.flood(bit(first(sea)), sea) == sea
    }

    /// The land cells connected to `i`.
    fn island(&self, c: &Candidate, i: usize) -> Bits {
        self.g.flood(bit(i), self.g.all & !c.sea)
    }

    /// Each island of a shading, in order of its lowest cell.
    fn islands(&self, sea: Bits) -> Vec<Bits> {
        let mut land = self.g.all & !sea;
        let mut out = Vec::new();
        while land != 0 {
            let island = self.g.flood(bit(first(land)), land);
            land &= !island;
            out.push(island);
        }
        out
    }

    /// A random cell of a nonempty set.
    fn pick(&self, set: Bits, rng: &mut SplitMix64) -> usize {
        let n = rng.below(count(set) as u64) as usize;
        cells(set).nth(n).expect("nonempty set")
    }

    /// The size an island aims for, never more than a third of the grid:
    /// 1 to 3 for Easy, mostly 2 to 4 otherwise, sometimes up to 6.
    fn target_size(&self, rng: &mut SplitMix64) -> usize {
        let roll = rng.below(100);
        let size = if self.level == Some(Difficulty::Easy) {
            match roll {
                0..30 => 1,
                30..70 => 2,
                _ => 3,
            }
        } else {
            match roll {
                0..10 => 1,
                10..35 => 2,
                35..65 => 3,
                65..85 => 4,
                85..95 => 5,
                _ => 6,
            }
        };
        size.clamp(1, (count(self.g.all) / 3).max(1))
    }

    /// Grow islands from seeds until the land target is reached, then give
    /// every all-sea 2x2 block a land cell. `None` if the growth got stuck.
    fn grow(&self, rng: &mut SplitMix64) -> Option<Bits> {
        let g = &self.g;
        let cells_total = count(g.all);
        if cells_total == 1 {
            return Some(g.all);
        }
        let land_target = (cells_total * (40 + rng.below(16) as usize) / 100).max(1);
        let mut land: Bits = 0;
        // Growing islands with their remaining growth.
        let mut islands: Vec<(Bits, usize)> = Vec::new();
        loop {
            if count(land) >= land_target {
                break;
            }
            // Grow each island one cell, in turn.
            let mut grew = false;
            for (island, left) in islands.iter_mut() {
                if *left == 0 || count(land) >= land_target {
                    continue;
                }
                let others = land & !*island;
                let frontier = g.border(*island) & !land & !g.adjacent(others);
                let valid: Bits = cells(frontier)
                    .filter(|&x| self.connected(g.all & !land & !bit(x)))
                    .fold(0, |acc, x| acc | bit(x));
                if valid == 0 {
                    *left = 0;
                    continue;
                }
                let x = bit(self.pick(valid, rng));
                land |= x;
                *island |= x;
                *left -= 1;
                grew = true;
            }
            if grew {
                continue;
            }
            // Every island is done: plant a new seed away from the land.
            let seeds: Bits = cells(g.all & !land & !g.adjacent(land))
                .filter(|&x| self.connected(g.all & !land & !bit(x)))
                .fold(0, |acc, x| acc | bit(x));
            if seeds == 0 {
                break;
            }
            let x = bit(self.pick(seeds, rng));
            land |= x;
            islands.push((x, self.target_size(rng) - 1));
        }
        if land == 0 {
            return None;
        }
        // Pools: give one cell of each all-sea block to an island next to
        // it, or make it an island of its own.
        loop {
            let pools = g.pools(g.all & !land);
            if pools == 0 {
                break;
            }
            let block = g.block(first(pools));
            let options: Bits = cells(block)
                .filter(|&x| {
                    let next = g.all & !land & !bit(x);
                    // Joining two islands is left to the moves.
                    count_islands_around(g, land, x) <= 1 && self.connected(next)
                })
                .fold(0, |acc, x| acc | bit(x));
            if options == 0 {
                return None;
            }
            land |= bit(self.pick(options, rng));
        }
        Some(g.all & !land)
    }

    /// The screened puzzle of a candidate.
    fn screened(&self, c: &Candidate) -> Screened {
        let mut clues = vec![0; self.shape.cells()];
        for i in cells(c.clues) {
            clues[i] = count(self.island(c, i)) as u32;
        }
        Screened {
            puzzle: Puzzle::new(self.shape, clues).expect("island sizes are in range"),
            sea: c.sea,
        }
    }

    /// Move the clue of the island holding `i` onto `i`.
    fn move_clue(&self, c: &mut Candidate, i: usize) {
        let island = self.island(c, i);
        c.clues &= !island;
        c.clues |= bit(i);
    }

    /// Make sea cell `x` land, if the sea stays connected and `x` touches at
    /// most `max_islands` islands. A new island gets its clue on `x`; joined
    /// islands keep the clue of the lowest-numbered cell.
    fn add_land(&self, c: &mut Candidate, x: usize, max_islands: usize) -> bool {
        let around = count_islands_around(&self.g, self.g.all & !c.sea, x);
        if around > max_islands || !self.connected(c.sea & !bit(x)) {
            return false;
        }
        c.sea &= !bit(x);
        let island = self.island(c, x);
        let clues = c.clues & island;
        c.clues &= !island;
        c.clues |= if clues == 0 {
            bit(x)
        } else {
            bit(first(clues))
        };
        true
    }

    /// A change that rules out the second solution `alt`: the clue of an
    /// island moves to a cell that is land in the solution and sea in `alt`.
    fn rule_out(&self, c: &Candidate, alt: Bits, rng: &mut SplitMix64) -> Option<Candidate> {
        let land_here = alt & !c.sea;
        if land_here == 0 {
            return None;
        }
        let mut next = c.clone();
        self.move_clue(&mut next, self.pick(land_here, rng));
        Some(next)
    }

    /// A change that adds information where the techniques got stuck:
    /// land added to the sea there, a new island there, or a clue moved
    /// there.
    fn easier(&self, c: &Candidate, open: Bits, rng: &mut SplitMix64) -> Option<Candidate> {
        for _ in 0..CHANGE_TRIES {
            let u = self.pick(open, rng);
            let mut next = c.clone();
            if c.sea & bit(u) != 0 {
                if self.add_land(&mut next, u, 1) {
                    return Some(next);
                }
            } else if c.clues & bit(u) == 0 && rng.below(2) == 0 {
                self.move_clue(&mut next, u);
                return Some(next);
            }
        }
        None
    }

    /// A change that takes information away or enlarges islands: a clue
    /// moved within its island, two islands joined, or an island grown.
    fn harder(&self, c: &Candidate, rng: &mut SplitMix64) -> Option<Candidate> {
        let islands = self.islands(c.sea);
        if islands.is_empty() {
            return None;
        }
        for _ in 0..CHANGE_TRIES {
            let island = islands[rng.below(islands.len() as u64) as usize];
            let mut next = c.clone();
            match rng.below(3) {
                0 => {
                    let elsewhere = island & !c.clues;
                    if elsewhere != 0 {
                        self.move_clue(&mut next, self.pick(elsewhere, rng));
                        return Some(next);
                    }
                }
                1 => {
                    // Join with a neighbouring island through one sea cell.
                    let others = self.g.all & !c.sea & !island;
                    let bridges = self.g.border(island) & c.sea & self.g.adjacent(others);
                    if bridges != 0 && self.add_land(&mut next, self.pick(bridges, rng), 2) {
                        return Some(next);
                    }
                }
                _ => {
                    let others = self.g.all & !c.sea & !island;
                    let frontier = self.g.border(island) & c.sea & !self.g.adjacent(others);
                    if frontier != 0 && self.add_land(&mut next, self.pick(frontier, rng), 1) {
                        return Some(next);
                    }
                }
            }
        }
        None
    }
}

/// How many islands of `land` touch cell `x`.
fn count_islands_around(g: &bits::Grid, land: Bits, x: usize) -> usize {
    let mut touched = g.adjacent(bit(x)) & land;
    let mut n = 0;
    while touched != 0 {
        touched &= !g.flood(bit(first(touched)), land);
        n += 1;
    }
    n
}

impl Construct for Grower {
    type Candidate = Candidate;
    type Puzzle = Screened;

    /// Grow islands inside a connected sea and number each on a random cell.
    fn build(&self, rng: &mut SplitMix64, budget: &Budget) -> Result<Candidate, BudgetExceeded> {
        loop {
            // Each attempt floods the grid once per cell added.
            if !budget.spend(count(self.g.all) as u64) {
                return Err(BudgetExceeded);
            }
            if let Some(sea) = self.grow(rng) {
                let mut clues = 0;
                for island in self.islands(sea) {
                    clues |= bit(self.pick(island, rng));
                }
                return Ok(Candidate {
                    sea,
                    clues,
                    best: (false, i64::MIN),
                    stale: 0,
                });
            }
        }
    }

    fn puzzle(&self, c: &Candidate) -> Screened {
        self.screened(c)
    }

    /// Easy and medium techniques first; then, if they did not finish, a
    /// search for a second solution, which makes the puzzle stuck at once;
    /// then the remaining techniques up to the cap. The result is the same
    /// as one capped solve from the start.
    fn screen(
        &self,
        p: &Screened,
        cap: Difficulty,
        budget: &Budget,
    ) -> Result<Outcome, BudgetExceeded> {
        let low = cap.min(Difficulty::Medium);
        let first_part = analyze_state(&p.puzzle, initial_masks(&p.puzzle), budget, low, false)?;
        if first_part.solved || cap <= Difficulty::Medium {
            return Ok(first_part.outcome());
        }
        let mut board = Board::new(&p.puzzle, budget);
        board.state = State::from_masks(&first_part.masks);
        if let Some(alt) = board.second_solution(p.sea)? {
            *self.alternative.borrow_mut() = Some((p.puzzle.clone(), alt));
            return Ok(first_part.outcome());
        }
        let first_outcome = first_part.outcome();
        let second_part = analyze_state(&p.puzzle, first_part.masks, budget, cap, false)?;
        Ok(match (first_outcome, second_part.outcome()) {
            (Outcome::Solved(l1, s1), Outcome::Solved(l2, s2)) => {
                Outcome::Solved(l1.max(l2), s1 + s2)
            }
            (_, stuck) => stuck,
        })
    }

    fn change(
        &self,
        c: &mut Candidate,
        outcome: &Outcome,
        _: Difficulty,
        rng: &mut SplitMix64,
    ) -> Move<Candidate> {
        if outcome.rank() > c.best {
            c.best = outcome.rank();
            c.stale = 0;
        } else {
            c.stale += 1;
            if c.stale > STALE * self.patience() {
                return Move::Restart;
            }
        }
        let next = match outcome {
            Outcome::Stuck(open) => {
                let puzzle = self.screened(c).puzzle;
                let alternative = self
                    .alternative
                    .borrow()
                    .as_ref()
                    .filter(|(p, _)| *p == puzzle)
                    .map(|&(_, alt)| alt);
                match alternative {
                    Some(alt) => self.rule_out(c, alt, rng),
                    None => {
                        let open = open.iter().fold(0, |acc, &i| acc | bit(i));
                        if open == 0 {
                            None
                        } else {
                            self.easier(c, open, rng)
                        }
                    }
                }
            }
            Outcome::Solved(..) => self.harder(c, rng),
        };
        next.map_or(Move::Restart, Move::Try)
    }

    fn patience(&self) -> u32 {
        // About four rejected changes per line before starting over.
        4 * (self.shape.rows() + self.shape.cols()) as u32
    }

    fn certify(&self, p: &Screened, budget: &Budget) -> Result<Option<Rating>, BudgetExceeded> {
        match crate::rate(&p.puzzle, budget) {
            Ok(rating) => Ok(Some(rating)),
            Err(RateError::BudgetExceeded) => Err(BudgetExceeded),
            Err(_) => Ok(None),
        }
    }
}

/// Generate with the shared search loop: grow islands from seeds inside a
/// connected sea, number each island on a random cell, and screen the puzzle
/// with the human techniques up to the requested level, checking that it has
/// one solution before any contradiction chain is tried. A second solution
/// is ruled out by moving a clue onto a cell where the two differ; cells the
/// techniques could not decide get more information (land added, a new
/// island, a clue moved there); a puzzle below the requested level gets a
/// clue moved within its island, two islands joined or an island grown.
/// The returned puzzle is rated in full and checked for uniqueness. Extreme
/// is not supported.
pub fn generate(options: &GenerateOptions, budget: &Budget) -> Result<Generated, GenerateError> {
    if options.difficulty == Some(Difficulty::Extreme) {
        return Err(GenerateError::UnsupportedRequest(
            "Extreme Nurikabe generation is unavailable: no Extreme human technique profile is supported",
        ));
    }
    let shape = options.params.shape;
    let grower = Grower {
        shape,
        g: bits::Grid::new(shape),
        level: options.difficulty,
        alternative: RefCell::new(None),
    };
    match search(
        &grower,
        options.difficulty,
        options.seed,
        options.max_attempts,
        budget,
    ) {
        Ok(found) => {
            let mut solution = Answer::new(shape);
            for i in 0..shape.cells() {
                solution.set(
                    i / shape.cols(),
                    i % shape.cols(),
                    if found.candidate.sea & bit(i) != 0 {
                        Mark::Shaded
                    } else {
                        Mark::Unshaded
                    },
                );
            }
            Ok(Generated {
                puzzle: grower.screened(&found.candidate).puzzle,
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
