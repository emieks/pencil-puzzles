#![allow(dead_code)]
// A deliberately independent oracle: plain loops over full assignments,
// sharing no code with the solver or the techniques.
use pencil_puzzles_kakuro::*;

/// The white cells of a puzzle, row-major.
pub fn whites(p: &Puzzle) -> Vec<usize> {
    (0..p.cells().len())
        .filter(|&i| p.cells()[i] == Cell::White)
        .collect()
}

/// Each run as its sum and cells, found by walking from every block.
pub fn runs(p: &Puzzle) -> Vec<(usize, u32, Vec<usize>)> {
    let cols = p.shape().cols();
    let n = p.cells().len();
    let mut out = Vec::new();
    for (i, &c) in p.cells().iter().enumerate() {
        let Cell::Block { down, across } = c else {
            continue;
        };
        for (sum, step) in [(down, cols), (across, 1)] {
            let Some(sum) = sum else { continue };
            let mut cells = Vec::new();
            let mut j = i + step;
            while j < n && p.cells()[j] == Cell::White && (step == cols || j / cols == i / cols) {
                cells.push(j);
                j += step;
            }
            out.push((i, u32::from(sum), cells));
        }
    }
    out
}

/// Whether every run of a full assignment (digits by cell, 0 on blocks)
/// has different digits adding up to its sum: R1. R2 holds for full
/// assignments.
pub fn keeps_runs(p: &Puzzle, digits: &[u8]) -> bool {
    runs(p)
        .iter()
        .all(|(_, sum, cells)| run_ok(digits, *sum, cells))
}

pub fn run_ok(digits: &[u8], sum: u32, cells: &[usize]) -> bool {
    let total: u32 = cells.iter().map(|&j| u32::from(digits[j])).sum();
    let distinct = (0..cells.len()).all(|a| (0..a).all(|b| digits[cells[a]] != digits[cells[b]]));
    total == sum && distinct
}

/// Every assignment of digits 1 to 9 to the given cells, as an odometer,
/// with 0 elsewhere.
pub fn assignments(n: usize, cells: &[usize]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut v = vec![1u8; cells.len()];
    loop {
        let mut digits = vec![0u8; n];
        for (k, &i) in cells.iter().enumerate() {
            digits[i] = v[k];
        }
        out.push(digits);
        let mut k = 0;
        while k < v.len() && v[k] == 9 {
            v[k] = 1;
            k += 1;
        }
        if k == v.len() {
            return out;
        }
        v[k] += 1;
    }
}

pub fn solutions(p: &Puzzle) -> Vec<Vec<u8>> {
    assignments(p.cells().len(), &whites(p))
        .into_iter()
        .filter(|d| keeps_runs(p, d))
        .collect()
}

pub fn answer(p: &Puzzle, digits: &[u8]) -> Answer {
    let mut a = Answer::new(p.shape());
    let cols = p.shape().cols();
    for i in whites(p) {
        a.set(i / cols, i % cols, Mark::Digit(digits[i]));
    }
    a
}

/// A puzzle with the given white cells and the sums of `digits`.
pub fn puzzle(shape: Shape, white: &[bool], digits: &[u8]) -> Puzzle {
    let cols = shape.cols();
    let n = white.len();
    let sum = |i: usize, step: usize| -> Option<u8> {
        let mut j = i + step;
        let mut s = 0;
        let mut any = false;
        while j < n && white[j] && (step == cols || j / cols == i / cols) {
            s += digits[j];
            any = true;
            j += step;
        }
        any.then_some(s)
    };
    let cells = (0..n)
        .map(|i| {
            if white[i] {
                Cell::White
            } else {
                Cell::Block {
                    down: sum(i, cols),
                    across: sum(i, 1),
                }
            }
        })
        .collect();
    Puzzle::new(shape, cells).unwrap()
}

/// White cells from row 1 and column 1 on, except `blocks`.
pub fn layout(shape: &str, blocks: &[(usize, usize)]) -> (Shape, Vec<bool>) {
    let s: Shape = shape.parse().unwrap();
    let white = (0..s.cells())
        .map(|i| {
            let (r, c) = (i / s.cols(), i % s.cols());
            r > 0 && c > 0 && !blocks.contains(&(r, c))
        })
        .collect();
    (s, white)
}
