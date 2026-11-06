#![allow(dead_code)]
// A deliberately independent oracle: plain loops over full shadings,
// sharing no code with the solver or the techniques.
use pencil_puzzles_hitori::*;

pub fn adjacent(i: usize, j: usize, cols: usize) -> bool {
    (i / cols).abs_diff(j / cols) + (i % cols).abs_diff(j % cols) == 1
}

/// Whether a full shading (`true` = shaded) keeps R1, R2 and R3.
pub fn rules(p: &Puzzle, shaded: &[bool]) -> [bool; 3] {
    let cols = p.shape().cols();
    let n = shaded.len();
    let num = p.numbers();
    let mut r1 = true;
    let mut r2 = true;
    for i in 0..n {
        for j in i + 1..n {
            let line = i / cols == j / cols || i % cols == j % cols;
            if line && !shaded[i] && !shaded[j] && num[i] == num[j] {
                r1 = false;
            }
            if adjacent(i, j, cols) && shaded[i] && shaded[j] {
                r2 = false;
            }
        }
    }
    let white: Vec<usize> = (0..n).filter(|&i| !shaded[i]).collect();
    let mut seen: Vec<usize> = white.first().copied().into_iter().collect();
    let mut changed = true;
    while changed {
        changed = false;
        for &j in &white {
            if !seen.contains(&j) && seen.iter().any(|&i| adjacent(i, j, cols)) {
                seen.push(j);
                changed = true;
            }
        }
    }
    let r3 = !white.is_empty() && seen.len() == white.len();
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

pub fn answer(p: &Puzzle, shaded: &[bool]) -> Answer {
    let mut a = Answer::new(p.shape());
    let cols = p.shape().cols();
    for (i, &s) in shaded.iter().enumerate() {
        let mark = if s { Mark::Shaded } else { Mark::Unshaded };
        a.set(i / cols, i % cols, mark);
    }
    a
}
