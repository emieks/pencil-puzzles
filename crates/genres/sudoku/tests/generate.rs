use pencil_puzzles_sudoku::*;
#[test]
fn generated_puzzles_are_unique_deterministic_and_truly_rated() {
    for shape in ["1x1", "4x4", "9x9"] {
        for seed in 0..8 {
            let options = GenerateOptions::new(shape.parse().unwrap(), seed);
            let g = generate(&options, &Budget::new(2_000_000)).unwrap();
            assert_eq!(generate(&options, &Budget::new(2_000_000)).unwrap(), g);
            assert!(g.rating.level < Difficulty::Extreme);
            assert!(g.puzzle.is_solved_by(&g.solution));
            assert_eq!(
                solve(&g.puzzle, &Budget::unlimited()).unwrap(),
                Solutions::Unique(g.solution)
            );
            assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
        }
    }
    // Seed 10 is a quick Expert search (T3: every level the profile defines).
    for (level, seed) in [
        (Difficulty::Easy, 0),
        (Difficulty::Medium, 0),
        (Difficulty::Hard, 0),
        (Difficulty::Expert, 10),
    ] {
        let mut options = GenerateOptions::new("9x9".parse().unwrap(), seed);
        options.difficulty = Some(level);
        let g = generate(&options, &Budget::new(100_000_000)).unwrap();
        assert_eq!(g.rating.level, level);
        for step in rate_with_steps(&g.puzzle, &Budget::unlimited()).unwrap().1 {
            for removed in step.removed {
                if let Place::Cell { row, col } = removed.place {
                    assert_ne!(
                        removed.value.parse::<u8>().unwrap(),
                        g.solution.get(row as usize, col as usize)
                    );
                }
            }
        }

        assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
    }
}
#[test]
fn budgets_and_attempts_bound_work_without_changing_results() {
    let options = GenerateOptions::new("9x9".parse().unwrap(), 42);
    let expected = generate(&options, &Budget::unlimited()).unwrap();
    for limit in [0, 1, 100, 1000, 10_000] {
        match generate(&options, &Budget::new(limit)) {
            Ok(g) => assert_eq!(g, expected),
            Err(e) => assert_eq!(e, GenerateError::BudgetExceeded),
        }
    }
    let mut enough = options;
    enough.max_attempts = expected.attempts;
    assert_eq!(generate(&enough, &Budget::unlimited()).unwrap(), expected);
    enough.max_attempts = 0;
    assert_eq!(
        generate(&enough, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 0 })
    );
    let mut impossible = GenerateOptions::new("1x1".parse().unwrap(), 0);
    impossible.difficulty = Some(Difficulty::Hard);
    impossible.max_attempts = 5;
    assert!(matches!(
        generate(&impossible, &Budget::unlimited()),
        Err(GenerateError::UnsupportedRequest(_))
    ));
    impossible.params = Params::default();
    impossible.difficulty = Some(Difficulty::Extreme);
    assert!(matches!(
        generate(&impossible, &Budget::new(0)),
        Err(GenerateError::UnsupportedRequest(_))
    ));
}
#[test]
fn nine_by_nine_limits_and_hints() {
    let p = Puzzle::new(Shape::default(), vec![0; 81]).unwrap();
    assert_eq!(solve(&p, &Budget::new(0)), Err(BudgetExceeded));
    assert_eq!(rate(&p, &Budget::new(1)), Err(RateError::BudgetExceeded));
    let g = generate(
        &GenerateOptions::new(Params::default(), 4),
        &Budget::new(2_000_000),
    )
    .unwrap();
    let blank = Answer::new(g.puzzle.shape());
    // Hints skip bookkeeping: the first hint is a technique, not the routine
    // removal of givens from their peers.
    match hint(&g.puzzle, &blank, &Budget::unlimited()).unwrap() {
        Hint::Step(step) => assert_ne!(step.technique, "peer_elimination"),
        other => panic!("expected a step, got {other:?}"),
    }
    assert!(matches!(
        hint(&g.puzzle, &g.solution, &Budget::unlimited()).unwrap(),
        Hint::Solved
    ));
    let (_, steps) = rate_with_steps(&g.puzzle, &Budget::unlimited()).unwrap();
    for step in steps {
        for removed in step.removed {
            if let Place::Cell { row, col } = removed.place {
                assert_ne!(
                    removed.value.parse::<u8>().unwrap(),
                    g.solution.get(row as usize, col as usize)
                );
            }
        }
    }
}

#[test]
fn generator_is_sparse_and_symmetric() {
    for seed in 0..8 {
        let o = GenerateOptions::new("9x9".parse().unwrap(), seed);
        let g = generate(&o, &Budget::new(2_000_000)).unwrap();
        assert!(g.puzzle.givens().iter().filter(|&&v| v > 0).count() < 40);
        for i in 0..81 {
            assert_eq!(g.puzzle.givens()[i] == 0, g.puzzle.givens()[80 - i] == 0);
        }
        for i in 0..41 {
            if g.puzzle.givens()[i] == 0 {
                continue;
            }
            let mut clues = g.puzzle.givens().to_vec();
            clues[i] = 0;
            clues[80 - i] = 0;
            let reduced = Puzzle::new(g.puzzle.shape(), clues).unwrap();
            // A clue can be necessary for the human repertoire even when it
            // is redundant for uniqueness. Minimality is not a quality goal.
            assert!(matches!(
                rate(&reduced, &Budget::unlimited()),
                Err(RateError::MultipleSolutions | RateError::Unrated)
            ));
        }
    }
}
