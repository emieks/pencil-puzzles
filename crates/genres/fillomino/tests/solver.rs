mod support;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_fillomino::*;

/// Every full assignment of a shape that keeps R1, found by brute force.
fn valid(shape: Shape) -> Vec<Vec<u32>> {
    let empty = Puzzle::new(shape, vec![0; shape.cells()]).unwrap();
    support::solutions(&empty)
}

/// T1, T6: the solver agrees with brute force, and the checker reports
/// exactly the rules each full assignment breaks.
#[test]
fn solver_and_checker_match_brute_force() {
    let mut rng = SplitMix64::new(311);
    for shape in ["1x1", "1x4", "2x2", "2x3", "3x2", "1x6"] {
        let s: Shape = shape.parse().unwrap();
        let fillings = valid(s);
        let all = support::assignments(s.cells());
        for round in 0..120 {
            // Clues from a valid filling (often unique) or at random.
            let clues: Vec<u32> = if round % 2 == 0 {
                let f = &fillings[rng.below(fillings.len() as u64) as usize];
                f.iter()
                    .map(|&v| if rng.below(3) == 0 { v } else { 0 })
                    .collect()
            } else {
                (0..s.cells())
                    .map(|_| {
                        if rng.below(3) == 0 {
                            1 + rng.below(s.cells() as u64) as u32
                        } else {
                            0
                        }
                    })
                    .collect()
            };
            let p = Puzzle::new(s, clues).unwrap();
            let truth: Vec<_> = fillings
                .iter()
                .filter(|f| support::rules(&p, f)[2])
                .collect();
            assert_eq!(
                count_solutions(&p, usize::MAX, &Budget::unlimited()).unwrap(),
                truth.len(),
                "{}",
                p.id()
            );
            match solve(&p, &Budget::unlimited()).unwrap() {
                Solutions::None => assert!(truth.is_empty()),
                Solutions::Unique(a) => {
                    assert_eq!(truth.len(), 1);
                    assert_eq!(a, support::answer(&p, truth[0]));
                }
                Solutions::Multiple(a, b) => {
                    assert!(truth.len() > 1);
                    assert_ne!(a, b);
                    assert!(p.is_solved_by(&a) && p.is_solved_by(&b));
                }
            }
            if round < 4 {
                for values in &all {
                    let a = support::answer(&p, values);
                    let violations = check(&p, &a, &Budget::unlimited()).unwrap();
                    let rules = support::rules(&p, values);
                    for (rule, &kept) in rules.iter().enumerate() {
                        assert_eq!(
                            violations.iter().any(|v| v.rule == rule as u32 + 1),
                            !kept,
                            "{} {a} R{}",
                            p.id(),
                            rule + 1
                        );
                    }
                }
            }
        }
    }
}

/// T6: a partial answer is never reported for a rule that some way of
/// filling in its unmarked cells would keep.
#[test]
fn partial_checks_never_flag_a_rule_some_completion_keeps() {
    let mut rng = SplitMix64::new(92);
    let s: Shape = "2x3".parse().unwrap();
    let all = support::assignments(6);
    for _ in 0..300 {
        let clues: Vec<u32> = (0..6)
            .map(|_| {
                if rng.below(4) == 0 {
                    1 + rng.below(4) as u32
                } else {
                    0
                }
            })
            .collect();
        let p = Puzzle::new(s, clues).unwrap();
        // Partial marks: clue cells keep their clue, others are unmarked or
        // a random number.
        let marks: Vec<u32> = (0..6)
            .map(|i| match p.clues()[i] {
                0 if rng.below(2) == 0 => 1 + rng.below(6) as u32,
                c => c,
            })
            .collect();
        let mut a = Answer::new(s);
        for (i, &m) in marks.iter().enumerate() {
            if m > 0 {
                a.set(i / 3, i % 3, Mark::Number(m));
            }
        }
        let violations = check(&p, &a, &Budget::unlimited()).unwrap();
        let completions: Vec<_> = all
            .iter()
            .filter(|v| (0..6).all(|i| marks[i] == 0 || marks[i] == v[i]))
            .collect();
        for v in violations {
            let rule = v.rule as usize - 1;
            assert!(
                completions.iter().all(|c| !support::rules(&p, c)[rule]),
                "R{} flagged although a completion keeps it: {} {a}",
                v.rule,
                p.id()
            );
        }
    }
}

#[test]
fn budgets_shape_errors_and_finished_answers() {
    let p: Puzzle = "grid:\n2.\n..\n.1".parse().unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(count_solutions(&p, 0, &Budget::new(0)).unwrap(), 0);
    let solution = solve(&p, &Budget::unlimited()).unwrap();
    let Solutions::Unique(solution) = solution else {
        panic!("{solution:?}");
    };
    assert_eq!(solution.to_string(), "22\n33\n31");
    assert!(
        check(
            &p,
            &Answer::new("1x1".parse().unwrap()),
            &Budget::unlimited()
        )
        .is_err()
    );
    let unfinished = Answer::new(p.shape()).finished();
    let violations = check(&p, &unfinished, &Budget::unlimited()).unwrap();
    assert_eq!(violations.iter().filter(|v| v.rule == 2).count(), 6);
    assert!(violations.iter().any(|v| v.rule == 3));
    let mut wrong = solution.clone();
    wrong.set(0, 0, Mark::Number(1));
    assert!(
        check(&p, &wrong, &Budget::unlimited())
            .unwrap()
            .iter()
            .any(|v| v.rule == 3)
    );
}

#[test]
fn view_ignores_an_answer_with_a_different_shape() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "grid:\n...\n...\n...".parse().unwrap();
    let a = Answer::new("1x1".parse().unwrap());
    let view = Fillomino::view(&p, Some(&a));
    assert_eq!(view.cells.len(), 9);
    assert!(view.cells.iter().all(|c| c.value == "."));
    assert_eq!(view.marks.len(), 10);
    // Sparse clues offer 1 to 9; a larger printed clue extends the range.
    let sparse = Puzzle::from_id("fillomino:5x5:3x").unwrap();
    assert_eq!(Fillomino::view(&sparse, None).marks.len(), 10);
    let big = Puzzle::from_id("fillomino:4x4:-12o").unwrap();
    assert_eq!(Fillomino::view(&big, None).marks.len(), 13);
    let tiny = Puzzle::from_id("fillomino:1x2:1a").unwrap();
    assert_eq!(Fillomino::view(&tiny, None).marks.len(), 3);
}
