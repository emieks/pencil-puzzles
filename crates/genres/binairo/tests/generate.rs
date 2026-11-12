use pencil_puzzles_binairo::*;

/// T3, T9: unique solutions, matching ratings, reproducible seeds, and
/// limits that only decide whether generation succeeds.
#[test]
fn generated_puzzles_have_exact_ratings_unique_solutions_and_repeatable_limits() {
    for shape in ["2x2", "4x4", "4x6", "6x6", "6x8", "8x6"] {
        for seed in 0..4 {
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
        ("4x4", Difficulty::Easy),
        ("4x4", Difficulty::Medium),
        ("4x6", Difficulty::Hard),
        ("6x4", Difficulty::Expert),
        ("6x10", Difficulty::Easy),
        ("6x10", Difficulty::Medium),
        ("6x10", Difficulty::Hard),
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
    let mut options = GenerateOptions::new("16x16".parse().unwrap(), 0);
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
    // An empty grid has many solutions; one with a 1 given in every cell
    // of the first row has none.
    let shape: Shape = "16x16".parse().unwrap();
    let mut givens = vec![Mark::Unmarked; 256];
    let many = Puzzle::new(shape, givens.clone()).unwrap();
    assert!(matches!(
        solve(&many, &Budget::new(1_000_000)),
        Ok(Solutions::Multiple(..)) | Err(BudgetExceeded)
    ));
    givens[..16].fill(Mark::One);
    let none = Puzzle::new(shape, givens).unwrap();
    assert_eq!(solve(&none, &Budget::unlimited()).unwrap(), Solutions::None);
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
    let open = (0..256)
        .find(|&i| g.puzzle.givens()[i] == Mark::Unmarked)
        .unwrap();
    let mut wrong = Answer::new(g.puzzle.shape());
    let flipped = if g.solution.get(open / 16, open % 16) == Mark::One {
        Mark::Zero
    } else {
        Mark::One
    };
    wrong.set(open / 16, open % 16, flipped);
    assert!(matches!(
        hint(&g.puzzle, &wrong, &Budget::unlimited()),
        Err(HintError::InvalidAnswer(_))
    ));
    let mut o = GenerateOptions::new("2x2".parse().unwrap(), 0);
    o.max_attempts = 0;
    assert_eq!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 0 })
    );
    for (shape, level) in [
        ("2x2", Difficulty::Extreme),
        ("2x2", Difficulty::Medium),
        ("4x4", Difficulty::Hard),
        ("4x4", Difficulty::Expert),
    ] {
        let mut o = GenerateOptions::new(shape.parse().unwrap(), 0);
        o.difficulty = Some(level);
        assert!(matches!(
            generate(&o, &Budget::unlimited()),
            Err(GenerateError::UnsupportedRequest(_))
        ));
    }
}

/// Hints after partial marks: each hint's digits are written in, and the
/// hints finish the puzzle.
#[test]
fn hints_continue_from_partial_marks() {
    let mut o = GenerateOptions::new("8x8".parse().unwrap(), 3);
    o.difficulty = Some(Difficulty::Hard);
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
    for r in 0..8 {
        for c in 0..8 {
            if g.puzzle.given(r, c) == Mark::Unmarked {
                assert_eq!(a.get(r, c), g.solution.get(r, c));
            }
        }
    }
    assert!(check(&g.puzzle, &a).unwrap().is_empty());
}
