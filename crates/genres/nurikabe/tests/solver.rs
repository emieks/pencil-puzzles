mod support;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_nurikabe::*;
#[test]
fn solver_and_checker_match_independent_exhaustive_boards() {
    let mut rng = SplitMix64::new(677);
    for shape in ["1x1", "1x4", "2x3", "3x3"] {
        let s: Shape = shape.parse().unwrap();
        for _ in 0..100 {
            let clues = (0..s.cells())
                .map(|_| {
                    if rng.below(4) == 0 {
                        1 + rng.below(s.cells() as u64) as u32
                    } else {
                        0
                    }
                })
                .collect();
            let p = Puzzle::new(s, clues).unwrap();
            let truth = support::solutions(&p);
            let actual = solve(&p, &Budget::unlimited()).unwrap();
            assert_eq!(
                count_solutions(&p, usize::MAX, &Budget::unlimited()).unwrap(),
                truth.len(),
                "{}",
                p.id()
            );
            match actual {
                Solutions::None => assert!(truth.is_empty()),
                Solutions::Unique(a) => {
                    assert_eq!(truth.len(), 1);
                    assert_eq!(a, support::answer(&p, &truth[0]));
                }
                Solutions::Multiple(a, b) => {
                    assert!(truth.len() > 1);
                    assert_ne!(a, b);
                    assert!(p.is_solved_by(&a));
                    assert!(p.is_solved_by(&b));
                }
            }
            for land in support::assignments(&p) {
                let a = support::answer(&p, &land);
                let violations = check(&p, &a, &Budget::unlimited()).unwrap();
                let rules = support::rules(&p, &land);
                for (rule, &valid) in rules.iter().enumerate() {
                    assert_eq!(
                        violations.iter().any(|v| v.rule == rule as u32 + 1),
                        !valid,
                        "{} {a} rule {}",
                        p.id(),
                        rule + 1
                    );
                }
            }
        }
    }
}
#[test]
fn budgets_shape_errors_and_partial_checks() {
    let p: Puzzle = "grid:\n1.1\n...\n1.1".parse().unwrap();
    let solution = solve(&p, &Budget::unlimited())
        .unwrap()
        .any()
        .unwrap()
        .clone();
    let truth = support::solutions(&p);
    for mask in 0..512 {
        let mut a = Answer::new(p.shape());
        for i in 0..9 {
            if mask & (1 << i) != 0 {
                a.set(i / 3, i % 3, solution.get(i / 3, i % 3));
            }
        }
        assert!(check(&p, &a, &Budget::unlimited()).unwrap().is_empty());
    }
    assert_eq!(truth.len(), 1);
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(count_solutions(&p, 0, &Budget::new(0)).unwrap(), 0);
    assert_eq!(count_solutions(&p, 1, &Budget::unlimited()).unwrap(), 1);
    assert!(
        check(
            &p,
            &Answer::new("1x1".parse().unwrap()),
            &Budget::unlimited()
        )
        .is_err()
    );
    let mut wrong = solution.clone();
    wrong.set(0, 0, Mark::Shaded);
    assert!(
        check(&p, &wrong, &Budget::unlimited())
            .unwrap()
            .iter()
            .any(|v| v.rule == 5)
    );
}

#[test]
fn view_ignores_an_answer_with_a_different_shape() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "grid:\n...\n...\n...".parse().unwrap();
    let a = Answer::new("1x1".parse().unwrap());
    let view = Nurikabe::view(&p, Some(&a));
    assert_eq!(view.cells.len(), 9);
    assert!(view.cells.iter().all(|c| c.value == "."));
}
/// A partial answer is never reported broken for a rule that some way of
/// filling in its unmarked cells would satisfy (T6).
#[test]
fn partial_checks_never_flag_a_rule_some_completion_satisfies() {
    let mut rng = SplitMix64::new(91);
    let s: Shape = "2x3".parse().unwrap();
    for _ in 0..400 {
        let clues = (0..6)
            .map(|_| {
                if rng.below(3) == 0 {
                    1 + rng.below(4) as u32
                } else {
                    0
                }
            })
            .collect();
        let Ok(p) = Puzzle::new(s, clues) else {
            continue;
        };
        // 0 unmarked, 1 shaded (sea), 2 unshaded (land). Shading a number is
        // an explicit R5 violation, covered elsewhere; this test is about
        // what unmarked cells could still become.
        let marks: Vec<u64> = (0..6)
            .map(|i| match rng.below(3) {
                1 if p.clues()[i] > 0 => 0,
                m => m,
            })
            .collect();
        let mut a = Answer::new(s);
        for (i, &m) in marks.iter().enumerate() {
            match m {
                1 => a.set(i / 3, i % 3, Mark::Shaded),
                2 => a.set(i / 3, i % 3, Mark::Unshaded),
                _ => {}
            }
        }
        let Ok(violations) = check(&p, &a, &Budget::unlimited()) else {
            continue;
        };
        let completions: Vec<Vec<bool>> = (0..64u32)
            .map(|bits| (0..6).map(|i| bits >> i & 1 == 1).collect())
            .filter(|land: &Vec<bool>| {
                // Unmarked numbered cells are land.
                (0..6).all(|i| match marks[i] {
                    1 => !land[i],
                    2 => land[i],
                    _ => p.clues()[i] == 0 || land[i],
                })
            })
            .collect();
        for v in violations {
            let rule = v.rule as usize - 1;
            assert!(
                completions
                    .iter()
                    .all(|land| !support::rules(&p, land)[rule]),
                "R{} flagged although some completion satisfies it: {} {marks:?}",
                v.rule,
                p.id()
            );
        }
    }
}
