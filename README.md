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

print(pp.genres())                           # ('fillomino', 'kakurasu', 'nurikabe', ...)
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

| Genre | Goal | Sizes | Generation time (easy → expert) |
| --- | --- | --- | --- |
| [Fillomino](docs/genres/fillomino.md) | Divide the grid into regions whose cells hold the region's size; same-size regions never share a side. | ≤ 10x10 | 6x6: 13 ms → 950 ms |
| [Hashi](docs/genres/hashi.md) | Connect all islands with straight bridges that never cross, giving each island as many bridges as its number. | ≤ 15x15 | 7x7: 0.3 ms → 355 ms |
| [Hitori](docs/genres/hitori.md) | Shade cells so no unshaded number repeats in a row or column; shaded cells never share a side and unshaded cells stay connected. | ≤ 12x12 | 8x8: 1.6 ms → 558 ms |
| [Kakurasu](docs/genres/kakurasu.md) | Shade cells so each row and column clue is the sum of its shaded cells' positions. | ≤ 32x32 | 6x6: 0.4 ms → 307 ms |
| [Kakuro](docs/genres/kakuro.md) | Fill the white cells with digits 1–9 so each run adds up to its clue without repeating a digit. | ≤ 16x16 | 8x8: 9.2 ms → 222 ms |
| [Nurikabe](docs/genres/nurikabe.md) | Shade a connected sea, leaving islands the size of their numbers and no shaded 2×2 block. | ≤ 10x10 | 10x10: 125 ms → 1.3 s |
| [Sudoku](docs/genres/sudoku.md) | Fill rows, columns and boxes with each digit exactly once. | 4x4, 9x9 | 9x9: 14 ms → 147 ms |

Generation times are medians at each genre's default size. Each genre page
lists its techniques per level and measured generation times per size. See
also [how puzzles are rated](docs/difficulty.md).

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
