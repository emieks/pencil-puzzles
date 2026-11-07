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
    let mut options = GenerateOptions::new("16x16".parse().unwrap(), 5);
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

/// Layouts have a half turn symmetry, no inner row or column without white
/// cells, and connected white cells.
#[test]
fn layouts_are_symmetric_full_and_connected() {
    for shape in ["5x5", "5x8", "6x6", "8x8", "10x10"] {
        for seed in 0..4 {
            let g = generate(
                &GenerateOptions::new(shape.parse().unwrap(), seed),
                &Budget::new(100_000_000),
            )
            .unwrap();
            let p = &g.puzzle;
            let (rows, cols) = (p.shape().rows(), p.shape().cols());
            let white = |r: usize, c: usize| p.cell(r, c) == Cell::White;
            for r in 1..rows {
                assert!((1..cols).any(|c| white(r, c)), "{} row {r}", p.id());
                for c in 1..cols {
                    assert_eq!(white(r, c), white(rows - r, cols - c), "{}", p.id());
                }
            }
            for c in 1..cols {
                assert!((1..rows).any(|r| white(r, c)), "{} column {c}", p.id());
            }
            let whites: Vec<(usize, usize)> = (1..rows)
                .flat_map(|r| (1..cols).map(move |c| (r, c)))
                .filter(|&(r, c)| white(r, c))
                .collect();
            let mut seen = vec![whites[0]];
            let mut at = 0;
            while at < seen.len() {
                let (r, c) = seen[at];
                at += 1;
                for next in [(r + 1, c), (r - 1, c), (r, c + 1), (r, c - 1)] {
                    if whites.contains(&next) && !seen.contains(&next) {
                        seen.push(next);
                    }
                }
            }
            assert_eq!(seen.len(), whites.len(), "{} is not connected", p.id());
        }
    }
}
