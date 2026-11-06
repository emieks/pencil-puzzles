//! Bounded iterative reachability, rule instances and complete binary search.
use crate::*;
use pencil_puzzles_core::grid::{self, flood};
use std::rc::Rc;
pub(crate) const LAND: u16 = 1;
pub(crate) const SEA: u16 = 2;
#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Island(usize, u32),
    Numbers,
    Sea,
    Pool,
}
#[derive(Clone, Debug)]
pub(crate) struct Instance {
    pub kind: Kind,
    pub scope: Vec<usize>,
    pub place: Place,
    pub rule: u32,
}
#[derive(Clone)]
pub(crate) struct Topology {
    pub neighbours: Vec<Vec<usize>>,
    pub instances: Vec<Instance>,
}
impl Topology {
    pub fn new(p: &Puzzle) -> Self {
        let sh = p.shape();
        let (rows, cols) = (sh.rows(), sh.cols());
        let v = sh.cells();
        let neighbours = grid::neighbours(rows, cols);
        let mut instances = Vec::new();
        for (i, &n) in p.clues().iter().enumerate().filter(|(_, n)| **n > 0) {
            let scope = (0..v)
                .filter(|&j| {
                    (i / cols).abs_diff(j / cols) + (i % cols).abs_diff(j % cols) <= n as usize
                })
                .collect();
            instances.push(Instance {
                kind: Kind::Island(i, n),
                scope,
                place: cell(i, cols),
                rule: 1,
            });
        }
        for (kind, rule) in [(Kind::Numbers, 2), (Kind::Sea, 3)] {
            instances.push(Instance {
                kind,
                scope: (0..v).collect(),
                place: Place::Board,
                rule,
            });
        }
        for r in 0..rows.saturating_sub(1) {
            for c in 0..cols.saturating_sub(1) {
                let i = r * cols + c;
                instances.push(Instance {
                    kind: Kind::Pool,
                    scope: vec![i, i + 1, i + cols, i + cols + 1],
                    place: cell(i, cols),
                    rule: 4,
                });
            }
        }
        Self {
            neighbours,
            instances,
        }
    }
}
pub(crate) fn cell(i: usize, cols: usize) -> Place {
    Place::Cell {
        row: (i / cols) as u32,
        col: (i % cols) as u32,
    }
}
#[derive(Clone)]
pub(crate) struct Board<'a> {
    pub puzzle: &'a Puzzle,
    pub budget: &'a Budget,
    pub topology: Rc<Topology>,
    pub masks: Vec<u16>,
}
impl<'a> Board<'a> {
    pub fn new(puzzle: &'a Puzzle, budget: &'a Budget) -> Self {
        Self {
            puzzle,
            budget,
            topology: Rc::new(Topology::new(puzzle)),
            masks: puzzle
                .clues()
                .iter()
                .map(|&n| if n > 0 { LAND } else { LAND | SEA })
                .collect(),
        }
    }
    pub fn spend(&self) -> Result<(), BudgetExceeded> {
        if self.budget.spend(1) {
            Ok(())
        } else {
            Err(BudgetExceeded)
        }
    }
    fn island_bounds(&self, k: usize, m: &[u16]) -> (usize, usize) {
        let instance = &self.topology.instances[k];
        let Kind::Island(origin, _) = instance.kind else {
            unreachable!()
        };
        let mut fixed = vec![false; m.len()];
        let mut possible = fixed.clone();
        for &i in &instance.scope {
            fixed[i] = m[i] == LAND;
            possible[i] = m[i] & LAND != 0;
        }
        (
            flood(origin, |j| fixed[j], &self.topology.neighbours).len(),
            flood(origin, |j| possible[j], &self.topology.neighbours).len(),
        )
    }
    pub fn broken(&self, k: usize, m: &[u16], tier: u8) -> Result<bool, BudgetExceeded> {
        self.spend()?;
        let instance = &self.topology.instances[k];
        if instance.scope.iter().any(|&i| m[i] == 0) {
            return Ok(true);
        }
        Ok(match instance.kind {
            Kind::Island(_, n) => {
                let (min, max) = self.island_bounds(k, m);
                min > n as usize || max < (n as usize)
            }
            Kind::Pool => instance.scope.iter().all(|&i| m[i] == SEA),
            Kind::Sea => {
                let possible: Vec<_> = m.iter().map(|v| v & SEA != 0).collect();
                let Some(start) = possible.iter().position(|&v| v) else {
                    return Ok(true);
                };
                if tier == 1 {
                    false
                } else {
                    let required: Vec<_> = m
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &v)| (v == SEA).then_some(i))
                        .collect();
                    let reached = flood(
                        required.first().copied().unwrap_or(start),
                        |j| possible[j],
                        &self.topology.neighbours,
                    );
                    required.iter().any(|i| !reached.contains(i))
                }
            }
            Kind::Numbers => {
                if tier == 1 {
                    return Ok(false);
                }
                let fixed: Vec<_> = m.iter().map(|&v| v == LAND).collect();
                let possible: Vec<_> = m.iter().map(|v| v & LAND != 0).collect();
                let mut visited = vec![false; m.len()];
                for i in 0..m.len() {
                    if fixed[i] && !visited[i] {
                        let component = flood(i, |j| fixed[j], &self.topology.neighbours);
                        for &j in &component {
                            visited[j] = true;
                        }
                        if component
                            .iter()
                            .filter(|&&j| self.puzzle.clues()[j] > 0)
                            .count()
                            > 1
                        {
                            return Ok(true);
                        }
                        let reachable = flood(i, |j| possible[j], &self.topology.neighbours);
                        if !reachable.iter().any(|&j| self.puzzle.clues()[j] > 0) {
                            return Ok(true);
                        }
                    }
                }
                false
            }
        })
    }
    pub fn unit(&self, k: usize, tier: u8) -> Result<Vec<(usize, u16)>, BudgetExceeded> {
        let inst = &self.topology.instances[k];
        if tier == 1 && matches!(inst.kind, Kind::Numbers) {
            return Ok(Vec::new());
        }
        if self.broken(k, &self.masks, tier)? {
            return Ok(inst.scope.iter().map(|&i| (i, 0)).collect());
        }
        let mut changes = Vec::new();
        let mut m = self.masks.clone();
        for &i in &inst.scope {
            if self.masks[i].count_ones() < 2 {
                continue;
            }
            let mut keep = self.masks[i];
            for bit in [LAND, SEA] {
                if keep & bit != 0 {
                    m[i] = bit;
                    if self.broken(k, &m, tier)? {
                        keep &= !bit;
                    }
                }
            }
            m[i] = self.masks[i];
            if keep != self.masks[i] {
                changes.push((i, keep));
            }
        }
        Ok(changes)
    }
    pub fn answer(&self) -> Answer {
        let mut a = Answer::new(self.puzzle.shape());
        for (i, &m) in self.masks.iter().enumerate() {
            a.set(
                i / self.puzzle.shape().cols(),
                i % self.puzzle.shape().cols(),
                if m == SEA {
                    Mark::Shaded
                } else {
                    Mark::Unshaded
                },
            );
        }
        a
    }
    /// Whether some cell has no candidate left.
    pub fn contradictory(&self) -> bool {
        self.masks.contains(&0)
    }
    pub fn propagate(&mut self) -> Result<bool, BudgetExceeded> {
        loop {
            if self.contradictory() {
                return Ok(false);
            }
            let mut changed = false;
            for tier in 1..=2 {
                for k in 0..self.topology.instances.len() {
                    let changes = self.unit(k, tier)?;
                    for (i, keep) in changes {
                        if self.masks[i] != keep {
                            self.masks[i] &= keep;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                return Ok(!self.contradictory());
            }
        }
    }
}
fn search(
    mut b: Board<'_>,
    limit: usize,
    found: &mut usize,
    mut visit: impl FnMut(Answer),
) -> Result<(), BudgetExceeded> {
    // Explicit recursive visitor shares output storage; recursion is at most one per cell.
    fn dfs(
        b: &mut Board<'_>,
        limit: usize,
        found: &mut usize,
        visit: &mut dyn FnMut(Answer),
    ) -> Result<(), BudgetExceeded> {
        if *found >= limit {
            return Ok(());
        }
        b.spend()?;
        if !b.propagate()? {
            return Ok(());
        }
        if let Some(i) = b.masks.iter().position(|m| m.count_ones() > 1) {
            for bit in [LAND, SEA] {
                if b.masks[i] & bit != 0 {
                    let mut next = b.clone();
                    next.masks[i] = bit;
                    dfs(&mut next, limit, found, visit)?;
                    if *found >= limit {
                        break;
                    }
                }
            }
        } else {
            *found += 1;
            visit(b.answer());
        }
        Ok(())
    }
    dfs(&mut b, limit, found, &mut visit)
}
/// Prove zero, one or multiple solutions; budget exhaustion is an error.
pub fn solve(p: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut answers = Vec::new();
    search(Board::new(p, budget), 2, &mut 0, |a| answers.push(a))?;
    Ok(match answers.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(answers.remove(0)),
        _ => Solutions::Multiple(answers.remove(0), answers.remove(0)),
    })
}
/// Count solutions up to a caller-supplied limit without retaining witnesses.
pub fn count_solutions(p: &Puzzle, limit: usize, budget: &Budget) -> Result<usize, BudgetExceeded> {
    let mut n = 0;
    search(Board::new(p, budget), limit, &mut n, |_| {})?;
    Ok(n)
}
/// Report broken instances under complete or partial marks, using a deterministic budget.
pub fn check(
    p: &Puzzle,
    a: &Answer,
    budget: &Budget,
) -> Result<Vec<Violation>, Box<dyn std::error::Error + Send + Sync>> {
    if a.shape() != p.shape() {
        return Err(ParseError::new("the answer has the wrong shape").into());
    }
    let mut b = Board::new(p, budget);
    let mut violations = Vec::new();
    for i in 0..b.masks.len() {
        let mark = a.get(i / p.shape().cols(), i % p.shape().cols());
        if p.clues()[i] > 0 && mark == Mark::Shaded {
            violations.push(Violation::new(
                5,
                cell(i, p.shape().cols()),
                vec![("clue", p.clues()[i] as i64)],
            ));
        }
        if p.clues()[i] == 0 {
            b.masks[i] = match mark {
                Mark::Unmarked => LAND | SEA,
                Mark::Unshaded => LAND,
                Mark::Shaded => SEA,
            };
        }
    }
    for (k, inst) in b.topology.instances.iter().enumerate() {
        if b.broken(k, &b.masks, 2)? {
            let values = match inst.kind {
                Kind::Island(_, n) => {
                    let (min, max) = b.island_bounds(k, &b.masks);
                    vec![
                        ("minimum", min as i64),
                        ("maximum", max as i64),
                        ("clue", n as i64),
                    ]
                }
                Kind::Pool => vec![("shaded", 4)],
                _ => vec![(
                    "components",
                    component_count(
                        &b,
                        if matches!(inst.kind, Kind::Sea) {
                            SEA
                        } else {
                            LAND
                        },
                    ) as i64,
                )],
            };
            violations.push(Violation::new(inst.rule, inst.place, values));
        }
    }
    Ok(violations)
}
fn component_count(b: &Board<'_>, bit: u16) -> usize {
    let allowed: Vec<_> = b.masks.iter().map(|&m| m == bit).collect();
    let mut seen = vec![false; allowed.len()];
    let mut n = 0;
    for i in 0..allowed.len() {
        if allowed[i] && !seen[i] {
            n += 1;
            for j in flood(i, |j| allowed[j], &b.topology.neighbours) {
                seen[j] = true;
            }
        }
    }
    n
}
