use pencil_puzzles_chocona::*;

/// T3, T9: unique solutions, matching ratings, reproducible seeds, and
/// limits that only decide whether generation succeeds.
#[test]
fn generated_puzzles_have_exact_ratings_unique_solutions_and_repeatable_limits() {
    for shape in ["1x1", "1x3", "2x2", "3x4", "5x5", "4x7", "7x3"] {
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
    for (shape, level) in [
        ("6x6", Difficulty::Easy),
        ("6x6", Difficulty::Medium),
        ("6x6", Difficulty::Hard),
        ("5x8", Difficulty::Easy),
        ("5x8", Difficulty::Medium),
        ("5x8", Difficulty::Hard),
        ("8x8", Difficulty::Expert),
    ] {
        let mut o = GenerateOptions::new(shape.parse().unwrap(), 1);
        o.difficulty = Some(level);
        let g = generate(&o, &Budget::new(100_000_000)).unwrap();
        assert_eq!(g.rating.level, level);
        assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
        assert_eq!(generate(&o, &Budget::new(100_000_000)).unwrap(), g);
    }
}

#[test]
fn maximum_size_work_and_hints_are_bounded() {
    let mut options = GenerateOptions::new("15x15".parse().unwrap(), 0);
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
    // One-cell regions with three shaded cells around an unshaded one in
    // a corner have no solution. One region without a number has many.
    let shape: Shape = "15x15".parse().unwrap();
    let cells: Vec<u32> = (0..225).collect();
    let mut clues = vec![Some(0); 225];
    for i in [0, 1, 15] {
        clues[i] = Some(1);
    }
    let none = Puzzle::new(shape, &cells, clues).unwrap();
    assert_eq!(solve(&none, &Budget::unlimited()).unwrap(), Solutions::None);
    let many = Puzzle::new(shape, &[0; 225], vec![None]).unwrap();
    assert!(matches!(
        solve(&many, &Budget::new(1_000_000)),
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
    o.difficulty = Some(Difficulty::Extreme);
    assert!(matches!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::UnsupportedRequest(_))
    ));
    for (shape, level) in [
        ("3x4", Difficulty::Expert),
        ("2x8", Difficulty::Expert),
        ("2x4", Difficulty::Hard),
        ("1x9", Difficulty::Medium),
        ("2x2", Difficulty::Medium),
    ] {
        let mut o = GenerateOptions::new(shape.parse().unwrap(), 0);
        o.difficulty = Some(level);
        assert!(matches!(
            generate(&o, &Budget::unlimited()),
            Err(GenerateError::UnsupportedRequest(_))
        ));
    }
}

/// Hints after partial marks: the hint is the next skill step, and
/// following hints finishes the puzzle.
#[test]
fn hints_continue_from_partial_marks() {
    let mut o = GenerateOptions::new("6x6".parse().unwrap(), 3);
    o.difficulty = Some(Difficulty::Medium);
    let g = generate(&o, &Budget::unlimited()).unwrap();
    let mut a = Answer::new(g.puzzle.shape());
    let mut hints = 0;
    while let Hint::Step(step) = hint(&g.puzzle, &a, &Budget::unlimited()).unwrap() {
        hints += 1;
        assert!(hints < 200, "hints do not finish the puzzle");
        assert!(!step.removed.is_empty());
        for lit in step.removed {
            let Place::Cell { row, col } = lit.place else {
                panic!("a hint names cells");
            };
            let (r, c) = (row as usize, col as usize);
            assert_ne!(lit.value, g.solution.get(r, c).to_char().to_string());
            a.set(r, c, g.solution.get(r, c));
        }
    }
    assert!(hints > 0);
    assert_eq!(a.finished(), g.solution);
}
