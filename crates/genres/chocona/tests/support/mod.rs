#![allow(dead_code)]
// A deliberately independent oracle: plain loops over full shadings,
// sharing no code with the solver or the techniques.
use pencil_puzzles_chocona::*;
use pencil_puzzles_core::SplitMix64;

/// Whether a full shading keeps R1 and R2.
pub fn rules(p: &Puzzle, shaded: &[bool]) -> [bool; 2] {
    let (rows, cols) = (p.shape().rows(), p.shape().cols());
    let mut counts = vec![0; p.region_count()];
    for (i, &t) in p.regions().iter().enumerate() {
        if shaded[i] {
            counts[t as usize] += 1;
        }
    }
    let r1 = (p.clues().iter().zip(&counts)).all(|(clue, &n)| clue.is_none_or(|c| c == n));
    // R2: the cells reachable from each shaded cell through shaded cells
    // fill the rectangle around them.
    let mut r2 = true;
    let mut seen = vec![false; shaded.len()];
    for start in 0..shaded.len() {
        if !shaded[start] || seen[start] {
            continue;
        }
        let mut group = vec![start];
        seen[start] = true;
        let mut k = 0;
        while k < group.len() {
            let i = group[k];
            k += 1;
            let (r, c) = (i / cols, i % cols);
            let mut near = Vec::new();
            if r > 0 {
                near.push(i - cols);
            }
            if r + 1 < rows {
                near.push(i + cols);
            }
            if c > 0 {
                near.push(i - 1);
            }
            if c + 1 < cols {
                near.push(i + 1);
            }
            for j in near {
                if shaded[j] && !seen[j] {
                    seen[j] = true;
                    group.push(j);
                }
            }
        }
        let rs = group.iter().map(|&i| i / cols);
        let cs = group.iter().map(|&i| i % cols);
        let height = rs.clone().max().unwrap() - rs.min().unwrap() + 1;
        let width = cs.clone().max().unwrap() - cs.min().unwrap() + 1;
        if height * width != group.len() {
            r2 = false;
        }
    }
    [r1, r2]
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

/// Random regions: each cell joins the region above or to the left, or
/// starts a new one. Each region has a random clue or none. Usually the
/// clues count a random shading that keeps R2, so that the puzzle has a
/// solution.
pub fn random_puzzle(rng: &mut SplitMix64, shape: Shape) -> Puzzle {
    let cols = shape.cols();
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
    let blank = Puzzle::new(shape, &labels, vec![None; count(&labels)]).unwrap();
    let mut size = vec![0u32; blank.region_count()];
    for &t in blank.regions() {
        size[t as usize] += 1;
    }
    let clues: Vec<Option<u32>> = size
        .iter()
        .map(|&s| (rng.below(4) > 0).then(|| rng.below(u64::from(s) + 1) as u32))
        .collect();
    let p = Puzzle::new(shape, &labels, clues).unwrap();
    if rng.below(4) == 0 {
        return p;
    }
    // A random shading that keeps R2: random cells, then fill the
    // rectangle around each group until every group is a rectangle.
    let n = shape.cells();
    let mut shaded: Vec<bool> = (0..n).map(|_| rng.below(3) == 0).collect();
    while !rules(&p, &shaded)[1] {
        let (rows, cols) = (shape.rows(), shape.cols());
        for r in 0..rows - 1 {
            for c in 0..cols - 1 {
                let i = r * cols + c;
                let block = [i, i + 1, i + cols, i + cols + 1];
                if block.iter().filter(|&&j| shaded[j]).count() == 3 {
                    for j in block {
                        shaded[j] = true;
                    }
                }
            }
        }
    }
    p.with_clues_of(&answer(&p, &shaded))
}

fn count(labels: &[u32]) -> usize {
    let mut l = labels.to_vec();
    l.sort_unstable();
    l.dedup();
    l.len()
}
