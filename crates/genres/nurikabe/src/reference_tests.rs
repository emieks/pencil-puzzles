//! Independent enumeration of rule scopes, bounds and canonical branch timelines.
use super::board::*;
use crate::*;
use pencil_puzzles_core::SplitMix64;
mod oracle {
    #![allow(dead_code)]
    // Deliberately independent full-assignment oracle: no production topology or propagation.
    use crate::*;
    pub fn neighbours(i: usize, rows: usize, cols: usize) -> Vec<usize> {
        (0..rows * cols)
            .filter(|&j| (i / cols).abs_diff(j / cols) + (i % cols).abs_diff(j % cols) == 1)
            .collect()
    }
    pub fn component(start: usize, land: &[bool], rows: usize, cols: usize) -> Vec<usize> {
        let color = land[start];
        let mut found = vec![start];
        loop {
            let next: Vec<_> = found
                .iter()
                .flat_map(|&i| neighbours(i, rows, cols))
                .filter(|&j| land[j] == color && !found.contains(&j))
                .collect();
            if next.is_empty() {
                break;
            }
            for i in next {
                if !found.contains(&i) {
                    found.push(i);
                }
            }
        }
        found
    }
    pub fn rules(p: &Puzzle, land: &[bool]) -> [bool; 5] {
        let s = p.shape();
        let mut good = [true; 5];
        for (i, &n) in p.clues().iter().enumerate() {
            if n > 0 {
                good[4] &= land[i];
                good[0] &= land[i] && component(i, land, s.rows(), s.cols()).len() == n as usize;
            }
        }
        let mut seen = vec![false; land.len()];
        let mut seas = 0;
        for i in 0..land.len() {
            if seen[i] {
                continue;
            }
            let c = component(i, land, s.rows(), s.cols());
            for &j in &c {
                seen[j] = true;
            }
            if land[i] {
                good[1] &= c.iter().filter(|&&j| p.clues()[j] > 0).count() == 1;
            } else {
                seas += 1;
            }
        }
        good[2] = seas == 1;
        for r in 0..s.rows().saturating_sub(1) {
            for c in 0..s.cols().saturating_sub(1) {
                let i = r * s.cols() + c;
                good[3] &= [i, i + 1, i + s.cols(), i + s.cols() + 1]
                    .iter()
                    .any(|&j| land[j]);
            }
        }
        good
    }
    pub fn assignments(p: &Puzzle) -> Vec<Vec<bool>> {
        (0..1u64 << p.shape().cells())
            .map(|bits| {
                (0..p.shape().cells())
                    .map(|i| bits & (1 << i) != 0)
                    .collect::<Vec<_>>()
            })
            .filter(|a| p.clues().iter().enumerate().all(|(i, &n)| n == 0 || a[i]))
            .collect()
    }
    pub fn solutions(p: &Puzzle) -> Vec<Vec<bool>> {
        assignments(p)
            .into_iter()
            .filter(|a| rules(p, a).iter().all(|b| *b))
            .collect()
    }
    pub fn answer(p: &Puzzle, land: &[bool]) -> Answer {
        let mut a = Answer::new(p.shape());
        for (i, &v) in land.iter().enumerate() {
            a.set(
                i / p.shape().cols(),
                i % p.shape().cols(),
                if v { Mark::Unshaded } else { Mark::Shaded },
            );
        }
        a
    }
}

/// The solver's propagation never removes a value some solution needs, and
/// never reports a contradiction while a solution remains.
#[test]
fn propagation_is_sound() {
    let mut rng = SplitMix64::new(787);
    for _ in 0..300 {
        let p = Puzzle::new(
            "2x3".parse().unwrap(),
            (0..6)
                .map(|_| {
                    if rng.below(4) == 0 {
                        1 + rng.below(5) as u32
                    } else {
                        0
                    }
                })
                .collect(),
        )
        .unwrap();
        let budget = Budget::unlimited();
        let mut b = Board::new(&p, &budget);
        for i in 0..6 {
            if p.clues()[i] == 0 {
                b.set_mask(i, 1 + rng.below(3) as u16);
            }
        }
        let truth: Vec<_> = oracle::solutions(&p)
            .into_iter()
            .filter(|a| {
                a.iter()
                    .enumerate()
                    .all(|(i, &land)| b.state.mask(i) & (if land { LAND } else { SEA }) != 0)
            })
            .collect();
        let consistent = b.propagate().unwrap();
        assert!(
            consistent || truth.is_empty(),
            "contradiction with a solution left"
        );
        if consistent {
            for a in &truth {
                for (i, &land) in a.iter().enumerate() {
                    assert_ne!(b.state.mask(i) & (if land { LAND } else { SEA }), 0);
                }
            }
        }
    }
}
