use pencil_puzzles_kakuro::*;

/// T3, T9: unique solutions, matching ratings, reproducible seeds, and
/// limits that only decide whether generation succeeds.
#[test]
fn generated_puzzles_have_exact_ratings_unique_solutions_and_repeatable_limits() {
    for shape in ["1x1", "2x5", "3x3", "3x7", "4x4", "5x4", "6x6"] {
        for seed in 0..5 {
            let options = GenerateOptions::new(shape.parse().unwrap(), seed);
            let g = generate(&options, &Budget::new(100_000_000)).unwrap();
            assert!(g.rating.level < Difficulty::Extreme);
            assert_eq!(generate(&options, &Budget::new(100_000_000)).unwrap(), g);
            assert!(g.puzzle.is_solved_by(&g.solution));
            assert_eq!(
                solve(&g.puzzle, &Budget::unlimited()).unwrap(),
                Solutions::Unique(g.solution.clone())
            );
            assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
            for step in rate_with_steps(&g.puzzle, &Budget::unlimited()).unwrap().1 {
                for lit in step.removed {
                    if let Place::Cell { row, col } = lit.place {
                        let kept = g.solution.get(row as usize, col as usize).symbol();
                        assert_ne!(lit.value, kept.to_string());
                    }
                }
            }
            let mut enough = options.clone();
            enough.max_attempts = g.attempts;
            assert_eq!(generate(&enough, &Budget::unlimited()).unwrap(), g);
            for cap in [0, 1, 1000] {
                match generate(&options, &Budget::new(cap)) {
                    Ok(found) => assert_eq!(found, g),
                    Err(e) => assert_eq!(e, GenerateError::BudgetExceeded),
                }
            }
        }
    }
    for shape in ["6x6", "5x7"] {
        for level in [
            Difficulty::Easy,
            Difficulty::Medium,
            Difficulty::Hard,
            Difficulty::Expert,
        ] {
            let mut o = GenerateOptions::new(shape.parse().unwrap(), 1);
            o.difficulty = Some(level);
            let g = generate(&o, &Budget::new(100_000_000)).unwrap();
            assert_eq!(g.rating.level, level);
            assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
            assert_eq!(generate(&o, &Budget::new(100_000_000)).unwrap(), g);
        }
    }
}

#[test]
fn maximum_size_work_and_hints_are_bounded() {
    let mut options = GenerateOptions::new("16x16".parse().unwrap(), 1);
    options.difficulty = Some(Difficulty::Easy);
    assert_eq!(
        generate(&options, &Budget::new(1)),
        Err(GenerateError::BudgetExceeded)
    );
    let g = generate(&options, &Budget::new(100_000_000)).unwrap();
    assert_eq!(
        rate(&g.puzzle, &Budget::new(1)),
        Err(RateError::BudgetExceeded)
    );
    assert_eq!(solve(&g.puzzle, &Budget::new(0)), Err(BudgetExceeded));
    use pencil_puzzles_core::Genre;
    let empty = Kakuro::empty_answer(&g.puzzle);
    assert!(matches!(
        hint(&g.puzzle, &empty, &Budget::new(100_000_000)).unwrap(),
        Hint::Step(_)
    ));
    assert_eq!(
        hint(&g.puzzle, &g.solution, &Budget::unlimited()).unwrap(),
        Hint::Solved
    );
    let cols = g.puzzle.shape().cols();
    let i = g
        .puzzle
        .cells()
        .iter()
        .position(|&c| c == Cell::White)
        .unwrap();
    let mut wrong = empty.clone();
    let Mark::Digit(d) = g.solution.get(i / cols, i % cols) else {
        panic!("white cell without a digit");
    };
    wrong.set(i / cols, i % cols, Mark::Digit(d % 9 + 1));
    assert!(matches!(
        hint(&g.puzzle, &wrong, &Budget::unlimited()),
        Err(HintError::InvalidAnswer(_))
    ));
    assert!(matches!(
        hint(
            &g.puzzle,
            &Answer::new(g.puzzle.shape()),
            &Budget::unlimited()
        ),
        Err(HintError::InvalidAnswer(_))
    ));
    let mut o = GenerateOptions::new("3x3".parse().unwrap(), 0);
    o.max_attempts = 0;
    assert_eq!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 0 })
    );
    o.max_attempts = 3;
    o.difficulty = Some(Difficulty::Hard);
    assert_eq!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 3 })
    );
    o.difficulty = Some(Difficulty::Extreme);
    assert!(matches!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::UnsupportedRequest(_))
    ));
}
