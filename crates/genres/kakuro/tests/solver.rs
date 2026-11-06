mod support;
use pencil_puzzles_core::SplitMix64;
use pencil_puzzles_kakuro::*;

/// Small layouts, including non-square ones.
fn layouts() -> Vec<(Shape, Vec<bool>)> {
    vec![
        support::layout("3x3", &[]),
        support::layout("3x4", &[]),
        support::layout("4x3", &[]),
    ]
}

/// Every full assignment of a layout with no digit repeated in a run.
fn distinct(s: Shape, white: &[bool]) -> Vec<Vec<u8>> {
    let base = support::puzzle(s, white, &vec![1; s.cells()]);
    let runs = support::runs(&base);
    support::assignments(s.cells(), &support::whites(&base))
        .into_iter()
        .filter(|d| {
            runs.iter().all(|(_, _, cells)| {
                (0..cells.len()).all(|a| (0..a).all(|b| d[cells[a]] != d[cells[b]]))
            })
        })
        .collect()
}

/// T1, T6: the solver agrees with brute force, and the checker reports
/// exactly the rules each full assignment breaks.
#[test]
fn solver_and_checker_match_brute_force() {
    let mut rng = SplitMix64::new(311);
    for (s, white) in layouts() {
        let fillings = distinct(s, &white);
        let cells = support::whites(&support::puzzle(s, &white, &vec![1; s.cells()]));
        for round in 0..120 {
            // Sums from a filling (often unique), or from random digits.
            let digits: Vec<u8> = if round % 2 == 0 {
                fillings[rng.below(fillings.len() as u64) as usize].clone()
            } else {
                (0..s.cells())
                    .map(|i| if white[i] { 1 + rng.below(9) as u8 } else { 0 })
                    .collect()
            };
            let p = support::puzzle(s, &white, &digits);
            let truth: Vec<&Vec<u8>> = fillings
                .iter()
                .filter(|f| support::keeps_runs(&p, f))
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
            if round < 6 {
                for _ in 0..300 {
                    let mut d = vec![0u8; s.cells()];
                    for &i in &cells {
                        d[i] = 1 + rng.below(9) as u8;
                    }
                    let a = support::answer(&p, &d);
                    let violations = check(&p, &a, &Budget::unlimited()).unwrap();
                    for (clue, sum, run) in support::runs(&p) {
                        let reported = violations.iter().any(|v| {
                            v.rule == 1
                                && v.place
                                    == (Place::Cell {
                                        row: (clue / s.cols()) as u32,
                                        col: (clue % s.cols()) as u32,
                                    })
                                && v.values.contains(&("sum", i64::from(sum)))
                        });
                        assert_eq!(reported, !support::run_ok(&d, sum, &run), "{} {a}", p.id());
                    }
                    assert!(violations.iter().all(|v| v.rule == 1));
                }
            }
        }
    }
}

/// T6: a partial answer is never reported for a run that some way of
/// filling in its unmarked cells keeps, and is reported whenever none does.
#[test]
fn partial_checks_are_exact_per_run() {
    let mut rng = SplitMix64::new(92);
    for (s, white) in layouts() {
        let fillings = distinct(s, &white);
        for _ in 0..60 {
            let f = &fillings[rng.below(fillings.len() as u64) as usize];
            let p = support::puzzle(s, &white, f);
            let cols = s.cols();
            let mut a = Answer::new(s);
            let mut open = Vec::new();
            let mut digits = vec![0u8; s.cells()];
            for i in support::whites(&p) {
                let mark = match rng.below(3) {
                    0 => Mark::Unmarked,
                    1 => Mark::Digit(f[i]),
                    _ => Mark::Digit(1 + rng.below(9) as u8),
                };
                if let Mark::Digit(d) = mark {
                    digits[i] = d;
                } else {
                    open.push(i);
                }
                a.set(i / cols, i % cols, mark);
            }
            let completions: Vec<Vec<u8>> = support::assignments(s.cells(), &open)
                .into_iter()
                .map(|c| (0..s.cells()).map(|i| digits[i].max(c[i])).collect())
                .collect();
            let violations = check(&p, &a, &Budget::unlimited()).unwrap();
            for (clue, sum, run) in support::runs(&p) {
                let place = Place::Cell {
                    row: (clue / cols) as u32,
                    col: (clue % cols) as u32,
                };
                let reported = violations
                    .iter()
                    .any(|v| v.place == place && v.values.contains(&("sum", i64::from(sum))));
                let possible = completions.iter().any(|c| support::run_ok(c, sum, &run));
                assert_eq!(reported, !possible, "{} {a}", p.id());
            }
        }
    }
}

#[test]
fn budgets_fit_errors_and_finished_answers() {
    let p: Puzzle = "grid:\n# 4/ 6/\n/3 . .\n/7 . .".parse().unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(count_solutions(&p, 0, &Budget::new(0)).unwrap(), 0);
    let Solutions::Unique(solution) = solve(&p, &Budget::unlimited()).unwrap() else {
        panic!("not unique");
    };
    assert_eq!(solution.to_string(), "###\n#12\n#34");
    assert!(check(&p, &Answer::new(p.shape()), &Budget::unlimited()).is_err());
    let empty: Answer = "###\n#..\n#..".parse().unwrap();
    assert!(check(&p, &empty, &Budget::unlimited()).unwrap().is_empty());
    let unfinished = empty.finished();
    let violations = check(&p, &unfinished, &Budget::unlimited()).unwrap();
    assert_eq!(violations.iter().filter(|v| v.rule == 2).count(), 4);
    assert_eq!(violations.iter().filter(|v| v.rule == 1).count(), 4);
    // Only R2: the runs through the empty cell already add up.
    let q: Puzzle = "grid:\n# 4/ 4/\n/1 . .\n/7 . .".parse().unwrap();
    let a: Answer = "###\n#10\n#34".parse().unwrap();
    let violations = check(&q, &a, &Budget::unlimited()).unwrap();
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, 2);
    // Repeated digits name the digit.
    let twice: Answer = "###\n#11\n#..".parse().unwrap();
    let violations = check(&p, &twice, &Budget::unlimited()).unwrap();
    assert_eq!(
        violations[0].values,
        [("down", 0), ("sum", 3), ("total", 2), ("repeated", 1)]
    );
    let blocks = Puzzle::from_id("kakuro:3x3:I/").unwrap();
    assert!(solve(&blocks, &Budget::unlimited()).unwrap().is_unique());
    let impossible: Puzzle = "grid:\n# 4/ 6/\n/2 . .\n/7 . .".parse().unwrap();
    assert_eq!(
        solve(&impossible, &Budget::unlimited()).unwrap(),
        Solutions::None
    );
}

#[test]
fn view_draws_blocks_and_ignores_an_answer_that_does_not_fit() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "grid:\n# 4/ 6/\n/3 . .\n/7 . .".parse().unwrap();
    let a = Answer::new("3x4".parse().unwrap());
    let view = Kakuro::view(&p, Some(&a));
    assert_eq!(view.cells.len(), 9);
    assert_eq!(view.marks.len(), 10);
    let block = view.cells[1].block.as_ref().unwrap();
    assert_eq!((block.upper.as_str(), block.lower.as_str()), ("", "4"));
    assert_eq!(block.name, "down 4");
    assert!(view.cells[0].block.as_ref().unwrap().upper.is_empty());
    assert_eq!(view.cells[3].block.as_ref().unwrap().upper, "3");
    assert!(view.cells[4].editable && view.cells[4].value == ".");
    let values: Vec<String> = view.cells.iter().map(|c| c.value.clone()).collect();
    assert_eq!(
        Kakuro::answer_from_values(&p, &values).unwrap(),
        Kakuro::empty_answer(&p)
    );
    let mut wrong = values.clone();
    wrong[0] = ".".into();
    assert!(Kakuro::answer_from_values(&p, &wrong).is_err());
    let editor = Kakuro::editor("3x3".parse().unwrap(), Some(&p));
    let fields: Vec<String> = editor.fields.iter().map(|f| f.value.clone()).collect();
    assert_eq!(fields, ["#", "4/", "6/", "/3", "", "", "/7", "", ""]);
    assert_eq!(Kakuro::puzzle_from_editor(p.shape(), &fields).unwrap(), p);
    let blank = Kakuro::editor("4x4".parse().unwrap(), None);
    assert_eq!(blank.fields[0].value, "#");
    assert_eq!(blank.fields[5].value, "");
    assert!(blank.fields[0].pattern.is_some());
    let mut bad = fields.clone();
    bad[4] = "x".into();
    assert!(Kakuro::puzzle_from_editor(p.shape(), &bad).is_err());
}
