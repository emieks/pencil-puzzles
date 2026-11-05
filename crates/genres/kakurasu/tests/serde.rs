#![cfg(feature = "serde")]

use pencil_puzzles_kakurasu::{Answer, Budget, GenerateOptions, Puzzle, generate};
use serde_json::json;

#[test]
fn puzzle_json() {
    let puzzle = Puzzle::new(vec![1, 2], vec![1, 2]).unwrap();
    let value = json!({ "row_clues": [1, 2], "col_clues": [1, 2] });
    assert_eq!(serde_json::to_value(&puzzle).unwrap(), value);
    assert_eq!(serde_json::from_value::<Puzzle>(value).unwrap(), puzzle);
}

#[test]
fn invalid_puzzle_json_is_rejected() {
    let value = json!({ "row_clues": [9], "col_clues": [1] });
    assert!(serde_json::from_value::<Puzzle>(value).is_err());
}

#[test]
fn grid_json() {
    let grid: Answer = "#.\n.#".parse().unwrap();
    let value = json!(["#.", ".#"]);
    assert_eq!(serde_json::to_value(&grid).unwrap(), value);
    assert_eq!(serde_json::from_value::<Answer>(value).unwrap(), grid);
}

#[test]
fn rating_json() {
    let generated = generate(
        &GenerateOptions::new("5x5".parse().unwrap(), 0),
        &Budget::unlimited(),
    )
    .unwrap();
    let value = serde_json::to_value(&generated.rating).unwrap();
    assert_eq!(value["level"], generated.rating.level.as_str());
}
