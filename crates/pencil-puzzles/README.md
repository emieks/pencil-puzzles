# pencil-puzzles

Generate, solve, rate and check pencil puzzles: aquarium, binairo, fillomino,
hashi, hitori, kakurasu, kakuro, nurikabe and sudoku. Every generated puzzle has exactly one
solution and a difficulty from easy to expert, set by the hardest human
technique it needs.

```sh
cargo add pencil-puzzles
```

```rust
use pencil_puzzles::sudoku::{self, GenerateOptions};
use pencil_puzzles::{Budget, Difficulty, Solutions};

let mut options = GenerateOptions::new("9x9".parse().unwrap(), 42);
options.difficulty = Some(Difficulty::Hard);
let generated = sudoku::generate(&options, &Budget::unlimited()).unwrap();

println!("{}", generated.puzzle);       // the puzzle in the text format
println!("{}", generated.puzzle.id());  // a one-line ID
println!("{:?}", generated.rating);     // level and techniques

match sudoku::solve(&generated.puzzle, &Budget::unlimited()).unwrap() {
    Solutions::Unique(answer) => assert_eq!(answer, generated.solution),
    _ => unreachable!(),
}
```

Each genre is a module with the same functions: `generate`, `solve`, `rate`,
`check` and `hint`. `genres()`, `find()` and `read()` work with any genre by
its id, and `read()` detects the genre of a text record, ID or JSON.

## Features

- `all-genres` (default): every genre.
- One feature per genre, such as `sudoku`. Turn off default features to
  build fewer genres.
- `serde`: serialization for puzzles, solutions and ratings.

## Links

- [API reference](https://docs.rs/pencil-puzzles)
- [Genre rules, techniques and formats](https://pencil-puzzles.org/docs/)
- [Command-line tool](https://crates.io/crates/pencil-puzzles-cli) and
  [Python package](https://pypi.org/project/pencil-puzzles/)
- [Source](https://github.com/emieks/pencil-puzzles)
