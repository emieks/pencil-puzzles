use pencil_puzzles_core::Genre;
use pencil_puzzles_hashi::*;

/// T3, T9: unique solutions, matching ratings, reproducible seeds, and
/// limits that only decide whether generation succeeds.
#[test]
fn generated_puzzles_have_exact_ratings_unique_solutions_and_repeatable_limits() {
    for shape in ["1x1", "1x5", "2x3", "4x3", "5x5", "7x7"] {
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
            let view = Hashi::view(&g.puzzle, Some(&g.solution));
            for step in rate_with_steps(&g.puzzle, &Budget::unlimited()).unwrap().1 {
                for lit in step.removed {
                    let link = view.links.iter().find(|l| l.place == lit.place).unwrap();
                    let solved = if link.value == "." { "0" } else { &link.value };
                    assert_ne!(lit.value, solved);
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
    for shape in ["7x7", "6x9"] {
        for level in [
            Difficulty::Easy,
            Difficulty::Medium,
            Difficulty::Hard,
            Difficulty::Expert,
        ] {
            let mut o = GenerateOptions::new(shape.parse().unwrap(), 1);
            o.difficulty = Some(level);
            let g = generate(&o, &Budget::new(500_000_000)).unwrap();
            assert_eq!(g.rating.level, level);
            assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
            assert_eq!(generate(&o, &Budget::new(500_000_000)).unwrap(), g);
        }
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
    // Every other cell an 8: many slots, no solution.
    let crowded: Vec<u8> = (0..225)
        .map(|i| if (i / 15 + i % 15) % 2 == 0 { 8 } else { 0 })
        .collect();
    let crowded = Puzzle::new("15x15".parse().unwrap(), crowded).unwrap();
    assert!(matches!(
        solve(&crowded, &Budget::new(1_000_000)),
        Ok(Solutions::None) | Err(BudgetExceeded)
    ));
    let empty = Hashi::empty_answer(&g.puzzle);
    assert!(matches!(
        hint(&g.puzzle, &empty, &Budget::new(100_000_000)).unwrap(),
        Hint::Step(_)
    ));
    assert_eq!(
        hint(&g.puzzle, &g.solution, &Budget::unlimited()).unwrap(),
        Hint::Solved
    );
    // A wrong bridge: one more or one less than the solution's first.
    let view = Hashi::view(&g.puzzle, Some(&g.solution));
    let mut marks: Vec<String> = view.cells.iter().map(|c| c.value.clone()).collect();
    marks.extend(view.links.iter().map(|l| l.value.clone()));
    let first = view.cells.len();
    marks[first] = if marks[first] == "1" { "2" } else { "1" }.into();
    let wrong = Hashi::answer_from_values(&g.puzzle, &marks).unwrap();
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

/// Grids too small for hard puzzles give up quickly.
#[test]
fn tiny_grids_fail_fast() {
    let mut o = GenerateOptions::new("3x3".parse().unwrap(), 0);
    o.difficulty = Some(Difficulty::Expert);
    assert_eq!(
        generate(&o, &Budget::unlimited()),
        Err(GenerateError::AttemptsExhausted { attempts: 10_000 })
    );
}
