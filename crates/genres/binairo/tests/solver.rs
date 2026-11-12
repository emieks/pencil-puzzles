mod support;
use pencil_puzzles_binairo::*;
use pencil_puzzles_core::SplitMix64;

/// T1: the solver agrees with brute force, including non-square grids.
#[test]
fn solver_matches_brute_force() {
    let mut rng = SplitMix64::new(311);
    for shape in ["2x2", "4x4", "4x6", "6x4"] {
        let s: Shape = shape.parse().unwrap();
        let all = support::grids(s);
        let empty = Puzzle::new(s, vec![Mark::Unmarked; s.cells()]).unwrap();
        assert_eq!(
            count_solutions(&empty, usize::MAX, &Budget::unlimited()).unwrap(),
            support::solutions(&empty, &all).len()
        );
        for _ in 0..150 {
            let p = support::random_puzzle(&mut rng, s, &all);
            let truth = support::solutions(&p, &all);
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
                    assert_eq!(a, support::answer(s, &truth[0]));
                }
                Solutions::Multiple(a, b) => {
                    assert!(truth.len() > 1);
                    assert_ne!(a, b);
                    assert!(p.is_solved_by(&a) && p.is_solved_by(&b));
                }
            }
        }
    }
}

/// T6: on complete grids the checker reports exactly the rules each grid
/// breaks.
#[test]
fn checker_matches_brute_force_on_complete_grids() {
    let mut rng = SplitMix64::new(7);
    for shape in ["2x2", "4x4", "4x6"] {
        let s: Shape = shape.parse().unwrap();
        let n = s.cells();
        let all = support::grids(s);
        for _ in 0..20 {
            let p = support::random_puzzle(&mut rng, s, &all);
            for k in 0..400 {
                // Grids whose rows keep R1 and R2, and random digits.
                let cells: Vec<u8> = if k % 2 == 0 {
                    all[rng.below(all.len() as u64) as usize].clone()
                } else {
                    (0..n).map(|_| rng.below(2) as u8).collect()
                };
                let a = support::answer(s, &cells);
                let violations = check(&p, &a).unwrap();
                for (rule, &kept) in support::rules(&p, &cells).iter().enumerate() {
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

/// T6: a partial answer is never reported for a rule that some way of
/// filling its unmarked cells would keep.
#[test]
fn partial_checks_never_flag_a_rule_some_completion_keeps() {
    let mut rng = SplitMix64::new(92);
    for shape in ["2x2", "4x4"] {
        let s: Shape = shape.parse().unwrap();
        let n = s.cells();
        let all = support::grids(s);
        for _ in 0..400 {
            let p = support::random_puzzle(&mut rng, s, &all);
            // 0 and 1 for digits, 2 for unmarked; at most 10 unmarked.
            let marks: Vec<u8> = (0..n)
                .map(|i| {
                    if i < 10 && rng.below(2) == 0 {
                        2
                    } else {
                        rng.below(2) as u8
                    }
                })
                .collect();
            let mut a = Answer::new(s);
            for (i, &m) in marks.iter().enumerate() {
                if m < 2 {
                    a.set(i / s.cols(), i % s.cols(), Mark::of(m));
                }
            }
            // An unmarked given cell holds its given digit.
            let mut marks = marks;
            for (i, m) in marks.iter_mut().enumerate() {
                if *m == 2
                    && let Some(d) = p.givens()[i].digit()
                {
                    *m = d;
                }
            }
            let open: Vec<usize> = (0..n).filter(|&i| marks[i] == 2).collect();
            let completions: Vec<Vec<u8>> = (0..1u32 << open.len())
                .map(|b| {
                    let mut cells = marks.clone();
                    for (k, &i) in open.iter().enumerate() {
                        cells[i] = (b >> k & 1) as u8;
                    }
                    cells
                })
                .collect();
            for v in check(&p, &a).unwrap() {
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
fn violations_carry_their_numbers() {
    let p: Puzzle = "grid:\n1...\n....\n....\n....".parse().unwrap();
    let a: Answer = "0000\n1100\n....\n1100".parse().unwrap();
    let violations = check(&p, &a).unwrap();
    assert!(violations.contains(&Violation::new(
        1,
        Place::Row(0),
        vec![("start", 0), ("length", 4), ("digit", 0)]
    )));
    assert!(violations.contains(&Violation::new(
        2,
        Place::Row(0),
        vec![("zeros", 4), ("ones", 0)]
    )));
    assert!(violations.contains(&Violation::new(3, Place::Row(3), vec![("same_as", 1)])));
    assert!(violations.contains(&Violation::new(
        4,
        Place::Cell { row: 0, col: 0 },
        vec![("given", 1), ("actual", 0)]
    )));
    // A given counts as written in where the answer leaves it unmarked.
    let columns = check(&p, &"....\n1...\n1...\n....".parse().unwrap()).unwrap();
    assert_eq!(
        columns,
        [
            Violation::new(
                1,
                Place::Col(0),
                vec![("start", 0), ("length", 3), ("digit", 1)]
            ),
            Violation::new(2, Place::Col(0), vec![("zeros", 0), ("ones", 3)])
        ]
    );
}

#[test]
fn budgets_shape_errors_and_finished_answers() {
    let p: Puzzle = "grid:\n1.\n..".parse().unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    let Solutions::Unique(solution) = solve(&p, &Budget::unlimited()).unwrap() else {
        panic!("not unique");
    };
    assert_eq!(solution.to_string(), "10\n01");
    assert!(p.is_solved_by(&solution));
    assert!(check(&p, &Answer::new("4x4".parse().unwrap())).is_err());
    // Finished, blanks are 0s.
    let finished = "1.\n.1".parse::<Answer>().unwrap().finished();
    assert!(p.is_solved_by(&finished));
    let violations = check(&p, &Answer::new(p.shape()).finished()).unwrap();
    assert_eq!(
        violations.iter().map(|v| v.rule).collect::<Vec<_>>(),
        [2, 2, 2, 2, 3, 3, 4]
    );
    let none: Puzzle = "grid:\n11\n..".parse().unwrap();
    assert_eq!(solve(&none, &Budget::unlimited()).unwrap(), Solutions::None);
}

#[test]
fn view_shows_givens_and_marks() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "grid:\n1.\n..".parse().unwrap();
    let mut a = Answer::new(p.shape());
    a.set(1, 1, Mark::One);
    a.set(0, 0, Mark::Zero);
    let view = Binairo::view(&p, Some(&a));
    let values: Vec<_> = view.cells.iter().map(|c| c.value.as_str()).collect();
    assert_eq!(values, ["1", ".", ".", "1"]);
    let editable: Vec<_> = view.cells.iter().map(|c| c.editable).collect();
    assert_eq!(editable, [false, true, true, true]);
    let labels: Vec<_> = view.cells.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["1", "", "", ""]);
    let marks: Vec<_> = view.marks.iter().map(|m| m.value.as_str()).collect();
    assert_eq!(marks, [".", "1", "0"]);
    let other = Binairo::view(&p, Some(&Answer::new("4x4".parse().unwrap())));
    assert_eq!(other.cells[3].value, ".");
    let values: Vec<String> = ["1", "0", ".", "1"].map(String::from).to_vec();
    assert_eq!(
        Binairo::answer_from_values(&p, &values)
            .unwrap()
            .to_string(),
        "10\n.1"
    );
    assert!(Binairo::answer_from_values(&p, &values[..3]).is_err());
    let bad: Vec<String> = ["1", "0", "#", "1"].map(String::from).to_vec();
    assert!(Binairo::answer_from_values(&p, &bad).is_err());
    let editor = Binairo::editor("2x2".parse().unwrap(), None);
    assert_eq!(editor.fields.len(), 4);
    assert!(
        editor
            .fields
            .iter()
            .all(|f| f.value.is_empty() && f.max == 1)
    );
    let loaded = Binairo::editor("6x6".parse().unwrap(), Some(&p));
    let fields: Vec<String> = loaded.fields.iter().map(|f| f.value.clone()).collect();
    assert_eq!(fields, ["1", "", "", ""]);
    let made = Binairo::puzzle_from_editor("2x2".parse().unwrap(), &fields).unwrap();
    assert_eq!(made, p);
    let mut wrong = fields.clone();
    wrong[1] = "2".into();
    assert!(Binairo::puzzle_from_editor("2x2".parse().unwrap(), &wrong).is_err());
    assert!(Binairo::puzzle_from_editor("2x2".parse().unwrap(), &fields[..3]).is_err());
    assert_eq!(p.render(Some(&a)), "1 .\n. 1");
}
