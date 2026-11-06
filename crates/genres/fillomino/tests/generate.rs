use pencil_puzzles_fillomino::*;

/// T3, T9: unique solutions, matching ratings, reproducible seeds, and
/// limits that only decide whether generation succeeds.
#[test]
fn generated_puzzles_have_exact_ratings_unique_solutions_and_repeatable_limits() {
    for shape in ["1x1", "1x2", "2x3", "4x3", "5x5"] {
        for seed in 0..6 {
            let options = GenerateOptions::new(shape.parse().unwrap(), seed);
            let g = generate(&options, &Budget::new(50_000_000)).unwrap();
            assert!(g.rating.level < Difficulty::Extreme);
            assert_eq!(generate(&options, &Budget::new(50_000_000)).unwrap(), g);
            assert!(g.puzzle.is_solved_by(&g.solution));
            assert_eq!(
                solve(&g.puzzle, &Budget::unlimited()).unwrap(),
                Solutions::Unique(g.solution.clone())
            );
            assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
            for step in rate_with_steps(&g.puzzle, &Budget::unlimited()).unwrap().1 {
                for lit in step.removed {
                    if let Place::Cell { row, col } = lit.place {
                        assert_ne!(
                            lit.value,
                            g.solution.get(row as usize, col as usize).token()
                        );
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
    for shape in ["5x5", "4x6"] {
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
    let mut options = GenerateOptions::new("10x10".parse().unwrap(), 0);
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
    let empty = Puzzle::new("10x10".parse().unwrap(), vec![0; 100]).unwrap();
    assert!(matches!(
        solve(&empty, &Budget::new(1_000_000)),
        Ok(Solutions::Multiple(..)) | Err(BudgetExceeded)
    ));
    assert!(matches!(
        hint(
            &g.puzzle,
            &Answer::new(g.puzzle.shape()),
            &Budget::new(100_000_000)
        )
        .unwrap(),
        Hint::Step(_)
    ));
    assert_eq!(
        hint(&g.puzzle, &g.solution, &Budget::unlimited()).unwrap(),
        Hint::Solved
    );
    if g.puzzle.clues()[0] == 0 {
        let mut wrong = g.solution.clone();
        let other = if g.solution.get(0, 0) == Mark::Number(1) {
            2
        } else {
            1
        };
        wrong.set(0, 0, Mark::Number(other));
        assert!(matches!(
            hint(&g.puzzle, &wrong, &Budget::unlimited()),
            Err(HintError::InvalidAnswer(_))
        ));
    }
    let mut o = GenerateOptions::new("1x1".parse().unwrap(), 0);
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

/// A region of 16 or more cells has no exact candidate, so the rater
/// reports such a puzzle unrated, though it solves.
#[test]
fn regions_of_sixteen_or_more_are_solved_but_unrated() {
    let p = Puzzle::from_id("fillomino:4x4:-16o").unwrap();
    assert!(solve(&p, &Budget::unlimited()).unwrap().is_unique());
    assert_eq!(rate(&p, &Budget::unlimited()), Err(RateError::Unrated));
    let p = Puzzle::from_id("fillomino:3x5:-15n").unwrap();
    assert!(rate(&p, &Budget::unlimited()).is_ok());
}
