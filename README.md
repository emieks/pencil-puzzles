# pencil-puzzles

[![CI](https://github.com/emieks/pencil-puzzles/actions/workflows/ci.yml/badge.svg)](https://github.com/emieks/pencil-puzzles/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/pencil-puzzles)](https://pypi.org/project/pencil-puzzles/)

A Rust library for generating, solving, rating and checking pencil puzzles,
available as a Python package, a command-line tool and a
[web app](https://emieks.github.io/pencil-puzzles/) that runs in the browser.

Generated puzzles have exactly one solution, which a brute-force solver
checks. Difficulty goes from easy to expert and is the level of the hardest
technique a puzzle needs (for example, an X-wing in sudoku). The
same seed always gives the same puzzle, and hints show the next technique to
apply.

```python
>>> import pencil_puzzles as pp
>>> g = pp.generate("sudoku", (9, 9), difficulty="medium", seed=0)
>>> print(g.puzzle.to_text())
genre: sudoku
grid:
..1.....4
....49..1
.37...25.
...1.264.
9.......5
.425.6...
.68...49.
3..42....
2.....3..
>>> g.rating.level, g.rating.techniques
(<Difficulty.MEDIUM: 'medium'>, ('claiming', 'hidden_single'))
```

## Python

Python 3.10 or newer, on Windows, macOS and Linux:

```sh
pip install pencil-puzzles
```

```python
import pencil_puzzles as pp

print(pp.genres())                           # ('fillomino', 'kakurasu', 'nurikabe', 'sudoku')
g = pp.generate("nurikabe", (6, 6), difficulty="hard", seed=7)  # (rows, cols)
assert pp.solve(g.puzzle) == g.solution     # the only solution
assert pp.rate(g.puzzle).level == "hard"     # rated by its hardest technique
assert pp.check(g.puzzle, g.solution) == []  # breaks no rule
print(pp.hint(g.puzzle))                     # the first step to take
same = pp.Puzzle.from_id(g.puzzle.id)        # share a puzzle by its ID
```

The API is six functions (`genres`, `generate`, `solve`, `rate`, `check`,
`hint`) and the types they return; see [python/README.md](python/README.md).

## Genres

| Genre | Goal | Sizes | Generation time (median, easy → expert) |
| --- | --- | --- | --- |
| [Fillomino](docs/genres/fillomino.md) | Divide the grid into regions and write each region's size in its cells. | up to 10x10 | 6x6: 13 ms → 0.9 s |
| [Hashi](docs/genres/hashi.md) | Join the islands with bridges; each island has as many bridges as its number. | up to 15x15 | not measured yet |
| [Hitori](docs/genres/hitori.md) | Shade cells so no number repeats in a row or column; shaded cells never touch and the rest stay connected. | up to 12x12 | not measured yet |
| [Kakurasu](docs/genres/kakurasu.md) | Shade cells so each row and column adds up to its clue. | up to 32x32 | 8x8: 2 ms → 0.3 s |
| [Nurikabe](docs/genres/nurikabe.md) | Shade one connected sea; each number is the size of its island. | up to 8x8 | 6x6: 4 ms → 3.7 s |
| [Sudoku](docs/genres/sudoku.md) | Fill every row, column and box with each digit once. | 4x4, 9x9 | 9x9: 0.2 s → 2.7 s |

Each genre page lists its techniques per level and measured generation
times per size. See also [how puzzles are rated](docs/difficulty.md).

## Command line

From a clone, with Rust 1.88 or newer:

```sh
cargo install --path crates/cli
pencil-puzzles generate sudoku --shape 9x9 --difficulty hard --seed 1 --format pretty
pencil-puzzles generate kakurasu --shape 6x6 --count 3 --seed 7 > puzzles.txt
pencil-puzzles rate puzzles.txt --format json
pencil-puzzles hint --id kakurasu:2x2:1.2/1.2
```

`solve`, `rate`, `check` and `hint` read files, standard input or `--id`,
and puzzles can be piped between commands. Run `pencil-puzzles --help` for
everything else.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE).
