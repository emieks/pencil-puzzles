mod support;
use pencil_puzzles_core::{Genre, SplitMix64};
use pencil_puzzles_hashi::*;

const SHAPES: [&str; 7] = ["1x1", "1x5", "3x3", "3x4", "4x3", "2x5", "4x4"];

/// T1, T6: the solver agrees with brute force, and the checker reports
/// exactly the rules each full assignment breaks.
#[test]
fn solver_and_checker_match_brute_force() {
    let mut rng = SplitMix64::new(311);
    let mut unique = 0;
    for shape in SHAPES {
        let s: Shape = shape.parse().unwrap();
        for round in 0..150 {
            let p = if round % 2 == 0 {
                support::solvable_puzzle(&mut rng, s, 3)
            } else {
                support::random_puzzle(&mut rng, s, 3, 4)
            };
            let slots = support::slots(&p);
            if slots.len() > 9 {
                continue;
            }
            let truth = support::solutions(&p, &slots);
            assert_eq!(
                count_solutions(&p, &Budget::unlimited()).unwrap(),
                truth.len().min(2),
                "{}",
                p.id()
            );
            match solve(&p, &Budget::unlimited()).unwrap() {
                Solutions::None => assert!(truth.is_empty()),
                Solutions::Unique(a) => {
                    unique += 1;
                    assert_eq!(truth.len(), 1);
                    assert_eq!(a, support::answer(&p, &slots, &truth[0]).finished());
                }
                Solutions::Multiple(a, b) => {
                    assert!(truth.len() > 1);
                    assert_ne!(a, b);
                    assert!(p.is_solved_by(&a) && p.is_solved_by(&b));
                }
            }
            if round < 6 {
                for values in support::assignments(slots.len()) {
                    let a = support::answer(&p, &slots, &values).finished();
                    let violations = check(&p, &a, &Budget::unlimited()).unwrap();
                    let rules = support::rules(&p, &slots, &values);
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
    assert!(unique > 50, "only {unique} unique puzzles");
}

/// T6: a partial answer is never reported for a rule that some way of
/// drawing its undecided bridges would keep.
#[test]
fn partial_checks_never_flag_a_rule_some_completion_keeps() {
    let mut rng = SplitMix64::new(92);
    let mut reported = 0;
    for round in 0..600 {
        let s: Shape = SHAPES[2 + round % 5].parse().unwrap();
        let p = support::random_puzzle(&mut rng, s, 3, 4);
        let slots = support::slots(&p);
        if slots.len() > 8 {
            continue;
        }
        // Drawn bridges: undecided (0) or 1 or 2.
        let drawn: Vec<u8> = slots
            .iter()
            .map(|_| {
                if rng.below(2) == 0 {
                    0
                } else {
                    1 + rng.below(2) as u8
                }
            })
            .collect();
        let a = support::answer(&p, &slots, &drawn);
        let violations = check(&p, &a, &Budget::unlimited()).unwrap();
        reported += violations.len();
        let completions: Vec<_> = support::assignments(slots.len())
            .into_iter()
            .filter(|v| (0..slots.len()).all(|k| drawn[k] == 0 || drawn[k] == v[k]))
            .collect();
        for v in violations {
            let rule = v.rule as usize - 1;
            assert!(
                completions
                    .iter()
                    .all(|c| !support::rules(&p, &slots, c)[rule]),
                "R{} flagged although a completion keeps it: {} {a}",
                v.rule,
                p.id()
            );
        }
    }
    assert!(reported > 100);
}

#[test]
fn budgets_shape_errors_and_finished_answers() {
    let p: Puzzle = "grid:\n2.2\n...\n1.1".parse().unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    let Solutions::Unique(solution) = solve(&p, &Budget::unlimited()).unwrap() else {
        panic!("not unique");
    };
    assert_eq!(solution.to_string(), "2-2\n|.|\n1.1");
    assert!(
        check(
            &p,
            &Answer::new("1x1".parse().unwrap()),
            &Budget::unlimited()
        )
        .is_err()
    );
    assert!(
        check(&p, &solution, &Budget::unlimited())
            .unwrap()
            .is_empty()
    );
    // Undrawn bridges are undecided until the answer is finished.
    let empty = Hashi::empty_answer(&p);
    assert!(check(&p, &empty, &Budget::unlimited()).unwrap().is_empty());
    let violations = check(&p, &empty.finished(), &Budget::unlimited()).unwrap();
    assert_eq!(violations.iter().filter(|v| v.rule == 1).count(), 4);
    assert!(violations.iter().any(|v| v.rule == 3));
    // Crossing bridges are reported at the cell where they cross.
    let cross: Puzzle = "grid:\n.1.\n1.1\n.1.".parse().unwrap();
    let a = Hashi::parse_answer(&cross, ". 1 .\n1 -| 1\n. 1 .").unwrap_or_else(|e| panic!("{e}"));
    let v = check(&cross, &a, &Budget::unlimited()).unwrap();
    assert_eq!(v[0].rule, 2);
    assert_eq!(v[0].place, Place::Cell { row: 1, col: 1 });
}

#[test]
fn answers_must_fit_the_islands() {
    let p: Puzzle = "grid:\n2.2\n...\n1.1".parse().unwrap();
    for (text, ok) in [
        ("2=2\n...\n1-1", true),
        ("...\n...\n...", true),
        (".=.\n...\n.-.", true),
        ("3=2\n...\n1-1", false),
        ("2=2\n.1.\n1-1", false),
        ("2=2\n-..\n1-1", false),
        ("2=2\n|..\n1-1", true),
        ("2=2\nH..\n1-1", true),
        ("2-2-\n...\n1-1", false),
    ] {
        let parsed = text
            .parse::<Answer>()
            .and_then(|a| Hashi::parse_answer(&p, &a.to_string()));
        assert_eq!(parsed.is_ok(), ok, "{text:?} {parsed:?}");
    }
    let wide: Puzzle = "grid:\n2..2".parse().unwrap();
    assert!(Hashi::parse_answer(&wide, "2=-2").is_err());
}

#[test]
fn views_carry_islands_and_links() {
    let p: Puzzle = "grid:\n2.2\n...\n1.1".parse().unwrap();
    let Solutions::Unique(solution) = solve(&p, &Budget::unlimited()).unwrap() else {
        panic!("not unique");
    };
    let view = Hashi::view(&p, Some(&solution));
    assert_eq!(view.cells.len(), 9);
    assert!(view.cells[0].circled && !view.cells[0].editable);
    assert_eq!(view.links.len(), 4);
    let values: Vec<_> = view.links.iter().map(|l| l.value.as_str()).collect();
    assert_eq!(values, ["1", "1", "1", "."]);
    let mut marks: Vec<String> = view.cells.iter().map(|c| c.value.clone()).collect();
    marks.extend(view.links.iter().map(|l| l.value.clone()));
    let a = Hashi::answer_from_values(&p, &marks).unwrap();
    assert!(
        check(&p, &a.finished(), &Budget::unlimited())
            .unwrap()
            .is_empty()
    );
    marks[0] = "3".into();
    assert!(Hashi::answer_from_values(&p, &marks).is_err());
    assert!(Hashi::answer_from_values(&p, &marks[..9]).is_err());
    let other = Answer::new("1x1".parse().unwrap());
    assert!(
        Hashi::view(&p, Some(&other))
            .links
            .iter()
            .all(|l| l.value == ".")
    );
}
