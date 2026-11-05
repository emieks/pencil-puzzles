//! All-different constraints, polynomial matching, bounded search and checking.
use crate::model::EMPTY;
use crate::{
    Answer, Budget, BudgetExceeded, ParseError, Place, Puzzle, Shape, Solutions, Violation,
};
use std::rc::Rc;
type UnitSupport = [u16; crate::MAX_SIZE];

#[derive(Clone, Debug)]
pub(crate) struct Topology {
    pub n: usize,
    pub units: Vec<Vec<usize>>,
    pub pairs: Vec<(usize, usize)>,
    #[cfg(test)]
    unit_bits: Vec<u128>,
}
impl Topology {
    pub fn new(shape: Shape) -> Self {
        let n = shape.size();
        let b = shape.box_size();
        let mut units: Vec<Vec<usize>> = Vec::new();
        for r in 0..n {
            units.push((0..n).map(|c| r * n + c).collect());
        }
        for c in 0..n {
            units.push((0..n).map(|r| r * n + c).collect());
        }
        for k in 0..n {
            units.push(
                (0..n)
                    .map(|j| ((k / b) * b + j / b) * n + (k % b) * b + j % b)
                    .collect(),
            );
        }
        let mut pairs = Vec::new();
        for a in 0..2 * n {
            for c in 2 * n..3 * n {
                if units[a].iter().filter(|i| units[c].contains(i)).count() >= 2 {
                    pairs.push((a, c));
                }
            }
        }
        #[cfg(test)]
        let unit_bits = units
            .iter()
            .map(|u| u.iter().fold(0u128, |m, &i| m | (1u128 << i)))
            .collect();
        Self {
            n,
            units,
            pairs,
            #[cfg(test)]
            unit_bits,
        }
    }
    /// Exact support with fixed-size scratch storage; callers charge work.
    fn supported(&self, masks: &[u16]) -> Option<UnitSupport> {
        matching_array(masks, self.n)
    }
    pub fn place(&self, unit: usize) -> Place {
        if unit < self.n {
            Place::Row(unit as u32)
        } else if unit < 2 * self.n {
            Place::Col((unit - self.n) as u32)
        } else {
            Place::Region((unit - 2 * self.n) as u32)
        }
    }
}
/// Complete all-different support via a matching and alternating-cycle reachability.
fn matching_array(masks: &[u16], n: usize) -> Option<UnitSupport> {
    if masks.iter().all(|m| m.count_ones() == 1) {
        if masks.iter().fold(0, |a, &b| a | b) != (1 << n) - 1 {
            return None;
        }
        let mut result = [0; crate::MAX_SIZE];
        result[..n].copy_from_slice(masks);
        return Some(result);
    }
    fn augment(cell: usize, masks: &[u16], seen: &mut u16, owner: &mut [usize]) -> bool {
        let mut candidates = masks[cell];
        while candidates != 0 {
            let v = candidates.trailing_zeros() as usize;
            let bit = 1 << v;
            candidates &= !bit;
            if *seen & bit != 0 {
                continue;
            }
            *seen |= bit;
            if owner[v] == usize::MAX || augment(owner[v], masks, seen, owner) {
                owner[v] = cell;
                return true;
            }
        }
        false
    }
    let mut owners = [usize::MAX; crate::MAX_SIZE];
    let owner = &mut owners[..n];
    for cell in 0..n {
        if !augment(cell, masks, &mut 0, owner) {
            return None;
        }
    }
    let mut reach = [0u16; crate::MAX_SIZE];
    for (cell, &mask) in masks.iter().enumerate() {
        for (v, &other) in owner.iter().enumerate() {
            if mask & (1 << v) != 0 {
                reach[cell] |= 1 << other;
            }
        }
    }
    for k in 0..n {
        for i in 0..n {
            if reach[i] & (1 << k) != 0 {
                reach[i] |= reach[k];
            }
        }
    }
    let mut result = [0; crate::MAX_SIZE];
    for (i, &mask) in masks.iter().enumerate() {
        for (v, &j) in owner.iter().enumerate() {
            if mask & (1 << v) != 0 && (i == j || reach[j] & (1 << i) != 0) {
                result[i] |= 1 << v;
            }
        }
    }
    Some(result)
}
#[cfg(test)]
fn matching(masks: &[u16], n: usize) -> Option<Vec<u16>> {
    matching_array(masks, n).map(|v| v[..n].to_vec())
}
/// Bounds on one occurrence of each digit in a unit.
fn bounds_array(masks: &[u16], n: usize) -> Option<UnitSupport> {
    if masks.contains(&0) {
        return None;
    }
    let mut keep = [0; crate::MAX_SIZE];
    keep[..n].copy_from_slice(masks);
    for v in 0..n {
        let bit = 1 << v;
        let mut possible = 0u16;
        let mut fixed = 0u16;
        for (i, &mask) in masks.iter().enumerate() {
            if mask & bit != 0 {
                possible |= 1 << i;
            }
            if mask == bit {
                fixed |= 1 << i;
            }
        }
        if possible == 0 || fixed.count_ones() > 1 {
            return None;
        }
        if possible.count_ones() == 1 {
            keep[possible.trailing_zeros() as usize] &= bit;
        }
        if fixed != 0 {
            let cell = fixed.trailing_zeros() as usize;
            for (i, k) in keep.iter_mut().enumerate() {
                if i != cell {
                    *k &= !bit;
                }
            }
        }
    }

    if keep[..n].contains(&0) {
        None
    } else {
        Some(keep)
    }
}
#[cfg(test)]
fn bounds(masks: &[u16], n: usize) -> Option<Vec<u16>> {
    bounds_array(masks, n).map(|v| v[..n].to_vec())
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
        let n = puzzle.shape().size();
        Self {
            puzzle,
            budget,
            topology: Rc::new(Topology::new(puzzle.shape())),
            masks: puzzle
                .givens()
                .iter()
                .map(|&v| if v == 0 { (1 << n) - 1 } else { 1 << (v - 1) })
                .collect(),
        }
    }
    pub fn contradictory(&self) -> bool {
        self.masks.contains(&0)
    }
    pub fn branch_cell(&self) -> Option<usize> {
        self.masks
            .iter()
            .enumerate()
            .filter(|(_, m)| m.count_ones() > 1)
            .min_by_key(|&(i, m)| (m.count_ones(), i))
            .map(|(i, _)| i)
    }
    pub fn answer(&self) -> Answer {
        Answer {
            shape: self.puzzle.shape(),
            cells: self
                .masks
                .iter()
                .map(|m| {
                    if m.count_ones() == 1 {
                        m.trailing_zeros() as u8 + 1
                    } else {
                        0
                    }
                })
                .collect(),
        }
    }
    pub fn unit(&self, unit: usize, tier: u8) -> Result<Vec<(usize, u16)>, BudgetExceeded> {
        if !self.budget.spend(1) {
            return Err(BudgetExceeded);
        }
        let cells = &self.topology.units[unit];
        let mut masks = [0; crate::MAX_SIZE];
        for (j, &i) in cells.iter().enumerate() {
            masks[j] = self.masks[i];
        }
        let masks = &masks[..self.topology.n];
        let result = if tier == 1 {
            bounds_array(masks, self.topology.n)
        } else {
            self.topology.supported(masks)
        };
        let keep = result.unwrap_or([0; crate::MAX_SIZE]);
        Ok(cells
            .iter()
            .copied()
            .zip(keep)
            .filter(|&(i, k)| self.masks[i] & k != self.masks[i])
            .collect())
    }
    #[cfg(test)]
    pub fn pair(&self, a: usize, b: usize) -> Result<Vec<(usize, u16)>, BudgetExceeded> {
        if !self.budget.spend(1) {
            return Err(BudgetExceeded);
        }
        let first = &self.topology.units[a];
        let second = &self.topology.units[b];
        let union = self.topology.unit_bits[a] | self.topology.unit_bits[b];
        let mut next = [0u16; crate::MAX_SIZE * crate::MAX_SIZE];
        let mut cells = union;
        while cells != 0 {
            let i = cells.trailing_zeros() as usize;
            cells &= cells - 1;
            next[i] = self.masks[i];
        }
        let broken = [first, second].iter().any(|unit| {
            let mut masks = [0; crate::MAX_SIZE];
            for (j, &i) in unit.iter().enumerate() {
                masks[j] = self.masks[i];
            }
            self.topology.supported(&masks[..self.topology.n]).is_none()
        });
        if broken {
            for &i in first.iter().chain(second) {
                next[i] = 0;
            }
        } else {
            for (from, to) in [(a, b), (b, a)] {
                for v in 0..self.topology.n {
                    let bit = 1 << v;
                    let mut any = false;
                    let mut confined = true;
                    for &i in &self.topology.units[from] {
                        if self.masks[i] & bit != 0 {
                            any = true;
                            if self.topology.unit_bits[to] & (1u128 << i) == 0 {
                                confined = false;
                                break;
                            }
                        }
                    }
                    if any && confined {
                        for &i in &self.topology.units[to] {
                            if self.topology.unit_bits[from] & (1u128 << i) == 0 {
                                next[i] &= !bit;
                            }
                        }
                    }
                }
            }
        }
        let mut changes = Vec::new();
        let mut cells = union;
        while cells != 0 {
            let i = cells.trailing_zeros() as usize;
            cells &= cells - 1;
            if self.masks[i] & next[i] != self.masks[i] {
                changes.push((i, next[i]));
            }
        }
        Ok(changes)
    }

    pub fn apply(&mut self, changes: &[(usize, u16)]) {
        for &(i, keep) in changes {
            self.masks[i] &= keep;
        }
    }
    pub fn propagate(&mut self) -> Result<bool, BudgetExceeded> {
        loop {
            if self.contradictory() {
                return Ok(false);
            }
            let mut changed = false;
            for unit in 0..self.topology.units.len() {
                let changes = self.unit(unit, 2)?;
                changed |= !changes.is_empty();
                self.apply(&changes);
                if self.contradictory() {
                    return Ok(false);
                }
            }
            if !changed {
                return Ok(true);
            }
        }
    }
}
fn search(
    mut board: Board<'_>,
    visit: &mut impl FnMut(Answer) -> bool,
) -> Result<bool, BudgetExceeded> {
    if !board.budget.spend(1) {
        return Err(BudgetExceeded);
    }
    if !board.propagate()? {
        return Ok(false);
    }
    let Some(i) = board.branch_cell() else {
        return Ok(visit(board.answer()));
    };
    for v in 0..board.topology.n {
        let bit = 1 << v;
        if board.masks[i] & bit == 0 {
            continue;
        }
        let mut next = board.clone();
        next.masks[i] = bit;
        if search(next, visit)? {
            return Ok(true);
        }
    }
    Ok(false)
}
/// Solve with proof of uniqueness and at most two distinct witnesses (G1).
pub fn solve(puzzle: &Puzzle, budget: &Budget) -> Result<Solutions<Answer>, BudgetExceeded> {
    let mut answers = Vec::new();
    search(Board::new(puzzle, budget), &mut |a| {
        answers.push(a);
        answers.len() == 2
    })?;
    Ok(match answers.len() {
        0 => Solutions::None,
        1 => Solutions::Unique(answers.remove(0)),
        _ => Solutions::Multiple(answers.remove(0), answers.remove(0)),
    })
}
/// Count solutions, stopping at the caller's limit.
pub fn count_solutions(
    puzzle: &Puzzle,
    limit: usize,
    budget: &Budget,
) -> Result<usize, BudgetExceeded> {
    let mut count = 0;
    if limit > 0 {
        search(Board::new(puzzle, budget), &mut |_| {
            count += 1;
            count == limit
        })?;
    }
    budget.check()?;
    Ok(count)
}
/// Check R1-R4. Partial units are reported only if no matching can complete them.
/// Finished empty marks (`0`) have no legal digit and break their units.
pub fn check(puzzle: &Puzzle, answer: &Answer) -> Result<Vec<Violation>, ParseError> {
    if answer.shape() != puzzle.shape() {
        return Err(ParseError::new("the answer has the wrong Sudoku shape"));
    }
    let n = puzzle.shape().size();
    let topology = Topology::new(puzzle.shape());
    let masks: Vec<_> = answer
        .cells
        .iter()
        .enumerate()
        .map(|(i, &v)| match v {
            0 => {
                if puzzle.givens()[i] == 0 {
                    (1 << n) - 1
                } else {
                    1 << (puzzle.givens()[i] - 1)
                }
            }
            EMPTY => 0,
            _ => 1 << (v - 1),
        })
        .collect();
    let mut result = Vec::new();
    for (unit, cells) in topology.units.iter().enumerate() {
        let candidates: Vec<_> = cells.iter().map(|&i| masks[i]).collect();
        if matching_array(&candidates, n).is_none() {
            result.push(Violation::new(
                (unit / n + 1) as u32,
                topology.place(unit),
                vec![
                    (
                        "filled",
                        candidates.iter().filter(|m| m.count_ones() == 1).count() as i64,
                    ),
                    ("size", n as i64),
                ],
            ));
        }
    }
    for (i, (&given, &actual)) in puzzle.givens().iter().zip(&answer.cells).enumerate() {
        if given != 0 && actual != 0 && actual != given {
            result.push(Violation::new(
                4,
                Place::Cell {
                    row: (i / n) as u32,
                    col: (i % n) as u32,
                },
                vec![
                    ("given", i64::from(given)),
                    (
                        "actual",
                        if actual == EMPTY {
                            0
                        } else {
                            i64::from(actual)
                        },
                    ),
                ],
            ));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pencil_puzzles_core::SplitMix64;
    fn permutations(masks: &[u16], n: usize) -> Vec<Vec<usize>> {
        fn enumerate(masks: &[u16], n: usize, values: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
            let cell = values.len();
            if cell == n {
                out.push(values.clone());
                return;
            }
            for v in 0..n {
                if masks[cell] & (1 << v) != 0 && !values.contains(&v) {
                    values.push(v);
                    enumerate(masks, n, values, out);
                    values.pop();
                }
            }
        }
        let mut out = Vec::new();
        enumerate(masks, n, &mut Vec::new(), &mut out);
        out
    }
    fn reference_bounds(masks: &[u16], n: usize) -> Option<Vec<u16>> {
        let mut result = vec![0; n];
        for cell in 0..n {
            for value in 0..n {
                if masks[cell] & (1 << value) == 0 {
                    continue;
                }
                let possible = (0..n).all(|digit| {
                    let mut low = 0;
                    let mut high = 0;
                    for (i, &mask) in masks.iter().enumerate() {
                        let values = if i == cell { 1 << value } else { mask };
                        if values == 1 << digit {
                            low += 1;
                        }
                        if values & (1 << digit) != 0 {
                            high += 1;
                        }
                    }
                    low <= 1 && high >= 1
                });
                if possible {
                    result[cell] |= 1 << value;
                }
            }
        }
        if result.contains(&0) {
            None
        } else {
            Some(result)
        }
    }
    #[test]
    fn matching_is_complete_and_bounds_are_exact_sound_and_monotone() {
        let mut rng = SplitMix64::new(339);
        for n in [1, 4] {
            for _ in 0..10_000 {
                let masks: Vec<_> = (0..n).map(|_| rng.below(1 << n) as u16).collect();
                let permutations = permutations(&masks, n);
                let actual = matching(&masks, n);
                let expected = if permutations.is_empty() {
                    None
                } else {
                    Some(
                        (0..n)
                            .map(|i| permutations.iter().fold(0, |m, p| m | 1 << p[i]))
                            .collect(),
                    )
                };
                assert_eq!(actual, expected, "{masks:?}");
                assert_eq!(bounds(&masks, n), reference_bounds(&masks, n));
                if let Some(keep) = bounds(&masks, n) {
                    for p in &permutations {
                        for i in 0..n {
                            assert_ne!(keep[i] & (1 << p[i]), 0);
                        }
                    }
                }
                let stronger: Vec<_> = masks
                    .iter()
                    .map(|&m| m & rng.below(1 << n) as u16)
                    .collect();
                for technique in [bounds, matching] {
                    let weak = technique(&masks, n).unwrap_or_else(|| vec![0; n]);
                    let strong = technique(&stronger, n).unwrap_or_else(|| vec![0; n]);
                    for i in 0..n {
                        assert_eq!(strong[i] & masks[i] & !weak[i], 0);
                    }
                }
            }
        }
    }
    #[test]
    fn locked_pairs_are_sound_and_monotone_even_in_broken_states() {
        let mut rng = SplitMix64::new(717);
        let truth = [1u8, 2, 3, 4, 3, 4, 1, 2, 2, 1, 4, 3, 4, 3, 2, 1];
        let p = Puzzle::new(Shape::new(4).unwrap(), vec![0; 16]).unwrap();
        let budget = Budget::unlimited();
        for _ in 0..200 {
            let mut b = Board::new(&p, &budget);
            for (i, m) in b.masks.iter_mut().enumerate() {
                *m = (rng.below(16) as u16) | (1 << (truth[i] - 1));
            }
            let mut stronger = b.clone();
            for m in &mut stronger.masks {
                *m &= rng.below(16) as u16;
            }
            for &(a, c) in &b.topology.pairs {
                let changes = b.pair(a, c).unwrap();
                let mut weak = b.clone();
                weak.apply(&changes);
                for (i, &v) in truth.iter().enumerate() {
                    assert_ne!(weak.masks[i] & (1 << (v - 1)), 0);
                }
                let changes = stronger.pair(a, c).unwrap();
                let mut strong = stronger.clone();
                strong.apply(&changes);
                for i in 0..16 {
                    assert_eq!(strong.masks[i] & b.masks[i] & !weak.masks[i], 0);
                }
            }
        }
    }
    #[test]
    fn locked_pair_deductions_preserve_every_independent_pair_completion() {
        let p = Puzzle::new(Shape::new(4).unwrap(), vec![0; 16]).unwrap();
        let budget = Budget::unlimited();
        let mut rng = SplitMix64::new(913);
        let mut b = Board::new(&p, &budget);
        let permutations = permutations(&[15; 4], 4);
        for &(a, c) in &b.topology.pairs {
            let first = b.topology.units[a].clone();
            let second = b.topology.units[c].clone();
            let mut completions = Vec::new();
            for left in &permutations {
                for right in &permutations {
                    let mut assignment = [0u16; 16];
                    for (&i, &v) in first.iter().zip(left) {
                        assignment[i] = 1 << v;
                    }
                    if second
                        .iter()
                        .zip(right)
                        .any(|(&i, &v)| assignment[i] != 0 && assignment[i] != 1 << v)
                    {
                        continue;
                    }
                    for (&i, &v) in second.iter().zip(right) {
                        assignment[i] = 1 << v;
                    }
                    completions.push(assignment);
                }
            }
            for _ in 0..500 {
                for m in &mut b.masks {
                    *m = rng.below(16) as u16;
                }
                let changes = b.pair(a, c).unwrap();
                let mut kept = b.masks.clone();
                for (i, keep) in changes {
                    kept[i] &= keep;
                }
                for solution in &completions {
                    if first
                        .iter()
                        .chain(&second)
                        .all(|&i| b.masks[i] & solution[i] != 0)
                    {
                        for &i in first.iter().chain(&second) {
                            assert_ne!(
                                kept[i] & solution[i],
                                0,
                                "pair {a},{c} discarded a supported value"
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn fixed_size_support_matches_nine_digit_enumeration_and_keeps_budget_charges() {
        let mut rng = SplitMix64::new(812);
        let t = Topology::new(Shape::default());
        for _ in 0..100 {
            let masks: Vec<_> = (0..9)
                .map(|i| {
                    let mut m = 1 << i;
                    for v in 0..9 {
                        if rng.below(3) == 0 {
                            m |= 1 << v;
                        }
                    }
                    m
                })
                .collect();
            let truth = permutations(&masks, 9);
            let expected: Vec<_> = (0..9)
                .map(|i| truth.iter().fold(0, |m, p| m | (1 << p[i])))
                .collect();
            assert_eq!(t.supported(&masks).map(|v| v[..9].to_vec()), Some(expected));
        }
        let p = Puzzle::new(Shape::default(), vec![0; 81]).unwrap();
        let budget = Budget::new(3);
        let b = Board::new(&p, &budget);
        b.unit(0, 2).unwrap();
        b.unit(0, 2).unwrap();
        b.pair(0, 18).unwrap();
        assert_eq!(budget.spent(), 3);
        assert_eq!(b.unit(0, 2), Err(BudgetExceeded));
    }
}
