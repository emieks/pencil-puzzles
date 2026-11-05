//! T8: executes in Node on wasm32-unknown-unknown, including maximum-size work.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use pencil_puzzles_kakurasu::*;
use wasm_bindgen_test::wasm_bindgen_test;
#[wasm_bindgen_test]
fn size_limit_solve_rate_generate() {
    let puzzle = Puzzle::new(vec![0; 32], vec![0; 32]).unwrap();
    assert!(solve(&puzzle, &Budget::new(1_000_000)).unwrap().is_unique());
    assert_eq!(
        rate(&puzzle, &Budget::new(1_000_000)).unwrap().level,
        Difficulty::Easy
    );
    let options = GenerateOptions::new("32x32".parse().unwrap(), 0);
    assert_eq!(
        generate(&options, &Budget::new(100)),
        Err(GenerateError::BudgetExceeded)
    );
}
#[wasm_bindgen_test]
fn golden_ids_on_32_bits() {
    for line in include_str!("golden.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let f: Vec<_> = line.split_whitespace().collect();
        let mut options = GenerateOptions::new(f[0].parse().unwrap(), f[2].parse().unwrap());
        options.difficulty = Some(f[1].parse().unwrap());
        let g = generate(&options, &Budget::unlimited()).unwrap();
        assert_eq!(g.puzzle.id(), f[3]);
        assert_eq!(g.generator_version.to_string(), f[4]);
    }
}
