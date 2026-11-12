mod support;
use pencil_puzzles_chocona::*;
use pencil_puzzles_core::SplitMix64;

/// T1, T6: the solver agrees with brute force, and the checker reports
/// exactly the rules each full shading breaks.
#[test]
fn solver_and_checker_match_brute_force() {
    let mut rng = SplitMix64::new(311);
    for shape in [
        "1x1", "1x4", "2x2", "2x3", "3x2", "3x3", "3x4", "4x3", "1x7", "5x1", "4x4", "2x8",
    ] {
        let s: Shape = shape.parse().unwrap();
        let all = support::shadings(s.cells());
        for round in 0..60 {
            let p = support::random_puzzle(&mut rng, s);
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
            if round < 3 && s.cells() <= 12 {
                for shaded in &all {
                    let a = support::answer(&p, shaded);
                    let violations = check(&p, &a).unwrap();
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
    for shape in ["2x3", "3x3", "4x2", "3x4"] {
        let s: Shape = shape.parse().unwrap();
        let n = s.cells();
        let all = support::shadings(n);
        for _ in 0..300 {
            let p = support::random_puzzle(&mut rng, s);
            // 0 unmarked, 1 unshaded, 2 shaded.
            let marks: Vec<u64> = (0..n).map(|_| rng.below(3)).collect();
            let mut a = Answer::new(s);
            for (i, &m) in marks.iter().enumerate() {
                let mark = [Mark::Unmarked, Mark::Unshaded, Mark::Shaded][m as usize];
                a.set(i / s.cols(), i % s.cols(), mark);
            }
            let violations = check(&p, &a).unwrap();
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
    let p: Puzzle = "regions:\naab\nacc\nclues: 2 . 0".parse().unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(count_solutions(&p, 0, &Budget::new(0)).unwrap(), 0);
    assert!(matches!(
        solve(&p, &Budget::unlimited()).unwrap(),
        Solutions::Multiple(..)
    ));
    let one: Puzzle = "regions:\naab\nacc\nclues: 3 . 1".parse().unwrap();
    let Solutions::Unique(solution) = solve(&one, &Budget::unlimited()).unwrap() else {
        panic!("not unique");
    };
    assert_eq!(solution.to_string(), "##-\n##-");
    assert!(check(&p, &Answer::new("1x1".parse().unwrap())).is_err());
    let unfinished = Answer::new(one.shape()).finished();
    let violations = check(&one, &unfinished).unwrap();
    assert_eq!(
        violations.iter().map(|v| v.rule).collect::<Vec<_>>(),
        [1, 1]
    );
    assert_eq!(violations[0].place, Place::Region(0));
    assert_eq!(violations[0].values, [("count", 0), ("clue", 3)]);
    // An L of shaded cells around an unshaded one.
    let bent: Answer = "##.\n-#.".parse().unwrap();
    assert!(check(&p, &bent).unwrap().contains(&Violation::new(
        2,
        Place::Cell { row: 0, col: 0 },
        Vec::new()
    )));
    // The same L with the fourth cell unmarked could still become a square.
    let open: Answer = "##.\n.#.".parse().unwrap();
    assert!(check(&p, &open).unwrap().iter().all(|v| v.rule != 2));
}

#[test]
fn view_and_editor_show_regions_and_numbers() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "regions:\naab\nacc\nclues: 2 . 0".parse().unwrap();
    let mut a = Answer::new(p.shape());
    a.set(0, 0, Mark::Shaded);
    let view = Chocona::view(&p, Some(&a));
    let regions: Vec<_> = view.cells.iter().map(|c| c.region).collect();
    assert_eq!(
        regions,
        [Some(0), Some(0), Some(1), Some(0), Some(2), Some(2)]
    );
    let labels: Vec<_> = view.cells.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["2", "", "", "", "0", ""]);
    assert_eq!(view.cells[0].value, "#");
    assert!(view.cells.iter().all(|c| c.editable));
    let other = Chocona::view(&p, Some(&Answer::new("1x1".parse().unwrap())));
    assert!(other.cells.iter().all(|c| c.value == "."));
    let values: Vec<String> = ["#", "-", ".", "-", ".", "."].map(String::from).to_vec();
    assert_eq!(
        Chocona::answer_from_values(&p, &values)
            .unwrap()
            .to_string(),
        "#-.\n-.."
    );
    assert!(Chocona::answer_from_values(&p, &values[..3]).is_err());
    // The editor has a field per cell: its region, and the number in the
    // region's first cell.
    let editor = Chocona::editor("2x2".parse().unwrap(), None);
    assert_eq!(editor.fields.len(), 4);
    assert!(editor.fields[0].pattern.is_some());
    assert!(editor.fields.iter().all(|f| f.value.is_empty()));
    let loaded = Chocona::editor("6x6".parse().unwrap(), Some(&p));
    let fields: Vec<String> = loaded.fields.iter().map(|f| f.value.clone()).collect();
    assert_eq!(fields, ["a2", "a", "b", "a", "c0", "c"]);
    assert!(loaded.view.cells.iter().all(|c| c.label.is_empty()));
    let made = Chocona::puzzle_from_editor("2x3".parse().unwrap(), &fields).unwrap();
    assert_eq!(made, p);
    // The number may be in any cell of its region.
    let moved: Vec<String> = ["a", "a", "b", "a2", "c", "c0"].map(String::from).to_vec();
    assert_eq!(
        Chocona::puzzle_from_editor("2x3".parse().unwrap(), &moved).unwrap(),
        p
    );
    for bad in [
        ["a2", "a1", "b", "a", "c0", "c"],
        ["a9", "a", "b", "a", "c0", "c"],
        ["a2", "", "b", "a", "c0", "c"],
        ["a2", "a", "b", "b", "c0", "c"],
        ["a:2", "a", "b", "a", "c0", "c"],
    ] {
        let bad: Vec<String> = bad.map(String::from).to_vec();
        assert!(
            Chocona::puzzle_from_editor("2x3".parse().unwrap(), &bad).is_err(),
            "{bad:?}"
        );
    }
    let numbered: Vec<String> = ["1:2", "1", "2", "1", "3:0", "3"]
        .map(String::from)
        .to_vec();
    assert_eq!(
        Chocona::puzzle_from_editor("2x3".parse().unwrap(), &numbered).unwrap(),
        p
    );
    assert_eq!(
        p.render(None),
        "+-+-+-+\n|2  |\u{20}|\n+ +-+-+\n| |0  |\n+-+-+-+"
    );
    assert_eq!(
        p.render(Some(&a)),
        "+-+-+-+\n|#  | |\n+ +-+-+\n| |   |\n+-+-+-+"
    );
}
