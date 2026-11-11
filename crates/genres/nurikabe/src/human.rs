//! Nurikabe's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order with two candidates: land
//! (unshaded, value 0) and sea (shaded, value 1). Numbered cells start as
//! land. An "island" is a connected group of land cells; a "numbered island"
//! contains a number. Cell sets are bitboards (`bits`).
use crate::Puzzle;
use crate::bits::{self, Bits, bit, cells, count, first};
use crate::board::{LAND, SEA, State};
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};

const BOTH: u16 = LAND | SEA;
/// Longest Hard contradiction chain, using Easy and Medium techniques.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain.
const LONG_CHAIN: usize = 20;

/// The board's shape and numbers.
pub(crate) struct Grid {
    g: bits::Grid,
    clues: Vec<u32>,
    /// Numbered cells.
    numbered: Bits,
}

impl Grid {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        let mut numbered = 0;
        for (i, &n) in puzzle.clues().iter().enumerate() {
            if n > 0 {
                numbered |= bit(i);
            }
        }
        Self {
            g: bits::Grid::new(puzzle.shape()),
            clues: puzzle.clues().to_vec(),
            numbered,
        }
    }

    /// The single number of an island, if it has exactly one.
    fn number(&self, island: Bits) -> Option<u32> {
        let numbers = island & self.numbered;
        (count(numbers) == 1).then(|| self.clues[first(numbers)])
    }
}

/// The candidates as cell sets: fixed land, fixed sea and open cells.
#[derive(Clone, Copy)]
struct Cells {
    land: Bits,
    sea: Bits,
    open: Bits,
}

impl Cells {
    fn new(grid: &Grid, masks: &[u16]) -> Self {
        let s = State::from_masks(masks);
        Self {
            land: s.land & !s.sea,
            sea: s.sea & !s.land,
            open: s.open(grid.g.all),
        }
    }
}

/// Connected groups of a set of cells, in order of their lowest cell.
fn groups(g: &bits::Grid, mut set: Bits) -> Vec<Bits> {
    let mut out = Vec::new();
    while set != 0 {
        let group = g.flood(bit(first(set)), set);
        set &= !group;
        out.push(group);
    }
    out
}

/// The cells of every numbered island.
fn numbered_land(grid: &Grid, islands: &[Bits]) -> Bits {
    islands
        .iter()
        .filter(|&&island| island & grid.numbered != 0)
        .fold(0, |acc, &island| acc | island)
}

/// Cells a numbered island could still grow into, with the island itself:
/// through open cells and unnumbered land, never into or next to another
/// numbered island, at most `reach` cells away.
fn reachable(grid: &Grid, c: Cells, numbered: Bits, island: Bits, reach: usize) -> Bits {
    let others = numbered & !island;
    let allowed = (c.land | c.open) & !others & !(c.open & grid.g.adjacent(others));
    grid.g.flood_steps(island, allowed, reach)
}

/// Whether the candidates already break a rule. Never true while some
/// completion could satisfy every rule; exact on full grids.
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    let g = &grid.g;
    let s = State::from_masks(masks);
    if s.dead() || s.land == g.all {
        return true;
    }
    let c = Cells::new(grid, masks);
    if g.pools(c.sea) != 0 {
        return true;
    }
    let islands = groups(g, c.land);
    let numbered = numbered_land(grid, &islands);
    let possible = c.land | c.open;
    for &island in &islands {
        match count(island & grid.numbered) {
            0 => {
                // Unnumbered land must be able to reach some number.
                if g.flood(island, possible) & grid.numbered == 0 {
                    return true;
                }
            }
            1 => {
                let clue = grid.number(island).expect("one number") as usize;
                if count(island) > clue
                    || count(reachable(grid, c, numbered, island, usize::MAX)) < clue
                {
                    return true;
                }
            }
            _ => return true,
        }
    }
    // The sea must be able to connect.
    c.sea != 0 && g.flood(bit(first(c.sea)), c.sea | c.open) & c.sea != c.sea
}

/// The cells of a set, in order, as changes to one value.
fn changes(set: Bits, keep: u16) -> Vec<(usize, u16)> {
    cells(set).map(|i| (i, keep)).collect()
}

/// Bookkeeping: a numbered island of its full size is walled off by sea.
fn island_complete(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let cs = Cells::new(grid, c.masks);
    let mut steps = Vec::new();
    for island in groups(&grid.g, cs.land) {
        c.charge()?;
        if grid
            .number(island)
            .is_some_and(|n| n as usize == count(island))
        {
            let border = grid.g.border(island) & cs.open;
            steps.extend(c.step(
                "island_complete",
                cells(island).collect(),
                Vec::new(),
                changes(border, SEA),
            ));
        }
    }
    Ok(steps)
}

/// Bookkeeping: once every numbered island is complete and no other land
/// is left, every open cell is sea.
fn all_complete(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let cs = Cells::new(grid, c.masks);
    c.charge()?;
    let islands = groups(&grid.g, cs.land);
    let done = islands.iter().all(|&island| {
        grid.number(island)
            .is_some_and(|n| n as usize == count(island))
    }) && grid.numbered & !cs.land == 0;
    if !done {
        return Ok(Vec::new());
    }
    Ok(c.step(
        "all_complete",
        cells(cs.land).collect(),
        Vec::new(),
        changes(cs.open, SEA),
    )
    .into_iter()
    .collect())
}

/// A cell next to two different numbered islands is sea.
fn separate_clues(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let cs = Cells::new(grid, c.masks);
    let islands: Vec<Bits> = groups(&grid.g, cs.land)
        .into_iter()
        .filter(|&island| island & grid.numbered != 0)
        .collect();
    let mut steps = Vec::new();
    for i in 0..c.masks.len() {
        c.charge()?;
        if cs.open & bit(i) == 0 {
            continue;
        }
        let around = grid.g.adjacent(bit(i));
        let touching: Vec<Bits> = islands
            .iter()
            .copied()
            .filter(|&island| island & around != 0)
            .collect();
        if touching.len() >= 2 {
            let mut support = vec![i];
            for island in touching {
                support.extend(cells(island));
            }
            steps.extend(c.step("separate_clues", support, Vec::new(), vec![(i, SEA)]));
        }
    }
    Ok(steps)
}

/// A 2x2 block with three sea cells has land in the fourth.
fn no_pool(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let cs = Cells::new(grid, c.masks);
    let mut steps = Vec::new();
    for corner in cells(grid.g.pools(grid.g.all)) {
        c.charge()?;
        let block = grid.g.block(corner);
        let open = block & cs.open;
        if count(open) == 1 && count(block & cs.sea) == 3 {
            steps.extend(c.step(
                "no_pool",
                cells(block).collect(),
                Vec::new(),
                vec![(first(open), LAND)],
            ));
        }
    }
    Ok(steps)
}

/// An island that must still grow, or unnumbered land that must still reach
/// a number, with only one way out extends through it.
fn island_exit(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let cs = Cells::new(grid, c.masks);
    let islands = groups(&grid.g, cs.land);
    let numbered = numbered_land(grid, &islands);
    let mut steps = Vec::new();
    for &island in &islands {
        c.charge()?;
        let growing = match count(island & grid.numbered) {
            0 => true,
            1 => (grid.number(island).expect("one number") as usize) > count(island),
            _ => false,
        };
        if !growing {
            continue;
        }
        let mut exits = grid.g.border(island) & cs.open;
        if island & grid.numbered != 0 {
            exits &= !grid.g.adjacent(numbered & !island);
        }
        if count(exits) == 1 {
            steps.extend(c.step(
                "island_exit",
                cells(island).collect(),
                Vec::new(),
                vec![(first(exits), LAND)],
            ));
        }
    }
    Ok(steps)
}

/// A sea area that must connect to other sea, with only one way out,
/// extends through it.
fn sea_exit(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let cs = Cells::new(grid, c.masks);
    let areas = groups(&grid.g, cs.sea);
    let mut steps = Vec::new();
    if areas.len() < 2 {
        return Ok(steps);
    }
    for area in areas {
        c.charge()?;
        let exits = grid.g.border(area) & cs.open;
        if count(exits) == 1 {
            steps.extend(c.step(
                "sea_exit",
                cells(area).collect(),
                Vec::new(),
                vec![(first(exits), SEA)],
            ));
        }
    }
    Ok(steps)
}

/// A cell no numbered island can reach is sea.
fn unreachable(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let cs = Cells::new(grid, c.masks);
    let islands = groups(&grid.g, cs.land);
    let numbered = numbered_land(grid, &islands);
    let mut reached = 0;
    let mut support = Vec::new();
    for &island in &islands {
        c.charge()?;
        let Some(clue) = grid.number(island) else {
            continue;
        };
        let need = (clue as usize).saturating_sub(count(island));
        support.extend(cells(island));
        reached |= reachable(grid, cs, numbered, island, need);
    }
    Ok(cells(cs.open & !reached)
        .filter_map(|i| c.step("unreachable", support.clone(), Vec::new(), vec![(i, SEA)]))
        .collect())
}

const fn entry(
    id: &'static str,
    level: Difficulty,
    kind: Kind,
    probes: bool,
    find: Find<Grid>,
) -> Technique<Grid> {
    Technique {
        id,
        level,
        kind,
        probes,
        find,
    }
}

use Difficulty::{Easy, Expert, Hard, Medium};
use Kind::{Bookkeeping, Skill};

static TECHNIQUES: [Technique<Grid>; 10] = [
    entry("island_complete", Easy, Bookkeeping, false, island_complete),
    entry("all_complete", Easy, Bookkeeping, false, all_complete),
    entry("separate_clues", Easy, Skill, false, separate_clues),
    entry("no_pool", Easy, Skill, false, no_pool),
    entry("island_exit", Easy, Skill, false, island_exit),
    entry("sea_exit", Easy, Skill, false, sea_exit),
    entry("unreachable", Medium, Skill, false, unreachable),
    // A cell where one value breaks a rule at once: merging islands, cutting
    // the sea in two, leaving an island too little room. No chain is needed.
    entry("forced", Medium, Skill, false, |c| {
        PROFILE.contradictions(c, "forced", Easy, 0)
    }),
    entry("short_contradiction", Hard, Skill, true, |c| {
        PROFILE.contradictions(c, "short_contradiction", Medium, SHORT_CHAIN)
    }),
    entry("contradiction", Expert, Skill, true, |c| {
        PROFILE.contradictions(c, "contradiction", Hard, LONG_CHAIN)
    }),
];

/// The canonical Nurikabe human profile.
pub(crate) static PROFILE: Profile<Grid> = Profile {
    id: "nurikabe-human",
    techniques: &TECHNIQUES,
    broken,
};

/// Numbered cells are land; every other cell is open.
pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    puzzle
        .clues()
        .iter()
        .map(|&n| if n > 0 { LAND } else { BOTH })
        .collect()
}

/// Solve from `masks` with techniques up to `cap`.
pub(crate) fn analyze_state(
    puzzle: &Puzzle,
    masks: Vec<u16>,
    budget: &Budget,
    cap: Difficulty,
    metrics: bool,
) -> Result<Trace, BudgetExceeded> {
    let placed = vec![false; masks.len()];
    PROFILE.analyze(&Grid::new(puzzle), masks, placed, budget, cap, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Answer, Mark, Shape};
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    fn answer(shape: Shape, sea: &[bool]) -> Answer {
        let mut answer = Answer::new(shape);
        for (i, &s) in sea.iter().enumerate() {
            let mark = if s { Mark::Shaded } else { Mark::Unshaded };
            answer.set(i / shape.cols(), i % shape.cols(), mark);
        }
        answer
    }

    /// Every valid shading of a small grid, by the independent answer
    /// checker; also checks that `broken` is exact on every full grid that
    /// leaves the numbers unshaded.
    fn valid_seas(grid: &Grid, puzzle: &Puzzle) -> Vec<Vec<bool>> {
        let cells = puzzle.clues().len();
        (0..1u32 << cells)
            .map(|bits| (0..cells).map(|i| bits >> i & 1 == 1).collect::<Vec<_>>())
            .filter(|sea| (0..cells).all(|i| puzzle.clues()[i] == 0 || !sea[i]))
            .filter(|sea| {
                let valid = puzzle.is_solved_by(&answer(puzzle.shape(), sea));
                let full: Vec<u16> = sea.iter().map(|&s| if s { SEA } else { LAND }).collect();
                assert_eq!(broken(grid, &full), !valid, "broken disagrees on {sea:?}");
                valid
            })
            .collect()
    }

    /// A random valid Nurikabe solution's puzzle on a small grid, by
    /// shading random cells until the result happens to be valid.
    fn random_puzzle(rng: &mut SplitMix64, rows: usize, cols: usize) -> Option<Puzzle> {
        let cells = rows * cols;
        let shape = Shape::new(rows, cols).unwrap();
        let sea: Vec<bool> = (0..cells).map(|_| rng.below(5) < 3).collect();
        let mut clues = vec![0; cells];
        let land: Vec<bool> = sea.iter().map(|s| !s).collect();
        let mut seen = vec![false; cells];
        let neighbours = pencil_puzzles_core::grid::neighbours(rows, cols);
        for i in 0..cells {
            if land[i] && !seen[i] {
                let island = pencil_puzzles_core::grid::flood(i, |j| land[j], &neighbours);
                for &j in &island {
                    seen[j] = true;
                }
                clues[island[rng.below(island.len() as u64) as usize]] = island.len() as u32;
            }
        }
        let puzzle = Puzzle::new(shape, clues).ok()?;
        puzzle.is_solved_by(&answer(shape, &sea)).then_some(puzzle)
    }

    /// Every technique keeps every valid solution of random partial states,
    /// and `broken` never fires while a solution remains.
    #[test]
    fn techniques_keep_every_solution() {
        let mut rng = SplitMix64::new(11);
        let budget = Budget::unlimited();
        let mut fired = BTreeSet::new();
        let mut checked = 0;
        while checked < 300 {
            let (rows, cols) = (2 + rng.below(2) as usize, 2 + rng.below(3) as usize);
            let Some(puzzle) = random_puzzle(&mut rng, rows, cols) else {
                continue;
            };
            checked += 1;
            let grid = Grid::new(&puzzle);
            let solutions = valid_seas(&grid, &puzzle);
            let truth = &solutions[rng.below(solutions.len() as u64) as usize];
            let masks: Vec<u16> = initial_masks(&puzzle)
                .into_iter()
                .zip(truth)
                .map(|(m, &s)| match (m, rng.below(3)) {
                    (BOTH, 0) => {
                        if s {
                            SEA
                        } else {
                            LAND
                        }
                    }
                    (m, _) => m,
                })
                .collect();
            let fits: Vec<_> = solutions
                .iter()
                .filter(|sea| {
                    (0..masks.len()).all(|i| masks[i] & if sea[i] { SEA } else { LAND } != 0)
                })
                .collect();
            assert!(!broken(&grid, &masks), "broken although a solution fits");
            let placed = vec![false; masks.len()];
            let candidates = Candidates::new(&grid, &masks, &placed, &budget);
            for technique in &TECHNIQUES {
                for step in technique.run(&candidates).unwrap() {
                    fired.insert(step.technique);
                    for &(i, keep) in &step.changes {
                        for sea in &fits {
                            let value = if sea[i] { SEA } else { LAND };
                            assert_ne!(keep & value, 0, "{} removed a true value", step.technique);
                        }
                    }
                }
            }
        }
        for technique in [
            "island_complete",
            "separate_clues",
            "no_pool",
            "island_exit",
            "sea_exit",
            "unreachable",
            "forced",
            "short_contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }
}
