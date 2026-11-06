#![allow(dead_code)]
// A deliberately independent oracle: plain loops over full assignments,
// sharing no code with the solver or the techniques.
use pencil_puzzles_fillomino::*;

pub fn adjacent(i: usize, j: usize, cols: usize) -> bool {
    (i / cols).abs_diff(j / cols) + (i % cols).abs_diff(j % cols) == 1
}

/// Cells holding the same number as `start` and connected to it.
pub fn group(start: usize, values: &[u32], cols: usize) -> Vec<usize> {
    let mut found = vec![start];
    let mut changed = true;
    while changed {
        changed = false;
        for j in 0..values.len() {
            if values[j] == values[start]
                && !found.contains(&j)
                && found.iter().any(|&i| adjacent(i, j, cols))
            {
                found.push(j);
                changed = true;
            }
        }
    }
    found
}

/// Whether a full assignment keeps R1 (every group's size is its number)
/// and R3 (printed numbers). R2 always holds for full assignments.
pub fn rules(p: &Puzzle, values: &[u32]) -> [bool; 3] {
    let cols = p.shape().cols();
    let r1 = (0..values.len()).all(|i| group(i, values, cols).len() as u32 == values[i]);
    let r3 = p
        .clues()
        .iter()
        .zip(values)
        .all(|(&c, &v)| c == 0 || c == v);
    [r1, true, r3]
}

/// Every assignment of sizes 1 to the cell count, as an odometer.
pub fn assignments(cells: usize) -> Vec<Vec<u32>> {
    let max = cells as u32;
    let mut out = Vec::new();
    let mut v = vec![1u32; cells];
    loop {
        out.push(v.clone());
        let mut k = 0;
        while k < cells && v[k] == max {
            v[k] = 1;
            k += 1;
        }
        if k == cells {
            return out;
        }
        v[k] += 1;
    }
}

pub fn solutions(p: &Puzzle) -> Vec<Vec<u32>> {
    assignments(p.shape().cells())
        .into_iter()
        .filter(|v| rules(p, v).iter().all(|&b| b))
        .collect()
}

pub fn answer(p: &Puzzle, values: &[u32]) -> Answer {
    let mut a = Answer::new(p.shape());
    let cols = p.shape().cols();
    for (i, &v) in values.iter().enumerate() {
        a.set(i / cols, i % cols, Mark::Number(v));
    }
    a
}
