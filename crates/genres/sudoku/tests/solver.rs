mod support;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_sudoku::*;
fn digits(a: &Answer) -> Vec<u8> {
    (0..a.shape().cells())
        .map(|i| a.get(i / a.shape().size(), i % a.shape().size()))
        .collect()
}
#[test]
fn every_four_by_four_solution_and_random_clue_masks_match_brute_force() {
    let all = support::solutions(4, &[0; 16], usize::MAX);
    assert_eq!(all.len(), 288);
    let mut rng = SplitMix64::new(713);
    for truth in all {
        for trial in 0..8 {
            let mut clues = truth.clone();
            if trial != 0 {
                for v in &mut clues {
                    if rng.below(3) != 0 {
                        *v = 0;
                    }
                }
            }
            if trial == 7 {
                let i = rng.below(16) as usize;
                clues[i] = 1 + rng.below(4) as u8;
            }
            let expected = support::solutions(4, &clues, usize::MAX);
            let p = Puzzle::new(Shape::new(4).unwrap(), clues.clone()).unwrap();
            assert_eq!(
                count_solutions(&p, usize::MAX, &Budget::unlimited()).unwrap(),
                expected.len(),
                "{}",
                p.id()
            );
            match solve(&p, &Budget::unlimited()).unwrap() {
                Solutions::None => assert!(expected.is_empty()),
                Solutions::Unique(a) => assert_eq!(expected, vec![digits(&a)]),
                Solutions::Multiple(a, b) => {
                    assert!(expected.len() > 1);
                    assert_ne!(a, b);
                    assert!(support::satisfies(4, &clues, &digits(&a)));
                    assert!(support::satisfies(4, &clues, &digits(&b)));
                }
            }
        }
    }
}
#[test]
fn nine_by_nine_matches_independent_pruning() {
    let mut rng = SplitMix64::new(993);
    let truth: Vec<u8> = (0..81)
        .map(|i| ((i / 9 * 3 + i / 9 / 3 + i % 9) % 9 + 1) as u8)
        .collect();
    for trial in 0..40 {
        let mut clues = truth.clone();
        for _ in 0..35 {
            clues[rng.below(81) as usize] = 0;
        }
        if trial % 3 == 0 {
            clues[rng.below(81) as usize] = 1 + rng.below(9) as u8;
        }
        let expected = support::solutions(9, &clues, 2);
        let p = Puzzle::new(Shape::default(), clues.clone()).unwrap();
        assert_eq!(
            count_solutions(&p, 2, &Budget::unlimited()).unwrap(),
            expected.len()
        );
        if let Some(a) = solve(&p, &Budget::unlimited()).unwrap().any() {
            assert!(support::satisfies(9, &clues, &digits(a)));
        }
    }
}
#[test]
fn budgets_and_count_limits() {
    let p = Puzzle::new(Shape::new(4).unwrap(), vec![0; 16]).unwrap();
    assert_eq!(count_solutions(&p, 0, &Budget::new(0)), Ok(0));
    assert_eq!(count_solutions(&p, 1, &Budget::unlimited()), Ok(1));
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(
        rate(&p, &Budget::unlimited()),
        Err(RateError::MultipleSolutions)
    );
}
