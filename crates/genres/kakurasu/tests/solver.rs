use std::collections::HashMap;

use pencil_puzzles_kakurasu::{
    Answer, Budget, Mark, Puzzle, Shape, Solutions, count_solutions, solve,
};

fn grid_from_bits(rows: usize, cols: usize, bits: u32) -> Answer {
    let mut grid = Answer::new(Shape::new(rows, cols).unwrap());
    for row in 0..rows {
        for col in 0..cols {
            grid.set(
                row,
                col,
                if (bits >> (row * cols + col)) & 1 == 1 {
                    Mark::Shaded
                } else {
                    Mark::Unshaded
                },
            );
        }
    }
    grid
}

/// Checks the solver against brute force: shade every possible grid of the
/// given size, count how many grids share each set of clues, and compare.
fn check_every_grid(rows: usize, cols: usize) {
    let mut grids_by_puzzle: HashMap<Puzzle, Vec<Answer>> = HashMap::new();
    for bits in 0..1 << (rows * cols) {
        let grid = grid_from_bits(rows, cols, bits);
        let puzzle = Puzzle::from_solution(&grid);
        grids_by_puzzle.entry(puzzle).or_default().push(grid);
    }
    for (puzzle, grids) in grids_by_puzzle {
        assert_eq!(
            count_solutions(&puzzle, usize::MAX, &Budget::unlimited()).unwrap(),
            grids.len(),
            "{puzzle}"
        );
        match solve(&puzzle, &Budget::unlimited()).unwrap() {
            Solutions::Unique(grid) => assert_eq!(vec![grid], grids, "{puzzle}"),
            Solutions::Multiple(first, second) => {
                assert!(grids.len() > 1, "{puzzle}");
                assert_ne!(first, second);
                assert!(grids.contains(&first) && grids.contains(&second));
            }
            _ => panic!("{puzzle} came from a grid, so it has a solution"),
        }
    }
}

#[test]
fn matches_brute_force_on_every_3x3_grid() {
    check_every_grid(3, 3);
}

#[test]
fn matches_brute_force_on_every_3x5_grid() {
    check_every_grid(3, 5);
}

#[test]
fn matches_brute_force_on_every_4x4_grid() {
    check_every_grid(4, 4);
}

#[test]
fn reports_puzzles_without_solutions() {
    // The row needs its only cell shaded, but the column needs it empty.
    let puzzle = Puzzle::new(vec![1], vec![0]).unwrap();
    assert_eq!(
        solve(&puzzle, &Budget::unlimited()).unwrap(),
        Solutions::None
    );
    assert_eq!(
        count_solutions(&puzzle, 10, &Budget::unlimited()).unwrap(),
        0
    );
}

#[test]
fn count_stops_at_limit() {
    // Each row and column is either 1 + 4 or 2 + 3, which allows two solutions.
    let puzzle = Puzzle::new(vec![5; 4], vec![5; 4]).unwrap();
    assert_eq!(
        count_solutions(&puzzle, usize::MAX, &Budget::unlimited()).unwrap(),
        2
    );
    assert_eq!(
        count_solutions(&puzzle, 1, &Budget::unlimited()).unwrap(),
        1
    );
    assert_eq!(
        count_solutions(&puzzle, 0, &Budget::unlimited()).unwrap(),
        0
    );
}

#[test]
fn solves_a_known_puzzle() {
    let puzzle: Puzzle = "rows: 4 5 4 10\ncols: 5 6 7 7".parse().unwrap();
    let expected: Answer = "#-#-\n-##-\n---#\n####".parse().unwrap();
    assert_eq!(
        solve(&puzzle, &Budget::unlimited()).unwrap(),
        Solutions::Unique(expected)
    );
}

// T1/T10: row-wise enumeration, with independent column bounds, no production propagation.
fn pruned_count(p: &Puzzle) -> usize {
    fn rows(p: &Puzzle, row: usize, totals: &mut [u32]) -> usize {
        if row == p.rows() {
            return usize::from(totals == p.col_clues());
        }
        let mut count = 0;
        for bits in 0u64..1u64 << p.cols() {
            let sum: u32 = (0..p.cols())
                .filter(|&c| bits >> c & 1 != 0)
                .map(|c| c as u32 + 1)
                .sum();
            if sum != p.row_clues()[row] {
                continue;
            }
            let mut next = totals.to_vec();
            for (c, t) in next.iter_mut().enumerate() {
                if bits >> c & 1 != 0 {
                    *t += row as u32 + 1;
                }
            }
            let remaining: u32 = (row + 2..=p.rows()).map(|r| r as u32).sum();
            if next
                .iter()
                .zip(p.col_clues())
                .any(|(&t, &goal)| t > goal || t + remaining < goal)
            {
                continue;
            }
            count += rows(p, row + 1, &mut next);
        }
        count
    }
    rows(p, 0, &mut vec![0; p.cols()])
}
#[test]
fn random_larger_puzzles_match_pruned_brute_force() {
    let mut rng = pencil_puzzles_core::SplitMix64::new(123);
    for size in [5, 6] {
        for _ in 0..100 {
            let mut a = Answer::new(Shape::new(size, size).unwrap());
            for r in 0..size {
                for c in 0..size {
                    a.set(
                        r,
                        c,
                        if rng.below(2) == 0 {
                            Mark::Shaded
                        } else {
                            Mark::Unshaded
                        },
                    );
                }
            }
            let p = Puzzle::from_solution(&a);
            assert_eq!(
                count_solutions(&p, usize::MAX, &Budget::unlimited()).unwrap(),
                pruned_count(&p)
            );
            let mut rows = p.row_clues().to_vec();
            rows[0] = rng.below((size * (size + 1) / 2 + 1) as u64) as u32;
            let p = Puzzle::new(rows, p.col_clues().to_vec()).unwrap();
            assert_eq!(
                count_solutions(&p, usize::MAX, &Budget::unlimited()).unwrap(),
                pruned_count(&p)
            );
        }
    }
}
