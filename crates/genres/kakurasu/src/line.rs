//! Deductions within a single row or column.
//!
//! A line is a slice of cells whose values are 1, 2, 3, ... from the start,
//! together with a target: the total the shaded cells must reach.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cell {
    Unknown,
    Shaded,
    Empty,
}

/// The line cannot reach its target with the cells as they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Contradiction;

/// A line deduction: takes a line and its target and fills in any cells it
/// can work out, in place.
pub(crate) type LineSolver = fn(&mut [Cell], u32) -> Result<(), Contradiction>;

/// Complete reasoning about one line: considers every way the unknown cells
/// can reach the target and fills in each cell that is the same in all of
/// them.
///
/// Runs a subset-sum search from both ends of the line, keeping sets of
/// totals as bits. `before[k]` holds the totals the first `k` cells can
/// make. `needed[k]` holds the totals that the first `k` cells could have
/// made for the cells from `k` onwards to finish the job, that is, `target`
/// minus what the cells from `k` onwards can make. A cell can be empty if
/// some total is in both `before[k]` and `needed[k + 1]`, and shaded if that
/// holds after adding the cell's value.
pub(crate) fn solve_full(line: &mut [Cell], target: u32) -> Result<(), Contradiction> {
    let len = line.len();
    let target = target as usize;
    let words = target / 64 + 1;
    let row = |k: usize| k * words..(k + 1) * words;

    let mut bits = vec![0u64; (2 * (len + 1) + 1) * words];
    let (before, rest) = bits.split_at_mut((len + 1) * words);
    let (needed, shifted) = rest.split_at_mut((len + 1) * words);

    before[0] = 1;
    for (k, &cell) in line.iter().enumerate() {
        let (done, rest) = before.split_at_mut(row(k + 1).start);
        let (from, to) = (&done[row(k)], &mut rest[..words]);
        if cell != Cell::Shaded {
            or_into(to, from);
        }
        if cell != Cell::Empty {
            or_shifted_up(to, from, k + 1);
        }
    }
    if !has_bit(&before[row(len)], target) {
        return Err(Contradiction);
    }

    needed[row(len)][target / 64] = 1 << (target % 64);
    for (k, &cell) in line.iter().enumerate().rev() {
        let (rest, done) = needed.split_at_mut(row(k + 1).start);
        let (from, to) = (&done[..words], &mut rest[row(k)]);
        if cell != Cell::Shaded {
            or_into(to, from);
        }
        if cell != Cell::Empty {
            or_shifted_down(to, from, k + 1);
        }
    }

    for (k, cell) in line.iter_mut().enumerate() {
        if *cell != Cell::Unknown {
            continue;
        }
        let (before, needed) = (&before[row(k)], &needed[row(k + 1)]);
        shifted.fill(0);
        or_shifted_up(shifted, before, k + 1);
        *cell = match (intersects(shifted, needed), intersects(before, needed)) {
            (true, true) => Cell::Unknown,
            (true, false) => Cell::Shaded,
            (false, true) => Cell::Empty,
            (false, false) => return Err(Contradiction),
        };
    }
    Ok(())
}

fn has_bit(bits: &[u64], index: usize) -> bool {
    (bits[index / 64] >> (index % 64)) & 1 == 1
}

fn intersects(a: &[u64], b: &[u64]) -> bool {
    a.iter().zip(b).any(|(a, b)| a & b != 0)
}

fn or_into(to: &mut [u64], from: &[u64]) {
    for (to, from) in to.iter_mut().zip(from) {
        *to |= from;
    }
}

/// `to |= from << by`, dropping bits past the end.
fn or_shifted_up(to: &mut [u64], from: &[u64], by: usize) {
    let (word_shift, bit_shift) = (by / 64, by % 64);
    for (j, to) in to.iter_mut().skip(word_shift).enumerate() {
        let mut value = from[j] << bit_shift;
        if bit_shift > 0 && j > 0 {
            value |= from[j - 1] >> (64 - bit_shift);
        }
        *to |= value;
    }
}

/// `to |= from >> by`.
fn or_shifted_down(to: &mut [u64], from: &[u64], by: usize) {
    let (word_shift, bit_shift) = (by / 64, by % 64);
    for i in 0..to.len().saturating_sub(word_shift) {
        let j = i + word_shift;
        let mut value = from[j] >> bit_shift;
        if bit_shift > 0 && j + 1 < from.len() {
            value |= from[j + 1] << (64 - bit_shift);
        }
        to[i] |= value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pencil_puzzles_core::SplitMix64;

    /// Runs an in-place line solver on a copy of `line`.
    fn run(solver: LineSolver, line: &[Cell], target: u32) -> Result<Vec<Cell>, Contradiction> {
        let mut line = line.to_vec();
        solver(&mut line, target).map(|()| line)
    }

    /// Every way to fill in the unknown cells so the line reaches `target`,
    /// found by trying all of them.
    fn completions(line: &[Cell], target: u32) -> Vec<Vec<bool>> {
        let len = line.len();
        (0u32..(1 << len))
            .map(|mask| (0..len).map(|k| (mask >> k) & 1 == 1).collect::<Vec<_>>())
            .filter(|shaded| {
                let fits = line.iter().zip(shaded).all(|(&cell, &shaded)| match cell {
                    Cell::Shaded => shaded,
                    Cell::Empty => !shaded,
                    Cell::Unknown => true,
                });
                let total: u32 = (1..).zip(shaded).filter(|(_, s)| **s).map(|(v, _)| v).sum();
                fits && total == target
            })
            .collect()
    }

    /// The value every completion agrees on for cell `k`, if any.
    fn agreed(completions: &[Vec<bool>], k: usize) -> Cell {
        if completions.iter().all(|c| c[k]) {
            Cell::Shaded
        } else if completions.iter().all(|c| !c[k]) {
            Cell::Empty
        } else {
            Cell::Unknown
        }
    }

    fn random_lines(count: usize) -> impl Iterator<Item = (Vec<Cell>, u32)> {
        let mut rng = SplitMix64::new(7);
        (0..count).map(move |_| {
            let len = 1 + rng.below(9) as usize;
            let line = (0..len)
                .map(|_| match rng.below(4) {
                    0 => Cell::Shaded,
                    1 => Cell::Empty,
                    _ => Cell::Unknown,
                })
                .collect();
            let max = (len * (len + 1) / 2) as u64;
            (line, rng.below(max + 2) as u32)
        })
    }

    /// A straightforward version of `solve_full` using one `bool` per total,
    /// to check the bit set version on lines too long for brute force.
    fn solve_full_reference(line: &[Cell], target: u32) -> Result<Vec<Cell>, Contradiction> {
        let (len, target) = (line.len(), target as usize);
        // reach[k][t]: cells k.. can add up to exactly t.
        let mut reach = vec![vec![false; target + 1]; len + 1];
        reach[len][0] = true;
        for k in (0..len).rev() {
            for t in 0..=target {
                reach[k][t] = (line[k] != Cell::Shaded && reach[k + 1][t])
                    || (line[k] != Cell::Empty && t > k && reach[k + 1][t - k - 1]);
            }
        }
        // made[k][t]: cells ..k can add up to exactly t.
        let mut made = vec![vec![false; target + 1]; len + 1];
        made[0][0] = true;
        for k in 0..len {
            for t in 0..=target {
                made[k + 1][t] = (line[k] != Cell::Shaded && made[k][t])
                    || (line[k] != Cell::Empty && t > k && made[k][t - k - 1]);
            }
        }
        if !made[len][target] {
            return Err(Contradiction);
        }
        let mut solved = line.to_vec();
        for k in (0..len).filter(|&k| line[k] == Cell::Unknown) {
            let possible = |extra: usize| {
                (0..=target)
                    .any(|t| made[k][t] && t + extra <= target && reach[k + 1][target - t - extra])
            };
            solved[k] = match (possible(k + 1), possible(0)) {
                (true, true) => Cell::Unknown,
                (true, false) => Cell::Shaded,
                (false, true) => Cell::Empty,
                (false, false) => return Err(Contradiction),
            };
        }
        Ok(solved)
    }

    #[test]
    fn full_solver_matches_reference_on_long_lines() {
        let mut rng = SplitMix64::new(11);
        for _ in 0..3_000 {
            let len = 10 + rng.below(55) as usize;
            // Mostly unknown cells, so many targets stay reachable.
            let line: Vec<Cell> = (0..len)
                .map(|_| match rng.below(10) {
                    0 => Cell::Shaded,
                    1 => Cell::Empty,
                    _ => Cell::Unknown,
                })
                .collect();
            let target = rng.below((len * (len + 1) / 2) as u64 + 1) as u32;
            assert_eq!(
                run(solve_full, &line, target),
                solve_full_reference(&line, target),
                "{line:?} -> {target}"
            );
        }
    }

    #[test]
    fn full_solver_matches_brute_force() {
        for (line, target) in random_lines(20_000) {
            let completions = completions(&line, target);
            match run(solve_full, &line, target) {
                Err(Contradiction) => assert!(completions.is_empty(), "{line:?} -> {target}"),
                Ok(solved) => {
                    assert!(!completions.is_empty(), "{line:?} -> {target}");
                    for (k, &cell) in solved.iter().enumerate() {
                        assert_eq!(cell, agreed(&completions, k), "{line:?} -> {target}");
                    }
                }
            }
        }
    }
}
