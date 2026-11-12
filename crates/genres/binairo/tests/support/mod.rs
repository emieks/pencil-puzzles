#![allow(dead_code)]
// A deliberately independent oracle: plain loops over full grids, sharing
// no code with the solver or the techniques.
use pencil_puzzles_binairo::*;
use pencil_puzzles_core::SplitMix64;

/// The digits of a grid's line `k`: rows first, then columns.
fn line(rows: usize, cols: usize, cells: &[u8], k: usize) -> Vec<u8> {
    if k < rows {
        cells[k * cols..(k + 1) * cols].to_vec()
    } else {
        (0..rows).map(|r| cells[r * cols + k - rows]).collect()
    }
}

/// Whether a full grid keeps R1, R2, R3 and the givens (R4).
pub fn rules(p: &Puzzle, cells: &[u8]) -> [bool; 4] {
    let (rows, cols) = (p.shape().rows(), p.shape().cols());
    let lines: Vec<Vec<u8>> = (0..rows + cols)
        .map(|k| line(rows, cols, cells, k))
        .collect();
    let r1 = lines
        .iter()
        .all(|l| l.windows(3).all(|w| !(w[0] == w[1] && w[1] == w[2])));
    let r2 = lines
        .iter()
        .all(|l| 2 * l.iter().filter(|&&d| d == 1).count() == l.len());
    let mut r3 = true;
    for a in 0..lines.len() {
        for b in a + 1..lines.len() {
            if (a < rows) == (b < rows) && lines[a] == lines[b] {
                r3 = false;
            }
        }
    }
    let r4 = (0..cells.len()).all(|i| p.givens()[i].digit().is_none_or(|g| g == cells[i]));
    [r1, r2, r3, r4]
}

/// Every grid of the shape whose rows keep R1 and R2.
pub fn grids(shape: Shape) -> Vec<Vec<u8>> {
    let cols = shape.cols();
    let rows: Vec<Vec<u8>> = (0..1u32 << cols)
        .map(|b| (0..cols).map(|c| (b >> c & 1) as u8).collect::<Vec<u8>>())
        .filter(|r| {
            2 * r.iter().filter(|&&d| d == 1).count() == cols
                && r.windows(3).all(|w| !(w[0] == w[1] && w[1] == w[2]))
        })
        .collect();
    let mut out = vec![Vec::new()];
    for _ in 0..shape.rows() {
        out = out
            .into_iter()
            .flat_map(|g| {
                rows.iter().map(move |r| {
                    let mut next = g.clone();
                    next.extend(r);
                    next
                })
            })
            .collect();
    }
    out
}

pub fn solutions(p: &Puzzle, all: &[Vec<u8>]) -> Vec<Vec<u8>> {
    all.iter()
        .filter(|g| rules(p, g).iter().all(|&b| b))
        .cloned()
        .collect()
}

pub fn answer(shape: Shape, cells: &[u8]) -> Answer {
    let mut a = Answer::new(shape);
    for (i, &d) in cells.iter().enumerate() {
        a.set(i / shape.cols(), i % shape.cols(), Mark::of(d));
    }
    a
}

/// Random givens: usually some cells of a grid from `all` (which may break
/// R1-R3 in its columns), otherwise random digits.
pub fn random_puzzle(rng: &mut SplitMix64, shape: Shape, all: &[Vec<u8>]) -> Puzzle {
    let n = shape.cells();
    let grid = &all[rng.below(all.len() as u64) as usize];
    let keep = rng.below(4) + 1;
    let random = rng.below(5) == 0;
    let givens = (0..n)
        .map(|i| {
            if rng.below(6) >= keep {
                Mark::Unmarked
            } else if random {
                Mark::of(rng.below(2) as u8)
            } else {
                Mark::of(grid[i])
            }
        })
        .collect();
    Puzzle::new(shape, givens).unwrap()
}
