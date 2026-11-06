#![allow(dead_code)]
// A deliberately independent oracle: its own scan for slots and plain
// loops over full assignments, sharing no code with the solver or the
// techniques.
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_hashi::*;

/// Two islands that see each other, as their cells and the cells between.
pub struct Slot {
    pub a: usize,
    pub b: usize,
    pub horizontal: bool,
    pub between: Vec<usize>,
}

pub fn slots(p: &Puzzle) -> Vec<Slot> {
    let (rows, cols) = (p.shape().rows(), p.shape().cols());
    let clues = p.clues();
    let mut out = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            if clues[r * cols + c] == 0 {
                continue;
            }
            if let Some(c2) = (c + 1..cols).find(|&c2| clues[r * cols + c2] > 0) {
                out.push(Slot {
                    a: r * cols + c,
                    b: r * cols + c2,
                    horizontal: true,
                    between: (c + 1..c2).map(|x| r * cols + x).collect(),
                });
            }
            if let Some(r2) = (r + 1..rows).find(|&r2| clues[r2 * cols + c] > 0) {
                out.push(Slot {
                    a: r * cols + c,
                    b: r2 * cols + c,
                    horizontal: false,
                    between: (r + 1..r2).map(|y| y * cols + c).collect(),
                });
            }
        }
    }
    out
}

/// Which of R1 (numbers), R2 (no crossings) and R3 (connected) a full
/// assignment of bridges to `slots` keeps.
pub fn rules(p: &Puzzle, slots: &[Slot], values: &[u8]) -> [bool; 3] {
    let clues = p.clues();
    let r1 = (0..clues.len()).filter(|&i| clues[i] > 0).all(|i| {
        let total: u8 = slots
            .iter()
            .zip(values)
            .filter(|(s, _)| s.a == i || s.b == i)
            .map(|(_, &v)| v)
            .sum();
        total == clues[i]
    });
    let r2 = slots.iter().zip(values).all(|(s, &v)| {
        v == 0
            || slots.iter().zip(values).all(|(t, &w)| {
                w == 0
                    || s.horizontal == t.horizontal
                    || !s.between.iter().any(|c| t.between.contains(c))
            })
    });
    let islands: Vec<usize> = (0..clues.len()).filter(|&i| clues[i] > 0).collect();
    let mut reached = vec![islands.first().copied().unwrap_or(0)];
    let mut changed = true;
    while changed {
        changed = false;
        for (s, &v) in slots.iter().zip(values) {
            if v == 0 {
                continue;
            }
            for (x, y) in [(s.a, s.b), (s.b, s.a)] {
                if reached.contains(&x) && !reached.contains(&y) {
                    reached.push(y);
                    changed = true;
                }
            }
        }
    }
    let r3 = islands.iter().all(|i| reached.contains(i));
    [r1, r2, r3]
}

/// Every assignment of 0 to 2 bridges to `n` slots, as an odometer.
pub fn assignments(n: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut v = vec![0u8; n];
    loop {
        out.push(v.clone());
        let mut k = 0;
        while k < n && v[k] == 2 {
            v[k] = 0;
            k += 1;
        }
        if k == n {
            return out;
        }
        v[k] += 1;
    }
}

pub fn solutions(p: &Puzzle, slots: &[Slot]) -> Vec<Vec<u8>> {
    assignments(slots.len())
        .into_iter()
        .filter(|v| rules(p, slots, v).iter().all(|&b| b))
        .collect()
}

/// The answer drawing `values` on `slots`, not yet finished.
pub fn answer(p: &Puzzle, slots: &[Slot], values: &[u8]) -> Answer {
    let cols = p.shape().cols();
    let mut a = Answer::new(p.shape());
    for (i, &n) in p.clues().iter().enumerate() {
        if n > 0 {
            a.set(i / cols, i % cols, Cell::Island(n));
        }
    }
    for (s, &v) in slots.iter().zip(values) {
        for &i in &s.between {
            let Cell::Bridges { across, down } = a.get(i / cols, i % cols) else {
                unreachable!()
            };
            let cell = if s.horizontal {
                Cell::Bridges { across: v, down }
            } else {
                Cell::Bridges { across, down: v }
            };
            a.set(i / cols, i % cols, cell);
        }
    }
    a
}

/// Random islands, never adjacent, with numbers from 1 to `max`.
pub fn random_puzzle(rng: &mut SplitMix64, shape: Shape, density: u64, max: u8) -> Puzzle {
    let cols = shape.cols();
    let mut clues = vec![0u8; shape.cells()];
    for i in 0..clues.len() {
        let left = i % cols > 0 && clues[i - 1] > 0;
        let up = i >= cols && clues[i - cols] > 0;
        if !left && !up && rng.below(density) == 0 {
            clues[i] = 1 + rng.below(u64::from(max)) as u8;
        }
    }
    Puzzle::new(shape, clues).unwrap()
}

/// Random islands with numbers from a random network of bridges, so that
/// some solution exists.
pub fn solvable_puzzle(rng: &mut SplitMix64, shape: Shape, density: u64) -> Puzzle {
    loop {
        let layout = random_puzzle(rng, shape, density, 1);
        let s = slots(&layout);
        let values: Vec<u8> = s.iter().map(|_| rng.below(3) as u8).collect();
        let mut clues = vec![0u8; shape.cells()];
        for (slot, &v) in s.iter().zip(&values) {
            clues[slot.a] += v;
            clues[slot.b] += v;
        }
        let islands_match = (0..clues.len()).all(|i| (clues[i] > 0) == (layout.clues()[i] > 0));
        if islands_match && rules(&layout, &s, &values)[1] {
            return Puzzle::new(shape, clues).unwrap();
        }
    }
}
