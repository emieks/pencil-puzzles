//! T8: real wasm32-unknown-unknown execution under Node.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use pencil_puzzles_aquarium::*;
use wasm_bindgen_test::wasm_bindgen_test;
#[wasm_bindgen_test]
fn largest_shape_solve_rate_generate_and_golden() {
    // One-cell tanks with a row one short of full have no solution.
    let cells: Vec<u32> = (0..225).collect();
    let mut rows = vec![15; 15];
    rows[0] = 14;
    let none = Puzzle::new("15x15".parse().unwrap(), &cells, rows, vec![15; 15]).unwrap();
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
