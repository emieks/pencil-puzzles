# pencil-puzzles

[![PyPI](https://img.shields.io/pypi/v/pencil-puzzles)](https://pypi.org/project/pencil-puzzles/)
[![CI](https://github.com/emieks/pencil-puzzles/actions/workflows/ci.yml/badge.svg)](https://github.com/emieks/pencil-puzzles/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Generate, solve and rate pencil puzzles in Python.

Every puzzle has exactly one solution, and a brute-force solver proves it.
Its difficulty depends on how a person would solve it: the hardest technique
the puzzle needs decides its level. Ask for a hard sudoku and you get one that
needs a technique like an X-wing, not just one with fewer clues.

<p>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/images/genres-dark.png">
    <img alt="A puzzle in progress for each genre: sudoku, nurikabe, hashi, kakuro, fillomino, hitori and kakurasu" src="docs/images/genres-light.png">
  </picture>
</p>

**[Play them in your browser →](https://emieks.github.io/pencil-puzzles/)**

## Quick start

```sh
pip install pencil-puzzles
```

Python 3.10 or newer, on Windows, macOS and Linux.

Generate a hard sudoku and print it:

```python
import pencil_puzzles as pp

game = pp.generate("sudoku", difficulty="hard")
print(game.puzzle)
```

You get a new puzzle every time, such as this one:

```text
. . 3 | . 8 2 | . 5 .
6 . . | 5 . . | . 9 .
5 4 . | 3 . . | 8 . .
------+-------+------
. . . | . 5 . | 4 . .
8 . . | . . . | . . 6
. . 1 | . 6 . | . . .
------+-------+------
. . 4 | . . 8 | . 7 3
. 8 . | . . 4 | . . 1
. 3 . | 6 1 . | 2 . .
```

In code, clues and solutions are tuples of rows, with 0 for an empty cell:

```python
print(game.puzzle.clues[0])
# (0, 0, 3, 0, 8, 2, 0, 5, 0)

print(game.solution.cells[0])
# (7, 9, 3, 1, 8, 2, 6, 5, 4)
```

Other genres work the same way. [Each genre's page](#genres) shows its values.

## The library

Six functions cover everything:

```python
pp.generate(genre, shape=None, difficulty=None, seed=None)  # a new puzzle, its solution and rating
pp.solve(puzzle)                                            # the solution
pp.rate(puzzle)                                             # the level and the techniques it needs
pp.hint(puzzle, answer=None)                                # the next step from your answer so far
pp.check(puzzle, answer, finished=False)                    # every rule the answer breaks
pp.genres()                                                 # the genres you can use
```

- **`genre`**: a name from `pp.genres()`, such as `"sudoku"`
- **`shape`**: `(rows, columns)`, or the genre's [standard size](#genres) if
  left out
- **`difficulty`**: `"easy"`, `"medium"`, `"hard"` or `"expert"`, or any level
  if left out
- **`seed`**: a number that fixes the puzzle, or random if left out
- **`answer`**: a player's grid so far, as `pp.Solution(genre, rows)`

Save a puzzle with `puzzle.to_text()` or share it as `puzzle.id`. The
[Python guide](python/README.md) has every option.

## Genres

| Genre | Goal | Standard size | Largest |
| --- | --- | --- | --- |
| [Fillomino](docs/genres/fillomino.md) | Divide the grid into regions whose cells hold the region's size; same-size regions never share a side. | 10×10 | 10×10 |
| [Hashi](docs/genres/hashi.md) | Connect all islands with straight bridges that never cross, giving each island as many bridges as its number. | 10×10 | 15×15 |
| [Hitori](docs/genres/hitori.md) | Shade cells so no unshaded number repeats in a row or column; shaded cells never share a side and unshaded cells stay connected. | 8×8 | 12×12 |
| [Kakurasu](docs/genres/kakurasu.md) | Shade cells so each row and column clue is the sum of its shaded cells' positions. | 7×7 | 32×32 |
| [Kakuro](docs/genres/kakuro.md) | Fill the white cells with digits 1–9 so each run adds up to its clue without repeating a digit. | 10×10 | 16×16 |
| [Nurikabe](docs/genres/nurikabe.md) | Shade a connected sea, leaving islands the size of their numbers and no shaded 2×2 block. | 10×10 | 10×10 |
| [Sudoku](docs/genres/sudoku.md) | Fill rows, columns and boxes with each digit exactly once. | 9×9 | 9×9 (also&nbsp;4×4) |

Each genre's page has its rules, techniques and formats.

## How difficulty works

The solver works like a person: at each step it uses the simplest technique
that makes progress. The hardest technique it needs sets the level.

| Level | What it takes |
| --- | --- |
| Easy | Direct deductions from a single rule or clue |
| Medium | Standard combinations and small patterns |
| Hard | Larger patterns, or short "what if this cell were…" chains |
| Expert | Several patterns combined, and longer chains |

The full method is in [how puzzles are rated](docs/difficulty.md).

## Beyond Python

**In the browser.** The [web app](https://emieks.github.io/pencil-puzzles/)
lets you play or solve every genre.

**On the command line.** With Rust 1.88 or newer:

```sh
cargo install --git https://github.com/emieks/pencil-puzzles pencil-puzzles-cli
pencil-puzzles generate nurikabe --shape 7x7 --difficulty hard --format pretty
```

Run `pencil-puzzles --help` to see every command.

**In Rust.** Add it with `cargo add pencil-puzzles`. The engine is a Rust
workspace in [`crates/`](crates).

## Contributing

Contributions of every size are welcome. Start with
[CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
