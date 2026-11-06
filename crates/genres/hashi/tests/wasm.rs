//! T8: real wasm32-unknown-unknown execution under Node.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use pencil_puzzles_hashi::*;
use wasm_bindgen_test::wasm_bindgen_test;
#[wasm_bindgen_test]
fn largest_shape_solve_rate_generate_and_golden() {
    let crowded: Vec<u8> = (0..225)
        .map(|i| if (i / 15 + i % 15) % 2 == 0 { 8 } else { 0 })
        .collect();
    let crowded = Puzzle::new("15x15".parse().unwrap(), crowded).unwrap();
    assert!(solve(&crowded, &Budget::new(1)).is_err());
    assert!(rate(&crowded, &Budget::new(1)).is_err());
    assert!(solve(&crowded, &Budget::unlimited()).is_ok());
    assert!(
        generate(
            &GenerateOptions::new("15x15".parse().unwrap(), 0),
            &Budget::new(1)
        )
        .is_err()
    );
    for line in include_str!("golden.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let f: Vec<_> = line.split_whitespace().collect();
        let mut o = GenerateOptions::new(f[0].parse().unwrap(), f[2].parse().unwrap());
        o.difficulty = Some(f[1].parse().unwrap());
        let g = generate(&o, &Budget::unlimited()).unwrap();
        assert_eq!(g.puzzle.id(), f[3]);
        assert_eq!(g.generator_version.to_string(), f[4]);
        assert_eq!(rate(&g.puzzle, &Budget::unlimited()).unwrap(), g.rating);
        assert!(solve(&g.puzzle, &Budget::unlimited()).unwrap().is_unique());
    }
}
