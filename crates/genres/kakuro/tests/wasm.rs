//! T8: real wasm32-unknown-unknown execution under Node.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use pencil_puzzles_kakuro::*;
use wasm_bindgen_test::wasm_bindgen_test;
#[wasm_bindgen_test]
fn largest_shape_solve_rate_generate_and_golden() {
    // The largest grid with every inner cell white has runs too long to
    // be valid, so the largest corpus puzzle stands in for it.
    let largest = include_str!("../../../../corpus/kakuro/largest.txt");
    let empty = parse_records(largest).unwrap().remove(0).puzzle;
    assert!(solve(&empty, &Budget::new(1)).is_err());
    assert!(rate(&empty, &Budget::new(1)).is_err());
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
