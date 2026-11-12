#![allow(dead_code)]
// A deliberately independent oracle: plain loops over full shadings,
// sharing no code with the solver or the techniques.
use pencil_puzzles_aquarium::*;
use pencil_puzzles_core::SplitMix64;

/// Whether a full shading (`true` = water) keeps R1, R2 and R3.
pub fn rules(p: &Puzzle, water: &[bool]) -> [bool; 3] {
    let (rows, cols) = (p.shape().rows(), p.shape().cols());
    let n = water.len();
    let r1 = (0..rows)
        .all(|r| (0..cols).filter(|&c| water[r * cols + c]).count() as u32 == p.row_clues()[r]);
    let r2 = (0..cols)
        .all(|c| (0..rows).filter(|&r| water[r * cols + c]).count() as u32 == p.col_clues()[c]);
    let mut r3 = true;
    for i in 0..n {
        for j in 0..n {
            if water[i] && !water[j] && p.tanks()[i] == p.tanks()[j] && j / cols >= i / cols {
                r3 = false;
            }
        }
    }
    [r1, r2, r3]
}

/// Every shading of `n` cells.
pub fn shadings(n: usize) -> Vec<Vec<bool>> {
    (0..1u32 << n)
        .map(|b| (0..n).map(|i| b >> i & 1 == 1).collect())
        .collect()
}

pub fn solutions(p: &Puzzle) -> Vec<Vec<bool>> {
    shadings(p.shape().cells())
        .into_iter()
        .filter(|s| rules(p, s).iter().all(|&b| b))
        .collect()
}

pub fn answer(p: &Puzzle, water: &[bool]) -> Answer {
    let mut a = Answer::new(p.shape());
    let cols = p.shape().cols();
    for (i, &w) in water.iter().enumerate() {
        let mark = if w { Mark::Shaded } else { Mark::Unshaded };
        a.set(i / cols, i % cols, mark);
    }
    a
}

/// Random tanks: each cell joins the tank above or to the left, or starts
/// a new one. The clues are random, or count a random shading that keeps
/// R3 so that the puzzle has a solution.
pub fn random_puzzle(rng: &mut SplitMix64, shape: Shape) -> Puzzle {
    let (rows, cols) = (shape.rows(), shape.cols());
    let mut labels: Vec<u32> = Vec::new();
    for i in 0..shape.cells() {
        let mut options = vec![i as u32];
        if i % cols > 0 {
            options.push(labels[i - 1]);
        }
        if i >= cols {
            options.push(labels[i - cols]);
        }
        labels.push(options[rng.below(options.len() as u64) as usize]);
    }
    let mut random = |count: usize, max: usize| -> Vec<u32> {
        (0..count)
            .map(|_| rng.below(max as u64 + 1) as u32)
            .collect()
    };
    let p = Puzzle::new(shape, &labels, random(rows, cols), random(cols, rows)).unwrap();
    if rng.below(4) == 0 {
        return p;
    }
    // Fill each tank up to a random row.
    let line: Vec<usize> = (0..shape.cells())
        .map(|_| rng.below(rows as u64 + 1) as usize)
        .collect();
    let water: Vec<bool> = (0..shape.cells())
        .map(|i| i / cols >= line[p.tanks()[i] as usize])
        .collect();
    p.with_clues_of(&answer(&p, &water))
}
