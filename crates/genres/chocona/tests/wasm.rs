//! T8: real wasm32-unknown-unknown execution under Node.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use pencil_puzzles_chocona::*;
use wasm_bindgen_test::wasm_bindgen_test;
#[wasm_bindgen_test]
fn largest_shape_solve_rate_generate_and_golden() {
    // One-cell regions with three shaded cells around an unshaded one in a
    // corner have no solution.
    let cells: Vec<u32> = (0..225).collect();
    let mut clues = vec![Some(0); 225];
    for i in [0, 1, 15] {
        clues[i] = Some(1);
    }
    let none = Puzzle::new("15x15".parse().unwrap(), &cells, clues).unwrap();
    assert_eq!(solve(&none, &Budget::unlimited()).unwrap(), Solutions::None);
    assert!(rate(&none, &Budget::new(1)).is_err());
    assert!(generate(&GenerateOptions::new(Params::default(), 0), &Budget::new(1)).is_err());
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
