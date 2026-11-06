mod support;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_hitori::*;

fn random_puzzle(rng: &mut SplitMix64, s: Shape) -> Puzzle {
    // Few distinct numbers make duplicates common.
    let k = 1 + rng.below(u64::from(s.largest()));
    let numbers = (0..s.cells()).map(|_| 1 + rng.below(k) as u32).collect();
    Puzzle::new(s, numbers).unwrap()
}

/// T1, T6: the solver agrees with brute force, and the checker reports
/// exactly the rules each full shading breaks.
#[test]
fn solver_and_checker_match_brute_force() {
    let mut rng = SplitMix64::new(311);
    for shape in [
        "1x1", "1x4", "2x2", "2x3", "3x2", "3x3", "3x4", "4x3", "1x7",
    ] {
        let s: Shape = shape.parse().unwrap();
        let all = support::shadings(s.cells());
        for round in 0..60 {
            let p = random_puzzle(&mut rng, s);
            let truth = support::solutions(&p);
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
                    assert_eq!(a, support::answer(&p, &truth[0]));
                }
                Solutions::Multiple(a, b) => {
                    assert!(truth.len() > 1);
                    assert_ne!(a, b);
                    assert!(p.is_solved_by(&a) && p.is_solved_by(&b));
                }
            }
            if round < 3 {
                for shaded in &all {
                    let a = support::answer(&p, shaded);
                    let violations = check(&p, &a, &Budget::unlimited()).unwrap();
                    let rules = support::rules(&p, shaded);
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
/// marking its unmarked cells would keep.
#[test]
fn partial_checks_never_flag_a_rule_some_completion_keeps() {
    let mut rng = SplitMix64::new(92);
    for shape in ["2x3", "3x3"] {
        let s: Shape = shape.parse().unwrap();
        let n = s.cells();
        let all = support::shadings(n);
        for _ in 0..300 {
            let p = random_puzzle(&mut rng, s);
            // 0 unmarked, 1 unshaded, 2 shaded.
            let marks: Vec<u64> = (0..n).map(|_| rng.below(3)).collect();
            let mut a = Answer::new(s);
            for (i, &m) in marks.iter().enumerate() {
                let mark = [Mark::Unmarked, Mark::Unshaded, Mark::Shaded][m as usize];
                a.set(i / s.cols(), i % s.cols(), mark);
            }
            let violations = check(&p, &a, &Budget::unlimited()).unwrap();
            let completions: Vec<_> = all
                .iter()
                .filter(|c| (0..n).all(|i| marks[i] == 0 || (marks[i] == 2) == c[i]))
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
}

#[test]
fn budgets_shape_errors_and_finished_answers() {
    let p: Puzzle = "grid:\n11\n12".parse().unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(count_solutions(&p, 0, &Budget::new(0)).unwrap(), 0);
    let Solutions::Unique(solution) = solve(&p, &Budget::unlimited()).unwrap() else {
        panic!("not unique");
    };
    assert_eq!(solution.to_string(), "#-\n--");
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
    assert_eq!(
        violations.iter().map(|v| v.rule).collect::<Vec<_>>(),
        [1, 1]
    );
    assert_eq!(violations[0].values, [("number", 1), ("count", 2)]);
    let mut all = Answer::new(p.shape());
    for i in 0..4 {
        all.set(i / 2, i % 2, Mark::Shaded);
    }
    let violations = check(&p, &all, &Budget::unlimited()).unwrap();
    assert_eq!(violations.iter().filter(|v| v.rule == 2).count(), 4);
    assert!(violations.iter().any(|v| v.rule == 3));
}

#[test]
fn view_shows_every_number_under_its_mark() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "grid:\n11\n12".parse().unwrap();
    let mut a = Answer::new(p.shape());
    a.set(0, 0, Mark::Shaded);
    let view = Hitori::view(&p, Some(&a));
    let labels: Vec<_> = view.cells.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["1", "1", "1", "2"]);
    assert_eq!(view.cells[0].value, "#");
    assert!(view.cells.iter().all(|c| c.editable));
    let other = Hitori::view(&p, Some(&Answer::new("1x1".parse().unwrap())));
    assert!(other.cells.iter().all(|c| c.value == "."));
    let values: Vec<String> = ["#", "-", ".", "-"].map(String::from).to_vec();
    assert_eq!(
        Hitori::answer_from_values(&p, &values).unwrap().to_string(),
        "#-\n.-"
    );
    assert!(Hitori::answer_from_values(&p, &values[..3]).is_err());
    let editor = Hitori::editor("3x2".parse().unwrap(), None);
    assert_eq!(editor.fields.len(), 6);
    assert_eq!(editor.fields[0].max, 3);
    let fields: Vec<String> = ["1", "2", "3", "1", "2", "3"].map(String::from).to_vec();
    let made = Hitori::puzzle_from_editor("3x2".parse().unwrap(), &fields).unwrap();
    assert_eq!(made.id(), "hitori:3x2:123123");
    let mut bad = fields.clone();
    bad[0] = "4".into();
    assert!(Hitori::puzzle_from_editor("3x2".parse().unwrap(), &bad).is_err());
    bad[0] = String::new();
    assert!(Hitori::puzzle_from_editor("3x2".parse().unwrap(), &bad).is_err());
}
