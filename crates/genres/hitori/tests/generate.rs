use pencil_puzzles_hitori::*;

/// T3, T9: unique solutions, matching ratings, reproducible seeds, and
/// limits that only decide whether generation succeeds.
#[test]
fn generated_puzzles_have_exact_ratings_unique_solutions_and_repeatable_limits() {
    for shape in ["1x1", "1x3", "2x2", "3x4", "5x5", "4x7"] {
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
                        let kept = g.solution.get(row as usize, col as usize).to_char();
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
    let mut options = GenerateOptions::new("12x12".parse().unwrap(), 0);
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
    let ones = Puzzle::new("12x12".parse().unwrap(), vec![1; 144]).unwrap();
    assert_eq!(solve(&ones, &Budget::unlimited()).unwrap(), Solutions::None);
    let distinct: Vec<u32> = (0..144).map(|i| (i / 12 + i % 12) % 12 + 1).collect();
    let latin = Puzzle::new("12x12".parse().unwrap(), distinct).unwrap();
    assert!(matches!(
        solve(&latin, &Budget::new(1_000_000)),
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
    let mut wrong = Answer::new(g.puzzle.shape());
    let flipped = if g.solution.get(0, 0) == Mark::Shaded {
        Mark::Unshaded
    } else {
        Mark::Shaded
    };
    wrong.set(0, 0, flipped);
    assert!(matches!(
        hint(&g.puzzle, &wrong, &Budget::unlimited()),
        Err(HintError::InvalidAnswer(_))
    ));
    let mut o = GenerateOptions::new("1x1".parse().unwrap(), 0);
    o.max_attempts = 0;
    assert_eq!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 0 })
    );
    // A 1x2 grid never has exactly one solution.
    let mut o = GenerateOptions::new("1x2".parse().unwrap(), 0);
    o.max_attempts = 50;
    assert_eq!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 50 })
    );
    o.difficulty = Some(Difficulty::Extreme);
    assert!(matches!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::UnsupportedRequest(_))
    ));
}
