//! Kakurasu's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are cells in row-major order with two candidates, shaded and
//! unshaded. Rule instances ("units") are the rows, top to bottom, then the
//! columns, left to right. Within a line, cells are worth 1, 2, 3, ...
use crate::Puzzle;
use crate::line::{Cell, solve_full};
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};

/// Candidate bit for a shaded cell.
pub(crate) const SHADED: u16 = 0b01;
/// Candidate bit for an unshaded cell.
pub(crate) const UNSHADED: u16 = 0b10;
const BOTH: u16 = SHADED | UNSHADED;

/// A line with at most this many ways to reach its total is a short list,
/// which a solver can write out (Medium). Longer lists are Hard.
const SHORT_LIST: u32 = 4;
/// Longest chain, in steps, of a Hard "what if" contradiction, which uses
/// Easy and Medium techniques only.
const SHORT_CHAIN: usize = 6;
/// Longest chain of an Expert contradiction, which may use Hard line techniques.
const LONG_CHAIN: usize = 20;

/// Rows, then columns: their cells in weight order and their clues.
pub(crate) struct Lines {
    cells: Vec<Vec<usize>>,
    clues: Vec<u32>,
}

impl Lines {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        let (rows, cols) = (puzzle.rows(), puzzle.cols());
        let mut cells: Vec<Vec<usize>> = (0..rows)
            .map(|r| (0..cols).map(|c| r * cols + c).collect())
            .collect();
        cells.extend((0..cols).map(|c| (0..rows).map(|r| r * cols + c).collect()));
        let mut clues = puzzle.row_clues().to_vec();
        clues.extend_from_slice(puzzle.col_clues());
        Self { cells, clues }
    }
}

/// What is known about one line.
struct Status {
    /// Unknown cells and their weights.
    unknown: Vec<(usize, u32)>,
    /// Total still needed, or `None` if the shaded cells already exceed it.
    need: Option<u32>,
    /// Sum of the unknown weights.
    open: u32,
}

fn status(lines: &Lines, masks: &[u16], line: usize) -> Status {
    let mut shaded = 0;
    let mut unknown = Vec::new();
    for (&cell, weight) in lines.cells[line].iter().zip(1..) {
        match masks[cell] {
            SHADED => shaded += weight,
            BOTH => unknown.push((cell, weight)),
            _ => {}
        }
    }
    Status {
        open: unknown.iter().map(|&(_, w)| w).sum(),
        unknown,
        need: lines.clues[line].checked_sub(shaded),
    }
}

/// Whether a cell has no candidate, or some line can no longer reach its
/// clue: its shaded cells exceed it, or no set of its open cells makes up
/// the rest.
fn broken(lines: &Lines, masks: &[u16]) -> bool {
    masks.contains(&0)
        || (0..lines.cells.len()).any(|line| {
            let s = status(lines, masks, line);
            s.need.is_none_or(|need| {
                let weights: Vec<_> = s.unknown.iter().map(|&(_, w)| w).collect();
                s.open < need || count_subsets(&weights, need, 1) == 0
            })
        })
}

/// Each line whose status allows a deduction, as a step on that line.
fn per_line(
    c: &Candidates<'_, Lines>,
    technique: &'static str,
    mut deduce: impl FnMut(usize, &Status, u32) -> Vec<(usize, u16)>,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let lines = c.context;
    let mut steps = Vec::new();
    for line in 0..lines.cells.len() {
        c.charge()?;
        let s = status(lines, c.masks, line);
        let Some(need) = s.need else { continue };
        if s.unknown.is_empty() || s.open < need {
            continue;
        }
        let changes = deduce(line, &s, need);
        if let Some(step) = c.step(technique, lines.cells[line].clone(), vec![line], changes) {
            steps.push(step);
        }
    }
    Ok(steps)
}

/// Bookkeeping: a line that has reached its clue has no more shaded cells.
fn line_complete(c: &Candidates<'_, Lines>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    per_line(c, "line_complete", |_, s, need| {
        if need == 0 {
            s.unknown.iter().map(|&(i, _)| (i, UNSHADED)).collect()
        } else {
            Vec::new()
        }
    })
}

/// A cell worth more than the line still needs is unshaded.
fn too_heavy(c: &Candidates<'_, Lines>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    per_line(c, "too_heavy", |_, s, need| {
        s.unknown
            .iter()
            .filter(|&&(_, w)| w > need)
            .map(|&(i, _)| (i, UNSHADED))
            .collect()
    })
}

/// A cell without which the other unknown cells cannot reach the total is shaded.
fn needed(c: &Candidates<'_, Lines>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    per_line(c, "needed", |_, s, need| {
        s.unknown
            .iter()
            .filter(|&&(_, w)| need > 0 && s.open - w < need)
            .map(|&(i, _)| (i, SHADED))
            .collect()
    })
}

/// Number of subsets of `weights` adding up to `total`, counted up to `cap`.
fn count_subsets(weights: &[u32], total: u32, cap: u32) -> u32 {
    let total = total as usize;
    let mut ways = vec![0u32; total + 1];
    ways[0] = 1;
    for &w in weights {
        let w = w as usize;
        for sum in (w..=total).rev() {
            ways[sum] = (ways[sum] + ways[sum - w]).min(cap);
        }
    }
    ways[total]
}

/// Cells that are the same in every way the unknown cells can reach the
/// line's total. `many` selects lines with more than a short list of ways.
fn combinations(
    c: &Candidates<'_, Lines>,
    technique: &'static str,
    many: bool,
) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let masks = c.masks;
    let lines = c.context;
    per_line(c, technique, |line, s, need| {
        let weights: Vec<_> = s.unknown.iter().map(|&(_, w)| w).collect();
        let ways = count_subsets(&weights, need, SHORT_LIST + 1);
        if ways == 0 || (ways > SHORT_LIST) != many {
            return Vec::new();
        }
        let mut cells: Vec<_> = lines.cells[line]
            .iter()
            .map(|&i| match masks[i] {
                SHADED => Cell::Shaded,
                BOTH => Cell::Unknown,
                _ => Cell::Empty,
            })
            .collect();
        if solve_full(&mut cells, lines.clues[line]).is_err() {
            return Vec::new();
        }
        lines.cells[line]
            .iter()
            .zip(cells)
            .filter(|&(&i, _)| masks[i] == BOTH)
            .filter_map(|(&i, cell)| match cell {
                Cell::Shaded => Some((i, SHADED)),
                Cell::Empty => Some((i, UNSHADED)),
                Cell::Unknown => None,
            })
            .collect()
    })
}

const fn entry(
    id: &'static str,
    level: Difficulty,
    kind: Kind,
    probes: bool,
    find: Find<Lines>,
) -> Technique<Lines> {
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

static TECHNIQUES: [Technique<Lines>; 7] = [
    entry("line_complete", Easy, Bookkeeping, false, line_complete),
    entry("too_heavy", Easy, Skill, false, too_heavy),
    entry("needed", Easy, Skill, false, needed),
    entry("combinations", Medium, Skill, false, |c| {
        combinations(c, "combinations", false)
    }),
    entry("many_combinations", Hard, Skill, false, |c| {
        combinations(c, "many_combinations", true)
    }),
    entry("short_contradiction", Hard, Skill, true, |c| {
        PROFILE.contradictions(c, "short_contradiction", Medium, SHORT_CHAIN)
    }),
    entry("contradiction", Expert, Skill, true, |c| {
        PROFILE.contradictions(c, "contradiction", Hard, LONG_CHAIN)
    }),
];

/// The canonical Kakurasu human profile.
pub(crate) static PROFILE: Profile<Lines> = Profile {
    id: "kakurasu-human",
    version: 1,
    techniques: &TECHNIQUES,
    broken,
};

/// Every cell open.
pub(crate) fn initial_masks(puzzle: &Puzzle) -> Vec<u16> {
    vec![BOTH; puzzle.rows() * puzzle.cols()]
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
    PROFILE.analyze(&Lines::new(puzzle), masks, placed, budget, cap, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pencil_puzzles_core::SplitMix64;

    /// Every completion of `puzzle` (rows x cols up to 4x4) that fits `masks`.
    fn completions(puzzle: &Puzzle, masks: &[u16]) -> Vec<u32> {
        let lines = Lines::new(puzzle);
        (0..1u32 << masks.len())
            .filter(|bits| {
                let full: Vec<u16> = (0..masks.len())
                    .map(|i| if bits >> i & 1 == 1 { SHADED } else { UNSHADED })
                    .collect();
                (0..masks.len()).all(|i| masks[i] & full[i] != 0) && !broken(&lines, &full)
            })
            .collect()
    }

    /// Every technique, including contradiction chains, keeps every
    /// completion of random partial states.
    #[test]
    fn techniques_keep_every_completion() {
        let mut rng = SplitMix64::new(7);
        let budget = Budget::unlimited();
        let mut fired = std::collections::BTreeSet::new();
        for trial in 0..400 {
            let (rows, cols) = (2 + trial % 3, 2 + (trial / 3) % 3);
            let truth: Vec<bool> = (0..rows * cols).map(|_| rng.below(2) == 0).collect();
            let shape = crate::Shape::new(rows, cols).unwrap();
            let mut answer = crate::Answer::new(shape);
            for (i, &shaded) in truth.iter().enumerate() {
                answer.set(
                    i / cols,
                    i % cols,
                    if shaded {
                        crate::Mark::Shaded
                    } else {
                        crate::Mark::Unshaded
                    },
                );
            }
            let puzzle = Puzzle::from_solution(&answer);
            let masks: Vec<u16> = truth
                .iter()
                .map(|&t| match rng.below(4) {
                    0 if t => SHADED,
                    0 => UNSHADED,
                    _ => BOTH,
                })
                .collect();
            let all = completions(&puzzle, &masks);
            assert!(!all.is_empty());
            let lines = Lines::new(&puzzle);
            let placed = vec![false; masks.len()];
            let candidates = Candidates::new(&lines, &masks, &placed, &budget);
            for technique in &TECHNIQUES {
                for step in technique.run(&candidates).unwrap() {
                    fired.insert(step.technique);
                    for &(i, keep) in &step.changes {
                        for bits in &all {
                            let value = if bits >> i & 1 == 1 { SHADED } else { UNSHADED };
                            assert_ne!(keep & value, 0, "{} removed a true value", step.technique);
                        }
                    }
                }
            }
        }
        for technique in [
            "line_complete",
            "too_heavy",
            "needed",
            "combinations",
            "short_contradiction",
        ] {
            assert!(
                fired.contains(technique),
                "{technique} never fired: {fired:?}"
            );
        }
    }

    #[test]
    fn short_lists_are_medium_and_long_ones_hard() {
        assert_eq!(count_subsets(&[1, 2, 3, 4], 5, 5), 2);
        assert_eq!(count_subsets(&[1, 2, 3, 4, 5, 6, 7, 8], 10, 5), 5);
        assert_eq!(count_subsets(&[2, 4], 3, 5), 0);
    }

    /// `broken` is exactly "not a solution" on every full shading, by the
    /// independent answer checker.
    #[test]
    fn broken_is_exact_on_full_grids() {
        let mut rng = SplitMix64::new(3);
        for trial in 0..60 {
            let (rows, cols) = (1 + trial % 3, 1 + (trial / 3) % 4);
            let shape = crate::Shape::new(rows, cols).unwrap();
            let mut clues = |n: usize, max: usize| {
                (0..n)
                    .map(|_| rng.below(max as u64 * (max as u64 + 1) / 2 + 1) as u32)
                    .collect::<Vec<_>>()
            };
            let row_clues = clues(rows, cols);
            let col_clues = clues(cols, rows);
            let puzzle = Puzzle::new(row_clues, col_clues).unwrap();
            let lines = Lines::new(&puzzle);
            for bits in 0..1u32 << (rows * cols) {
                let mut answer = crate::Answer::new(shape);
                let full: Vec<u16> = (0..rows * cols)
                    .map(|i| {
                        let shaded = bits >> i & 1 == 1;
                        let mark = if shaded {
                            crate::Mark::Shaded
                        } else {
                            crate::Mark::Unshaded
                        };
                        answer.set(i / cols, i % cols, mark);
                        if shaded { SHADED } else { UNSHADED }
                    })
                    .collect();
                assert_eq!(broken(&lines, &full), !puzzle.is_solved_by(&answer));
            }
        }
    }
}
