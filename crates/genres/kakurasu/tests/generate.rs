use pencil_puzzles_kakurasu::{
    Budget, Difficulty, GenerateError, GenerateOptions, Solutions, generate, rate, solve,
};

#[test]
fn generated_puzzles_are_unique_reproducible_and_truly_rated() {
    for shape in ["1x1", "1x5", "2x3", "5x5", "6x8", "8x8"] {
        for seed in 0..5 {
            let options = GenerateOptions::new(shape.parse().unwrap(), seed);
            let a = generate(&options, &Budget::new(20_000_000)).unwrap();
            let b = generate(&options, &Budget::new(20_000_000)).unwrap();
            assert_eq!(a, b);
            assert_eq!(
                solve(&a.puzzle, &Budget::unlimited()).unwrap(),
                Solutions::Unique(a.solution)
            );
            assert_eq!(rate(&a.puzzle, &Budget::unlimited()).unwrap(), a.rating);
            assert!(a.rating.level < Difficulty::Extreme);
        }
    }
}

#[test]
fn attempts_and_budgets_are_limits_not_choices() {
    let mut options = GenerateOptions::new("1x1".parse().unwrap(), 0);
    options.difficulty = Some(Difficulty::Extreme);
    assert!(matches!(
        generate(&options, &Budget::unlimited()),
        Err(GenerateError::UnsupportedRequest(_))
    ));
    // No 1x1 puzzle is Hard.
    options.difficulty = Some(Difficulty::Hard);
    options.max_attempts = 5;
    assert_eq!(
        generate(&options, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 5 })
    );
    options.max_attempts = 0;
    assert_eq!(
        generate(&options, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 0 })
    );
    // Every level the profile defines (T3), on a non-square grid.
    for level in [
        Difficulty::Easy,
        Difficulty::Medium,
        Difficulty::Hard,
        Difficulty::Expert,
    ] {
        let mut o = GenerateOptions::new("6x8".parse().unwrap(), 0);
        o.difficulty = Some(level);
        let g = generate(&o, &Budget::unlimited()).unwrap();
        assert_eq!(g.rating.level, level);
        assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
        assert_eq!(
            solve(&g.puzzle, &Budget::unlimited()).unwrap(),
            Solutions::Unique(g.solution)
        );
    }
    options = GenerateOptions::new("6x6".parse().unwrap(), 0);
    let expected = generate(&options, &Budget::unlimited()).unwrap();
    for limit in [0, 1, 10, 100, 1000] {
        match generate(&options, &Budget::new(limit)) {
            Ok(got) => assert_eq!(got, expected),
            Err(error) => assert_eq!(error, GenerateError::BudgetExceeded),
        }
    }
    options.max_attempts = expected.attempts;
    assert_eq!(generate(&options, &Budget::unlimited()).unwrap(), expected);
}

#[test]
fn size_limit_operations_obey_budgets() {
    let puzzle = pencil_puzzles_kakurasu::Puzzle::new(vec![0; 32], vec![0; 32]).unwrap();
    for limit in [0, 1, 64, 1000] {
        let budget = Budget::new(limit);
        if let Ok(result) = solve(&puzzle, &budget) {
            assert!(result.is_unique());
        }
        let budget = Budget::new(limit);
        match rate(&puzzle, &budget) {
            Ok(rating) => assert_eq!(rating.level, Difficulty::Easy),
            Err(error) => assert_eq!(error, pencil_puzzles_kakurasu::RateError::BudgetExceeded),
        }
        let options = GenerateOptions::new("32x32".parse().unwrap(), 0);
        assert_eq!(
            generate(&options, &Budget::new(limit)),
            Err(GenerateError::BudgetExceeded)
        );
    }
}
