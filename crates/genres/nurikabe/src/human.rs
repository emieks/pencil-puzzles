//! Nurikabe's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order with two candidates: land
//! (unshaded, value 0) and sea (shaded, value 1). Numbered cells start as
//! land. An "island" is a connected group of land cells; a "numbered island"
//! contains a number.
use crate::Puzzle;
use crate::board::{LAND, SEA};
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};
use std::collections::{BTreeSet, VecDeque};

const BOTH: u16 = LAND | SEA;
/// Longest Hard contradiction chain, using Easy and Medium techniques.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain.
const LONG_CHAIN: usize = 20;

/// The board's shape, numbers and neighbours.
pub(crate) struct Grid {
    cols: usize,
    clues: Vec<u32>,
    neighbours: Vec<Vec<usize>>,
}

impl Grid {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        let (rows, cols) = (puzzle.shape().rows(), puzzle.shape().cols());
        let neighbours = pencil_puzzles_core::grid::neighbours(rows, cols);
        Self {
            cols,
            clues: puzzle.clues().to_vec(),
            neighbours,
        }
    }

    /// Top-left cells of every 2x2 block.
    fn blocks(&self) -> impl Iterator<Item = [usize; 4]> + '_ {
        let (cols, rows) = (self.cols, self.clues.len() / self.cols.max(1));
        (0..rows.saturating_sub(1)).flat_map(move |r| {
            (0..cols.saturating_sub(1)).map(move |c| {
                let i = r * cols + c;
                [i, i + 1, i + cols, i + cols + 1]
            })
        })
    }
}

/// Connected groups of cells holding exactly `value`.
struct Groups {
    /// Group index of each cell, if it holds the value.
    of: Vec<Option<usize>>,
    cells: Vec<Vec<usize>>,
}

fn groups(grid: &Grid, masks: &[u16], value: u16) -> Groups {
    let mut of = vec![None; masks.len()];
    let mut cells = Vec::new();
    for start in 0..masks.len() {
        if masks[start] != value || of[start].is_some() {
            continue;
        }
        let id = cells.len();
        let mut group = vec![start];
        of[start] = Some(id);
        let mut at = 0;
        while at < group.len() {
            for &j in &grid.neighbours[group[at]] {
                if masks[j] == value && of[j].is_none() {
                    of[j] = Some(id);
                    group.push(j);
                }
            }
            at += 1;
        }
        cells.push(group);
    }
    Groups { of, cells }
}

/// The numbered cells of an island.
fn numbers(grid: &Grid, island: &[usize]) -> Vec<usize> {
    island
        .iter()
        .copied()
        .filter(|&i| grid.clues[i] > 0)
        .collect()
}

/// The single number of an island, if it has exactly one.
fn number(grid: &Grid, island: &[usize]) -> Option<u32> {
    match numbers(grid, island)[..] {
        [i] => Some(grid.clues[i]),
        _ => None,
    }
}

/// Unknown cells next to a group, in order.
fn border(grid: &Grid, masks: &[u16], group: &[usize]) -> Vec<usize> {
    let mut out = BTreeSet::new();
    for &i in group {
        out.extend(
            grid.neighbours[i]
                .iter()
                .copied()
                .filter(|&j| masks[j] == BOTH),
        );
    }
    out.into_iter().collect()
}

/// Whether unknown cell `i` touches a numbered island other than `own`.
fn touches_other(grid: &Grid, land: &Groups, own: Option<usize>, i: usize) -> bool {
    grid.neighbours[i].iter().any(|&j| {
        land.of[j].is_some_and(|g| Some(g) != own && !numbers(grid, &land.cells[g]).is_empty())
    })
}

/// Cells a numbered island could still grow into, with the distance from
/// the island: through unknown cells and unnumbered land, never next to
/// another numbered island, at most `reach` cells away.
fn reachable(grid: &Grid, masks: &[u16], land: &Groups, island: usize, reach: usize) -> Vec<usize> {
    let mut dist = vec![usize::MAX; masks.len()];
    let mut queue = VecDeque::new();
    for &i in &land.cells[island] {
        dist[i] = 0;
        queue.push_back(i);
    }
    while let Some(i) = queue.pop_front() {
        if dist[i] == reach {
            continue;
        }
        for &j in &grid.neighbours[i] {
            if dist[j] != usize::MAX || masks[j] & LAND == 0 {
                continue;
            }
            let other_numbered = land.of[j]
                .is_some_and(|g| g != island && !numbers(grid, &land.cells[g]).is_empty());
            if other_numbered || (masks[j] == BOTH && touches_other(grid, land, Some(island), j)) {
                continue;
            }
            dist[j] = dist[i] + 1;
            queue.push_back(j);
        }
    }
    (0..masks.len())
        .filter(|&i| dist[i] != usize::MAX)
        .collect()
}

/// Whether the candidates already break a rule. Never true while some
/// completion could satisfy every rule; exact on full grids.
fn broken(grid: &Grid, masks: &[u16]) -> bool {
    if masks.contains(&0) || masks.iter().all(|&m| m == LAND) {
        return true;
    }
    if grid.blocks().any(|b| b.iter().all(|&i| masks[i] == SEA)) {
        return true;
    }
    let land = groups(grid, masks, LAND);
    for (g, island) in land.cells.iter().enumerate() {
        match numbers(grid, island).len() {
            0 => {
                // Unnumbered land must be able to reach some number.
                let possible: Vec<bool> = masks.iter().map(|&m| m & LAND != 0).collect();
                let region =
                    pencil_puzzles_core::grid::flood(island[0], |j| possible[j], &grid.neighbours);
                if !region.iter().any(|&i| grid.clues[i] > 0) {
                    return true;
                }
            }
            1 => {
                let clue = number(grid, island).expect("one number") as usize;
                if island.len() > clue {
                    return true;
                }
                let room = reachable(grid, masks, &land, g, masks.len());
                if room.len() < clue {
                    return true;
                }
            }
            _ => return true,
        }
    }
    // The sea must be able to connect.
    let possible: Vec<bool> = masks.iter().map(|&m| m & SEA != 0).collect();
    if let Some(first) = masks.iter().position(|&m| m == SEA) {
        let region = pencil_puzzles_core::grid::flood(first, |j| possible[j], &grid.neighbours);
        let mut reached = vec![false; masks.len()];
        for i in region {
            reached[i] = true;
        }
        if (0..masks.len()).any(|i| masks[i] == SEA && !reached[i]) {
            return true;
        }
    }
    false
}

/// Bookkeeping: a numbered island of its full size is walled off by sea.
fn island_complete(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let land = groups(grid, c.masks, LAND);
    let mut steps = Vec::new();
    for island in &land.cells {
        c.charge()?;
        if number(grid, island).is_some_and(|n| n as usize == island.len()) {
            let changes = border(grid, c.masks, island)
                .into_iter()
                .map(|i| (i, SEA))
                .collect();
            steps.extend(c.step("island_complete", island.clone(), Vec::new(), changes));
        }
    }
    Ok(steps)
}

/// Bookkeeping: once every numbered island is complete and no other land
/// is left, every open cell is sea.
fn all_complete(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let land = groups(grid, c.masks, LAND);
    c.charge()?;
    let done = land
        .cells
        .iter()
        .all(|island| number(grid, island).is_some_and(|n| n as usize == island.len()))
        && (0..c.masks.len()).all(|i| grid.clues[i] == 0 || land.of[i].is_some());
    if !done {
        return Ok(Vec::new());
    }
    let changes = (0..c.masks.len())
        .filter(|&i| c.masks[i] == BOTH)
        .map(|i| (i, SEA))
        .collect();
    let support = land.cells.concat();
    Ok(c.step("all_complete", support, Vec::new(), changes)
        .into_iter()
        .collect())
}

/// A cell next to two different numbered islands is sea.
fn separate_clues(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let land = groups(grid, c.masks, LAND);
    let mut steps = Vec::new();
    for i in 0..c.masks.len() {
        c.charge()?;
        if c.masks[i] != BOTH {
            continue;
        }
        let touching: BTreeSet<_> = grid.neighbours[i]
            .iter()
            .filter_map(|&j| land.of[j])
            .filter(|&g| !numbers(grid, &land.cells[g]).is_empty())
            .collect();
        if touching.len() >= 2 {
            let mut support = vec![i];
            for &g in &touching {
                support.extend(&land.cells[g]);
            }
            steps.extend(c.step("separate_clues", support, Vec::new(), vec![(i, SEA)]));
        }
    }
    Ok(steps)
}

/// A 2x2 block with three sea cells has land in the fourth.
fn no_pool(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let mut steps = Vec::new();
    for block in c.context.blocks() {
        c.charge()?;
        let open: Vec<_> = block
            .iter()
            .copied()
            .filter(|&i| c.masks[i] == BOTH)
            .collect();
        if let [i] = open[..]
            && block.iter().filter(|&&j| c.masks[j] == SEA).count() == 3
        {
            steps.extend(c.step("no_pool", block.to_vec(), Vec::new(), vec![(i, LAND)]));
        }
    }
    Ok(steps)
}

/// An island that must still grow, or unnumbered land that must still reach
/// a number, with only one way out extends through it.
fn island_exit(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let land = groups(grid, c.masks, LAND);
    let mut steps = Vec::new();
    for (g, island) in land.cells.iter().enumerate() {
        c.charge()?;
        let growing = match numbers(grid, island).len() {
            0 => true,
            1 => (number(grid, island).expect("one number") as usize) > island.len(),
            _ => false,
        };
        if !growing {
            continue;
        }
        let numbered = !numbers(grid, island).is_empty();
        let exits: Vec<_> = border(grid, c.masks, island)
            .into_iter()
            .filter(|&i| !numbered || !touches_other(grid, &land, Some(g), i))
            .collect();
        if let [i] = exits[..] {
            steps.extend(c.step("island_exit", island.clone(), Vec::new(), vec![(i, LAND)]));
        }
    }
    Ok(steps)
}

/// A sea area that must connect to other sea, with only one way out,
/// extends through it.
fn sea_exit(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let sea = groups(grid, c.masks, SEA);
    let mut steps = Vec::new();
    if sea.cells.len() < 2 {
        return Ok(steps);
    }
    for area in &sea.cells {
        c.charge()?;
        if let [i] = border(grid, c.masks, area)[..] {
            steps.extend(c.step("sea_exit", area.clone(), Vec::new(), vec![(i, SEA)]));
        }
    }
    Ok(steps)
}

/// A cell no numbered island can reach is sea.
fn unreachable(c: &Candidates<'_, Grid>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let grid = c.context;
    let land = groups(grid, c.masks, LAND);
    let mut reached = vec![false; c.masks.len()];
    let mut support = Vec::new();
    for (g, island) in land.cells.iter().enumerate() {
        c.charge()?;
        let Some(clue) = number(grid, island) else {
            continue;
        };
        let need = (clue as usize).saturating_sub(island.len());
        support.extend(island);
        for i in reachable(grid, c.masks, &land, g, need) {
            reached[i] = true;
        }
    }
    let mut steps = Vec::new();
    for (i, &seen) in reached.iter().enumerate() {
        if c.masks[i] == BOTH && !seen {
            steps.extend(c.step("unreachable", support.clone(), Vec::new(), vec![(i, SEA)]));
        }
    }
    Ok(steps)
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
    version: 1,
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
        let grid = Grid::new(&Puzzle::new(shape, vec![0; cells]).unwrap());
        for i in 0..cells {
            if land[i] && !seen[i] {
                let island = pencil_puzzles_core::grid::flood(i, |j| land[j], &grid.neighbours);
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
