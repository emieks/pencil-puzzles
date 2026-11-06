//! The exact solver and the checker. Independent of the human techniques:
//! every cell keeps the set of sizes it could still have, for any size up
//! to the number of cells.
use crate::{Answer, BudgetExceeded, Mark, Puzzle};
use pencil_puzzles_core::grid::{flood, flood_from};
use pencil_puzzles_core::{Budget, ParseError, Place, Solutions, Violation};

/// Possible sizes of one cell: bit `v - 1` stands for size `v`. Grids have
/// at most 100 cells, so 128 bits cover every size.
type Sizes = u128;

fn bit(v: u32) -> Sizes {
    1 << (v - 1)
}

fn single(d: Sizes) -> Option<u32> {
    (d.count_ones() == 1).then(|| d.trailing_zeros() + 1)
}

/// The largest size any region could have: a region either holds a clue,
/// and has that clue's size, or lies inside a clue-free connected area.
pub(crate) fn size_bound(p: &Puzzle, nb: &[Vec<usize>]) -> u32 {
    let clues = p.clues();
    let mut bound = clues.iter().copied().max().unwrap_or(0);
    let mut seen = vec![false; clues.len()];
    for i in 0..clues.len() {
        if clues[i] == 0 && !seen[i] {
            let area = flood(i, |j| clues[j] == 0, nb);
            for &j in &area {
                seen[j] = true;
            }
            bound = bound.max(area.len() as u32);
        }
    }
    bound
}

#[derive(Clone)]
struct Board<'a> {
    nb: &'a [Vec<usize>],
    budget: &'a Budget,
    sizes: Vec<Sizes>,
}

impl Board<'_> {
    fn spend(&self, n: u64) -> Result<(), BudgetExceeded> {
        if self.budget.spend(n) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }

    /// Groups of fixed cells with equal sizes: the cells of each, and the
    /// group of every fixed cell.
    fn groups(&self) -> (Vec<Vec<usize>>, Vec<Option<usize>>) {
        let mut of = vec![None; self.sizes.len()];
        let mut groups = Vec::new();
        for i in 0..self.sizes.len() {
            if of[i].is_some() {
                continue;
            }
            let Some(v) = single(self.sizes[i]) else {
                continue;
            };
            let g = flood(i, |j| self.sizes[j] == bit(v), self.nb);
            for &j in &g {
                of[j] = Some(groups.len());
            }
            groups.push(g);
        }
        (groups, of)
    }

    /// Remove impossible sizes until nothing changes. Returns false if a
    /// cell has no size left or a group can never be right.
    fn propagate(&mut self) -> Result<bool, BudgetExceeded> {
        let n = self.sizes.len();
        loop {
            self.spend(n as u64)?;
            if self.sizes.contains(&0) {
                return Ok(false);
            }
            let before = self.sizes.clone();
            let (groups, of) = self.groups();
            for g in &groups {
                let v = single(self.sizes[g[0]]).expect("fixed");
                let size = g.len() as u32;
                if size > v {
                    return Ok(false);
                }
                // Neighbours that allow the size, including cells fixed to it
                // earlier in this pass, which join the group.
                let mut exits = Vec::new();
                for &i in g {
                    for &j in &self.nb[i] {
                        if self.sizes[j] & bit(v) != 0 && !g.contains(&j) {
                            exits.push(j);
                        }
                    }
                }
                exits.sort_unstable();
                exits.dedup();
                if size == v {
                    for j in exits {
                        if single(self.sizes[j]).is_none() {
                            self.sizes[j] &= !bit(v);
                        }
                    }
                } else if let [j] = exits[..] {
                    if single(self.sizes[j]).is_none() {
                        self.sizes[j] = bit(v);
                    }
                } else if exits.is_empty() {
                    return Ok(false);
                } else {
                    self.grow(g, v)?;
                }
            }
            // A cell can't take a size that would merge groups past it, or
            // that has too little room.
            let mut room = vec![0u32; n];
            let mut seen = vec![usize::MAX; n];
            for v in 1..=n as u32 {
                let sizes = &self.sizes;
                let allows = |j: usize| sizes[j] & bit(v) != 0;
                let mut remove = Vec::new();
                for i in 0..n {
                    if seen[i] == v as usize || !allows(i) {
                        continue;
                    }
                    let area = flood(i, allows, self.nb);
                    for &j in &area {
                        seen[j] = v as usize;
                        room[j] = area.len() as u32;
                    }
                }
                for i in 0..n {
                    if !allows(i) {
                        continue;
                    }
                    if room[i] < v {
                        if single(sizes[i]).is_some() {
                            return Ok(false);
                        }
                        remove.push(i);
                        continue;
                    }
                    if single(sizes[i]).is_none() {
                        let mut touching: Vec<usize> = self.nb[i]
                            .iter()
                            .filter(|&&j| sizes[j] == bit(v))
                            .filter_map(|&j| of[j])
                            .collect();
                        touching.sort_unstable();
                        touching.dedup();
                        let merged: usize =
                            1 + touching.iter().map(|&g| groups[g].len()).sum::<usize>();
                        if merged as u32 > v {
                            remove.push(i);
                        }
                    }
                }
                for i in remove {
                    self.sizes[i] &= !bit(v);
                }
            }
            if self.sizes == before {
                return Ok(!self.sizes.contains(&0));
            }
        }
    }

    /// For a group of size `v` that must still grow: fill its room if the
    /// room has exactly `v` cells, and take every cell it cannot do without.
    fn grow(&mut self, g: &[usize], v: u32) -> Result<(), BudgetExceeded> {
        let b = bit(v);
        let need = v as usize;
        let reach = need - g.len();
        // Distances from the group through cells allowing the size.
        let mut dist = vec![usize::MAX; self.sizes.len()];
        let mut area = g.to_vec();
        for &i in g {
            dist[i] = 0;
        }
        let mut at = 0;
        while at < area.len() {
            let i = area[at];
            at += 1;
            for &j in &self.nb[i] {
                if dist[j] == usize::MAX && self.sizes[j] & b != 0 {
                    dist[j] = dist[i] + 1;
                    area.push(j);
                }
            }
        }
        self.spend(area.len() as u64)?;
        if area.len() == need {
            for &j in &area {
                self.sizes[j] = b;
            }
            return Ok(());
        }
        let candidates: Vec<usize> = area
            .iter()
            .copied()
            .filter(|&y| dist[y] <= reach && single(self.sizes[y]).is_none())
            .collect();
        for y in candidates {
            self.spend(need as u64)?;
            let without = flood_from(g, |j| j != y && self.sizes[j] & b != 0, self.nb, need).len();
            if without < need {
                self.sizes[y] = b;
            }
        }
        Ok(())
    }

    fn answer(&self, shape: crate::Shape) -> Answer {
        let mut a = Answer::new(shape);
        for (i, &d) in self.sizes.iter().enumerate() {
            let v = single(d).expect("solved");
            a.set(i / shape.cols(), i % shape.cols(), Mark::Number(v));
        }
        a
    }

    /// The open cell to branch on: one next to an incomplete group if any
    /// (fewest sizes first), else the open cell with the fewest sizes.
    fn branch_cell(&self) -> Option<usize> {
        let open = |i: usize| single(self.sizes[i]).is_none();
        let growing = (0..self.sizes.len()).filter(|&i| {
            open(i)
                && self.nb[i]
                    .iter()
                    .any(|&j| single(self.sizes[j]).is_some_and(|v| self.sizes[i] & bit(v) != 0))
        });
        let fewest = |cells: &mut dyn Iterator<Item = usize>| {
            cells.min_by_key(|&i| (self.sizes[i].count_ones(), i))
        };
        fewest(&mut { growing }).or_else(|| fewest(&mut (0..self.sizes.len()).filter(|&i| open(i))))
    }
}

fn search(
    b: &mut Board<'_>,
    shape: crate::Shape,
    limit: usize,
    found: &mut usize,
    visit: &mut dyn FnMut(Answer),
) -> Result<(), BudgetExceeded> {
    // Each level fixes one more cell, so recursion is at most one per cell.
    if *found >= limit {
        return Ok(());
    }
    b.spend(1)?;
    if !b.propagate()? {
        return Ok(());
    }
    let Some(i) = b.branch_cell() else {
        *found += 1;
        visit(b.answer(shape));
        return Ok(());
    };
    let mut rest = b.sizes[i];
    while rest != 0 {
        let v = rest.trailing_zeros() + 1;
        rest &= !bit(v);
        let mut next = b.clone();
        next.sizes[i] = bit(v);
        search(&mut next, shape, limit, found, visit)?;
        if *found >= limit {
            break;
        }
    }
    Ok(())
}

fn start<'a>(p: &Puzzle, nb: &'a [Vec<usize>], budget: &'a Budget) -> Board<'a> {
    let n = p.clues().len() as u32;
    let bound = size_bound(p, nb);
    let mut clue_sizes: Sizes = 0;
    for &c in p.clues().iter().filter(|&&c| c > 0) {
        clue_sizes |= bit(c);
    }
    let mut sizes = Vec::with_capacity(p.clues().len());
    let mut area_of = vec![0u32; p.clues().len()];
    let mut seen = vec![false; p.clues().len()];
    for i in 0..p.clues().len() {
        if p.clues()[i] == 0 && !seen[i] {
            let area = flood(i, |j| p.clues()[j] == 0, nb);
            for &j in &area {
                seen[j] = true;
                area_of[j] = area.len() as u32;
            }
        }
    }
    for (i, &c) in p.clues().iter().enumerate() {
        sizes.push(if c > 0 {
            bit(c)
        } else {
            // Unclued regions fit inside the cell's clue-free area.
            let up_to = area_of[i].min(bound).min(n);
            (Sizes::MAX >> (128 - up_to)) | clue_sizes
        });
    }
    Board { nb, budget, sizes }
}

fn run(
    p: &Puzzle,
    limit: usize,
    budget: &Budget,
    visit: &mut dyn FnMut(Answer),
) -> Result<usize, BudgetExceeded> {
    let nb = pencil_puzzles_core::grid::neighbours(p.shape().rows(), p.shape().cols());
    let mut b = start(p, &nb, budget);
    let mut found = 0;
    search(&mut b, p.shape(), limit, &mut found, visit)?;
    Ok(found)
}

/// Prove zero, one or more solutions; two different ones when there are more.
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut answers = Vec::new();
    run(p, 2, budget, &mut |a| answers.push(a))?;
    Ok(match answers.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(answers.remove(0)),
        _ => Solutions::Multiple(answers.remove(0), answers.remove(0)),
    })
}

/// Count solutions, stopping at `limit`.
pub fn count_solutions(p: &Puzzle, limit: usize, budget: &Budget) -> Result<usize, BudgetExceeded> {
    run(p, limit, budget, &mut |_| {})
}

pub(crate) fn cell(i: usize, cols: usize) -> Place {
    Place::Cell {
        row: (i / cols) as u32,
        col: (i % cols) as u32,
    }
}

/// Every broken rule of a complete or partial answer (P8). Unmarked cells
/// could still take any size, so a group is reported only when no way of
/// filling them in could make it right.
pub fn check(
    p: &Puzzle,
    a: &Answer,
    budget: &Budget,
) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
    let shape = p.shape();
    if a.shape() != shape {
        return Err(ParseError::new("the answer has the wrong shape").into());
    }
    let n = shape.cells();
    if !budget.spend((n * n) as u64) {
        return Err(Box::new(BudgetExceeded));
    }
    let cols = shape.cols();
    let nb = pencil_puzzles_core::grid::neighbours(shape.rows(), cols);
    let marks: Vec<Mark> = (0..n).map(|i| a.get(i / cols, i % cols)).collect();
    let mut violations = Vec::new();
    let number = |i: usize| match marks[i] {
        Mark::Number(v) => Some(v),
        _ => None,
    };
    let mut seen = vec![false; n];
    for i in 0..n {
        let Some(v) = number(i) else {
            continue;
        };
        if seen[i] {
            continue;
        }
        let group = flood(i, |j| number(j) == Some(v), &nb);
        for &j in &group {
            seen[j] = true;
        }
        let size = group.len();
        // Room to grow: unmarked cells and same-numbered cells around it.
        let room = flood(
            i,
            |j| marks[j] == Mark::Unmarked || number(j) == Some(v),
            &nb,
        )
        .len();
        if size as u64 > u64::from(v) || (room as u64) < u64::from(v) {
            let first = *group.iter().min().expect("nonempty");
            violations.push(Violation::new(
                1,
                cell(first, cols),
                vec![("number", i64::from(v)), ("cells", size as i64)],
            ));
        }
    }
    for (i, (&mark, &c)) in marks.iter().zip(p.clues()).enumerate() {
        if mark == Mark::Empty {
            violations.push(Violation::new(2, cell(i, cols), Vec::new()));
        }
        if c > 0 && (mark == Mark::Empty || number(i).is_some_and(|v| v != c)) {
            violations.push(Violation::new(
                3,
                cell(i, cols),
                vec![("clue", i64::from(c))],
            ));
        }
    }
    violations.sort_by_key(|v| (v.rule, v.place));
    Ok(violations)
}
