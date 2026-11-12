//! T8: real wasm32-unknown-unknown execution under Node.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use pencil_puzzles_binairo::*;
use wasm_bindgen_test::wasm_bindgen_test;
#[wasm_bindgen_test]
fn largest_shape_solve_rate_generate_and_golden() {
    // A 16x16 grid whose first row is all 1s has no solution.
    let shape: Shape = "16x16".parse().unwrap();
    let mut givens = vec![Mark::Unmarked; 256];
    givens[..16].fill(Mark::One);
    let none = Puzzle::new(shape, givens).unwrap();
    assert_eq!(solve(&none, &Budget::unlimited()).unwrap(), Solutions::None);
    assert!(rate(&none, &Budget::new(1)).is_err());
    assert!(generate(&GenerateOptions::new(Params::default(), 0), &Budget::new(1)).is_err());
    let mut o = GenerateOptions::new(Params::new(shape), 0);
    o.difficulty = Some(Difficulty::Expert);
    let g = generate(&o, &Budget::unlimited()).unwrap();
    assert_eq!(g.rating.level, Difficulty::Expert);
    for line in include_str!("golden.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let f: Vec<_> = line.split_whitespace().collect();
        let mut o = GenerateOptions::new(f[0].parse().unwrap(), f[2].parse().unwrap());
        o.difficulty = Some(f[1].parse().unwrap());
        let g = generate(&o, &Budget::unlimited()).unwrap();
        assert_eq!(g.puzzle.id(), f[3]);
        assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
        assert!(solve(&g.puzzle, &Budget::unlimited()).unwrap().is_unique());
    }
}
