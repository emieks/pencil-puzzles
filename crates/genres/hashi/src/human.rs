//! Hashi's human techniques, plugged into the shared solve loop
//! (`pencil_puzzles_core::human`).
//!
//! Variables are slots (see [`Layout`]): pairs of islands that see each
//! other. Candidate bit `v` stands for `v` bridges, from 0 to 2. A slot is
//! fixed when one candidate is left.
use crate::board::Layout;
use pencil_puzzles_core::human::{Candidates, Find, HumanStep, Kind, Profile, Technique, Trace};
use pencil_puzzles_core::{Budget, BudgetExceeded, Difficulty};

/// Longest Hard contradiction chain, using Easy and Medium techniques.
const SHORT_CHAIN: usize = 6;
/// Longest Expert contradiction chain, using techniques up to Hard.
const LONG_CHAIN: usize = 20;

fn fixed(mask: u16) -> bool {
    mask.count_ones() == 1
}

fn value(mask: u16) -> u8 {
    mask.trailing_zeros() as u8
}

fn bits(mask: u16) -> impl Iterator<Item = u8> {
    (0..3).filter(move |v| mask & (1 << v) != 0)
}

/// Whether some choice of candidates for `slots` adds up to `n`.
fn reachable(masks: &[u16], slots: &[usize], n: u8) -> bool {
    let mut sums: u16 = 1;
    for &s in slots {
        let mut next = 0;
        for v in bits(masks[s]) {
            next |= sums << v;
        }
        sums = next;
    }
    sums & (1 << n) != 0
}

/// Whether the candidates break a rule: an island can't reach its number,
/// two crossing slots both need bridges, or the slots that may hold bridges
/// can't join every island. Exact when every slot is fixed.
pub(crate) fn broken(l: &Layout, masks: &[u16]) -> bool {
    if masks.contains(&0) {
        return true;
    }
    if (0..l.cells.len()).any(|k| !reachable(masks, &l.at[k], l.numbers[k])) {
        return true;
    }
    let needs = |s: usize| masks[s] & 1 == 0;
    if (0..masks.len()).any(|s| needs(s) && l.crosses[s].iter().any(|&c| needs(c))) {
        return true;
    }
    l.groups(|s| masks[s] != 1) > 1
}

/// The bridges already fixed at island `k`.
fn fixed_sum(l: &Layout, masks: &[u16], k: usize) -> u8 {
    l.at[k]
        .iter()
        .filter(|&&s| fixed(masks[s]))
        .map(|&s| value(masks[s]))
        .sum()
}

/// Bookkeeping: an island whose fixed bridges reach its number gets no more.
fn island_complete(c: &Candidates<'_, Layout>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let l = c.context;
    let mut steps = Vec::new();
    for k in 0..l.cells.len() {
        c.charge()?;
        if fixed_sum(l, c.masks, k) != l.numbers[k] {
            continue;
        }
        let open: Vec<_> = l.at[k]
            .iter()
            .filter(|&&s| !fixed(c.masks[s]))
            .map(|&s| (s, 1))
            .collect();
        steps.extend(c.step("island_complete", l.at[k].clone(), vec![k], open));
    }
    Ok(steps)
}

/// Bookkeeping: a slot that needs a bridge blocks the slots it crosses.
fn crossing(c: &Candidates<'_, Layout>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let l = c.context;
    let mut steps = Vec::new();
    for s in 0..c.masks.len() {
        c.charge()?;
        if c.masks[s] & 1 != 0 || l.crosses[s].is_empty() {
            continue;
        }
        let changes = l.crosses[s].iter().map(|&x| (x, 1)).collect();
        steps.extend(c.step("crossing", vec![s], Vec::new(), changes));
    }
    Ok(steps)
}

/// Keep only the counts that appear in some way to reach the island's
/// number with the slots' candidates.
fn island_bounds(c: &Candidates<'_, Layout>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let l = c.context;
    let mut steps = Vec::new();
    for k in 0..l.cells.len() {
        let slots = &l.at[k];
        let n = l.numbers[k];
        let mut keep = vec![0u16; slots.len()];
        // At most four slots of three counts each: 81 combinations.
        let mut choice = vec![0u8; slots.len()];
        'combos: loop {
            c.charge()?;
            let fits = slots
                .iter()
                .zip(&choice)
                .all(|(&s, &v)| c.masks[s] & (1 << v) != 0);
            if fits && choice.iter().sum::<u8>() == n {
                for (j, &v) in choice.iter().enumerate() {
                    keep[j] |= 1 << v;
                }
            }
            for v in choice.iter_mut() {
                if *v < 2 {
                    *v += 1;
                    continue 'combos;
                }
                *v = 0;
            }
            break;
        }
        if keep.contains(&0) {
            // No way at all; the broken check reports it.
            continue;
        }
        let changes = slots.iter().copied().zip(keep).collect();
        steps.extend(c.step("island_bounds", slots.clone(), vec![k], changes));
    }
    Ok(steps)
}

/// Two islands that one slot would finish together, such as two 1s or two
/// 2s with two bridges, would be cut off from the rest; so that count goes,
/// unless they are the only islands.
fn isolated_pair(c: &Candidates<'_, Layout>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let l = c.context;
    let mut steps = Vec::new();
    if l.cells.len() <= 2 {
        return Ok(steps);
    }
    for (s, slot) in l.slots.iter().enumerate() {
        c.charge()?;
        let [a, b] = slot.ends;
        let n = l.numbers[a];
        if n == l.numbers[b] && n <= 2 && c.masks[s] & (1 << n) != 0 {
            steps.extend(c.step("isolated_pair", vec![s], vec![a, b], vec![(s, !(1 << n))]));
        }
    }
    Ok(steps)
}

/// A slot that is the only remaining connection between two parts of the
/// puzzle needs at least one bridge: a bridge (cut edge) of the graph of
/// slots that may hold bridges.
fn only_connection(c: &Candidates<'_, Layout>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let l = c.context;
    let open = |s: usize| c.masks[s] != 1;
    // Slots ruled out are the premises of every step.
    let closed: Vec<usize> = (0..c.masks.len()).filter(|&s| !open(s)).collect();
    let mut steps = Vec::new();
    for s in cut_slots(l, open, c.budget)? {
        if c.masks[s] & 1 != 0 {
            let mut support = closed.clone();
            support.push(s);
            steps.extend(c.step("only_connection", support, Vec::new(), vec![(s, !1)]));
        }
    }
    Ok(steps)
}

/// The slots whose removal would split the islands they join, in slot
/// order: bridges of the graph, by an iterative depth-first search.
fn cut_slots(
    l: &Layout,
    open: impl Fn(usize) -> bool,
    budget: &Budget,
) -> Result<Vec<usize>, BudgetExceeded> {
    let n = l.cells.len();
    if !budget.spend((n + l.slots.len()) as u64 + 1) {
        return Err(BudgetExceeded);
    }
    let mut order = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut cuts = Vec::new();
    let mut time = 0;
    for root in 0..n {
        if order[root] != usize::MAX {
            continue;
        }
        // Each frame: island, slot used to reach it, next slot index to try.
        let mut stack = vec![(root, usize::MAX, 0usize)];
        order[root] = time;
        low[root] = time;
        time += 1;
        while let Some(&mut (k, via, ref mut next)) = stack.last_mut() {
            if let Some(&s) = l.at[k].get(*next) {
                *next += 1;
                if s == via || !open(s) {
                    continue;
                }
                let [a, b] = l.slots[s].ends;
                let other = if a == k { b } else { a };
                if order[other] == usize::MAX {
                    order[other] = time;
                    low[other] = time;
                    time += 1;
                    stack.push((other, s, 0));
                } else {
                    low[k] = low[k].min(order[other]);
                }
            } else {
                stack.pop();
                if let Some(&(parent, _, _)) = stack.last() {
                    low[parent] = low[parent].min(low[k]);
                    if low[k] > order[parent] {
                        cuts.push(via);
                    }
                }
            }
        }
    }
    cuts.sort_unstable();
    Ok(cuts)
}

/// Apply the bookkeeping rules to a fixpoint, starting from slot `s`, and
/// return every slot read or changed.
fn settle(l: &Layout, masks: &mut [u16], s: usize) -> Vec<usize> {
    let mut touched = vec![s];
    let mut queue = vec![s];
    while let Some(s) = queue.pop() {
        if masks[s] == 0 {
            break;
        }
        let mut changed = Vec::new();
        if masks[s] & 1 == 0 {
            for &x in &l.crosses[s] {
                if masks[x] != masks[x] & 1 {
                    masks[x] &= 1;
                    changed.push(x);
                }
            }
        }
        if fixed(masks[s]) {
            for k in l.slots[s].ends {
                if fixed_sum(l, masks, k) == l.numbers[k] {
                    for &x in &l.at[k] {
                        if !fixed(masks[x]) {
                            masks[x] &= 1;
                            changed.push(x);
                        }
                    }
                    touched.extend(&l.at[k]);
                }
            }
        }
        touched.extend(&changed);
        queue.extend(changed);
    }
    touched.sort_unstable();
    touched.dedup();
    touched
}

/// A count that breaks a rule as soon as it is written in, after its
/// bookkeeping, such as a bridge that closes off a group of finished
/// islands. Every refuted count of a slot goes in one step.
fn forced(c: &Candidates<'_, Layout>) -> Result<Vec<HumanStep>, BudgetExceeded> {
    let l = c.context;
    let mut steps = Vec::new();
    for s in 0..c.masks.len() {
        if fixed(c.masks[s]) {
            continue;
        }
        let mut removed = 0;
        let mut support = vec![s];
        for v in bits(c.masks[s]) {
            c.charge()?;
            let mut masks = c.masks.to_vec();
            masks[s] = 1 << v;
            let touched = settle(l, &mut masks, s);
            if !c.budget.spend((l.cells.len() + masks.len()) as u64) {
                return Err(BudgetExceeded);
            }
            if broken(l, &masks) {
                removed |= 1 << v;
                support.extend(touched);
            }
        }
        if removed != 0 && removed != c.masks[s] {
            steps.extend(c.step("forced", support, Vec::new(), vec![(s, !removed)]));
        }
    }
    Ok(steps)
}

const fn entry(
    id: &'static str,
    level: Difficulty,
    kind: Kind,
    probes: bool,
    find: Find<Layout>,
) -> Technique<Layout> {
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

pub(crate) static TECHNIQUES: [Technique<Layout>; 8] = [
    entry("island_complete", Easy, Bookkeeping, false, island_complete),
    entry("crossing", Easy, Bookkeeping, false, crossing),
    entry("island_bounds", Easy, Skill, false, island_bounds),
    entry("isolated_pair", Easy, Skill, false, isolated_pair),
    entry("only_connection", Medium, Skill, false, only_connection),
    entry("forced", Medium, Skill, false, forced),
    entry("short_contradiction", Hard, Skill, true, |c| {
        PROFILE.contradictions(c, "short_contradiction", Medium, SHORT_CHAIN)
    }),
    entry("contradiction", Expert, Skill, true, |c| {
        PROFILE.contradictions(c, "contradiction", Hard, LONG_CHAIN)
    }),
];

/// The canonical Hashi human profile.
pub(crate) static PROFILE: Profile<Layout> = Profile {
    id: "hashi-human",
    version: 1,
    techniques: &TECHNIQUES,
    broken,
};

/// Every slot starts with every count up to its cap.
pub(crate) fn initial_masks(l: &Layout) -> Vec<u16> {
    l.slots.iter().map(|s| (1 << (s.cap + 1)) - 1).collect()
}

/// Solve from `masks` with techniques up to `cap`.
pub(crate) fn analyze_state(
    l: &Layout,
    masks: Vec<u16>,
    budget: &Budget,
    cap: Difficulty,
    metrics: bool,
) -> Result<Trace, BudgetExceeded> {
    let placed = vec![false; masks.len()];
    PROFILE.analyze(l, masks, placed, budget, cap, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Answer, Puzzle, Shape};
    use pencil_puzzles_core::SplitMix64;
    use std::collections::BTreeSet;

    /// Every assignment of 0 to 2 bridges to `n` slots.
    fn assignments(n: usize) -> Vec<Vec<u8>> {
        let mut out = vec![Vec::new()];
        for _ in 0..n {
            out = out
                .into_iter()
                .flat_map(|v| {
                    (0..3).map(move |x| {
                        let mut w = v.clone();
                        w.push(x);
                        w
                    })
                })
                .collect();
        }
        out
    }

    /// Whether bridges `values` solve the puzzle, by the checker (itself
    /// tested against an independent oracle).
    fn solves(p: &Puzzle, l: &Layout, values: &[u8]) -> bool {
        let a = Answer::from_counts(p, l, values, true);
        crate::check(p, &a, &Budget::unlimited())
            .unwrap()
            .is_empty()
    }

    /// Random islands with numbers from a random set of bridges.
    fn puzzle(rng: &mut SplitMix64, shape: Shape) -> Puzzle {
        loop {
            let cols = shape.cols();
            let mut clues = vec![0u8; shape.cells()];
            for i in 0..clues.len() {
                let left = i % cols > 0 && clues[i - 1] > 0;
                let up = i >= cols && clues[i - cols] > 0;
                let right = i % cols + 1 < cols && clues[i + 1] > 0;
                let down = i + cols < clues.len() && clues[i + cols] > 0;
                if !left && !up && !right && !down && rng.below(3) == 0 {
                    clues[i] = 8;
                }
            }
            let p = Puzzle::new(shape, clues).unwrap();
            let l = Layout::new(&p);
            let values: Vec<u8> = l.slots.iter().map(|_| rng.below(3) as u8).collect();
            let mut clues = vec![0u8; shape.cells()];
            for (slot, &v) in l.slots.iter().zip(&values) {
                for k in slot.ends {
                    clues[l.cells[k]] += v;
                }
            }
            let crossing = (0..values.len())
                .any(|s| values[s] > 0 && l.crosses[s].iter().any(|&x| values[x] > 0));
            if !crossing && l.cells.iter().all(|&i| clues[i] > 0) && l.slots.len() <= 9 {
                return Puzzle::new(shape, clues).unwrap();
            }
        }
    }

    /// A slot that needs a bridge blocks the slot it crosses. Small random
    /// puzzles rarely have crossing slots, so this is checked directly.
    #[test]
    fn crossing_blocks_the_crossed_slot() {
        let p: Puzzle = "grid:
.2.
2.2
.2."
        .parse()
        .unwrap();
        let l = Layout::new(&p);
        assert_eq!(l.crosses, [vec![1], vec![0]]);
        let masks = vec![0b110, 0b111];
        let placed = vec![false; 2];
        let budget = Budget::unlimited();
        let steps = crossing(&Candidates::new(&l, &masks, &placed, &budget)).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].changes, [(1, 1)]);
    }

    /// `broken` is exact on every full assignment.
    #[test]
    fn broken_is_exact_on_full_assignments() {
        let mut rng = SplitMix64::new(5);
        for round in 0..60 {
            let shape: Shape = ["3x3", "3x4", "4x4"][round % 3].parse().unwrap();
            let p = puzzle(&mut rng, shape);
            let l = Layout::new(&p);
            for values in assignments(l.slots.len()) {
                let masks: Vec<u16> = values.iter().map(|&v| 1 << v).collect();
                assert_eq!(
                    broken(&l, &masks),
                    !solves(&p, &l, &values),
                    "{} {values:?}",
                    p.id()
                );
            }
        }
    }

    /// Every technique keeps every solution of random partial states,
    /// `broken` never fires while a solution remains, and the checker
    /// reports a partial answer whenever `broken` does (L23, P8).
    #[test]
    fn techniques_keep_every_solution() {
        let mut rng = SplitMix64::new(17);
        let budget = Budget::unlimited();
        let mut fired = BTreeSet::new();
        for round in 0..1500 {
            let shape: Shape = ["3x3", "3x4", "4x3", "4x4", "2x5"][round % 5]
                .parse()
                .unwrap();
            let p = puzzle(&mut rng, shape);
            let l = Layout::new(&p);
            let all: Vec<Vec<u8>> = assignments(l.slots.len())
                .into_iter()
                .filter(|v| solves(&p, &l, v))
                .collect();
            if all.is_empty() {
                continue;
            }
            let truth = &all[rng.below(all.len() as u64) as usize];
            // Partial candidates: the truth plus random other counts.
            let masks: Vec<u16> = initial_masks(&l)
                .into_iter()
                .zip(truth)
                .map(|(m, &v)| {
                    if rng.below(3) == 0 {
                        m
                    } else {
                        (m & rng.next_u64() as u16) | 1 << v
                    }
                })
                .collect();
            let fits: Vec<&Vec<u8>> = all
                .iter()
                .filter(|f| (0..masks.len()).all(|s| masks[s] & (1 << f[s]) != 0))
                .collect();
            assert!(!broken(&l, &masks), "broken although a solution fits");
            let placed = vec![false; masks.len()];
            let candidates = Candidates::new(&l, &masks, &placed, &budget);
            for technique in &TECHNIQUES {
                for step in technique.run(&candidates).unwrap() {
                    fired.insert(step.technique);
                    for &(s, keep) in &step.changes {
                        for f in &fits {
                            assert_ne!(
                                keep & (1 << f[s]),
                                0,
                                "{} removed a true count from {} with {masks:?}",
                                step.technique,
                                p.id()
                            );
                        }
                    }
                }
            }
            // A partial answer: some true bridges drawn, some wrong ones.
            let drawn: Vec<u8> = truth
                .iter()
                .map(|&v| match rng.below(4) {
                    0 => v,
                    1 => rng.below(3) as u8,
                    _ => 0,
                })
                .collect();
            let answer_masks: Vec<u16> = initial_masks(&l)
                .into_iter()
                .zip(&drawn)
                .map(|(m, &d)| if d > 0 { 1 << d } else { m })
                .collect();
            if broken(&l, &answer_masks) {
                let a = Answer::from_counts(&p, &l, &drawn, false);
                assert!(
                    !crate::check(&p, &a, &budget).unwrap().is_empty(),
                    "{} {a}",
                    p.id()
                );
            }
        }
        for technique in [
            "island_complete",
            "island_bounds",
            "isolated_pair",
            "only_connection",
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
