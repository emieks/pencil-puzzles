mod support;
use pencil_puzzles_aquarium::*;
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
                for water in &all {
                    let a = support::answer(&p, water);
                    let violations = check(&p, &a).unwrap();
                    let rules = support::rules(&p, water);
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
    for shape in ["2x3", "3x3", "4x2"] {
        let s: Shape = shape.parse().unwrap();
        let n = s.cells();
        let all = support::shadings(n);
        for _ in 0..300 {
            let p = support::random_puzzle(&mut rng, s);
            // 0 unmarked, 1 air, 2 water.
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
    let p: Puzzle = "rows: 1 2\ncols: 2 1\nregions:\nab\naa".parse().unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(count_solutions(&p, 0, &Budget::new(0)).unwrap(), 0);
    let Solutions::Unique(solution) = solve(&p, &Budget::unlimited()).unwrap() else {
        panic!("not unique");
    };
    assert_eq!(solution.to_string(), "#-\n##");
    assert!(check(&p, &Answer::new("1x1".parse().unwrap())).is_err());
    let unfinished = Answer::new(p.shape()).finished();
    let violations = check(&p, &unfinished).unwrap();
    assert_eq!(
        violations.iter().map(|v| v.rule).collect::<Vec<_>>(),
        [1, 1, 2, 2]
    );
    assert_eq!(violations[0].place, Place::Row(0));
    assert_eq!(violations[0].values, [("count", 0), ("clue", 1)]);
    // Air under water in tank a, which is region 0.
    let floating: Answer = "#.\n-.".parse().unwrap();
    let violations = check(&p, &floating).unwrap();
    assert_eq!(
        violations.last(),
        Some(&Violation::new(3, Place::Region(0), Vec::new()))
    );
    // Water beside air in one row of a tank.
    let uneven: Answer = "..\n#-".parse().unwrap();
    assert!(check(&p, &uneven).unwrap().iter().any(|v| v.rule == 3));
}

#[test]
fn view_shows_tanks_and_clues() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "rows: 1 2\ncols: 2 1\nregions:\nab\naa".parse().unwrap();
    let mut a = Answer::new(p.shape());
    a.set(0, 0, Mark::Shaded);
    let view = Aquarium::view(&p, Some(&a));
    let regions: Vec<_> = view.cells.iter().map(|c| c.region).collect();
    assert_eq!(regions, [Some(0), Some(1), Some(0), Some(0)]);
    assert_eq!(view.right, ["1", "2"]);
    assert_eq!(view.bottom, ["2", "1"]);
    assert_eq!(view.cells[0].value, "#");
    assert!(view.cells.iter().all(|c| c.editable));
    let other = Aquarium::view(&p, Some(&Answer::new("1x1".parse().unwrap())));
    assert!(other.cells.iter().all(|c| c.value == "."));
    let values: Vec<String> = ["#", "-", ".", "-"].map(String::from).to_vec();
    assert_eq!(
        Aquarium::answer_from_values(&p, &values)
            .unwrap()
            .to_string(),
        "#-\n.-"
    );
    assert!(Aquarium::answer_from_values(&p, &values[..3]).is_err());
    // The editor lists row clues, column clues, then a tank for each cell.
    let editor = Aquarium::editor("2x2".parse().unwrap(), None);
    assert_eq!(editor.fields.len(), 8);
    assert_eq!(editor.fields[0].max, 2);
    assert!(editor.fields[4].pattern.is_some());
    assert!(editor.fields.iter().all(|f| f.value.is_empty()));
    let loaded = Aquarium::editor("6x6".parse().unwrap(), Some(&p));
    let fields: Vec<String> = loaded.fields.iter().map(|f| f.value.clone()).collect();
    assert_eq!(fields, ["1", "2", "2", "1", "a", "b", "a", "a"]);
    let made = Aquarium::puzzle_from_editor("2x2".parse().unwrap(), &fields).unwrap();
    assert_eq!(made, p);
    let mut bad = fields.clone();
    bad[0] = "3".into();
    assert!(Aquarium::puzzle_from_editor("2x2".parse().unwrap(), &bad).is_err());
    bad[0] = String::new();
    assert!(Aquarium::puzzle_from_editor("2x2".parse().unwrap(), &bad).is_err());
    // Tank b in two pieces.
    let split: Vec<String> = ["1", "2", "2", "1", "b", "a", "a", "b"]
        .map(String::from)
        .to_vec();
    assert!(Aquarium::puzzle_from_editor("2x2".parse().unwrap(), &split).is_err());
    assert_eq!(
        p.render(Some(&a)),
        "   2 1\n  +-+-+\n1 |#| |\n  + +-+\n2 |   |\n  +-+-+"
    );
}
