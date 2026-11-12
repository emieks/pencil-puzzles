---
description: Documentation for pencil-puzzles, a Python package that generates, solves and rates sudoku, kakuro, nurikabe and other pencil puzzles.
---

# pencil-puzzles

pencil-puzzles generates, solves and rates pencil puzzles such as sudoku,
nurikabe and kakuro. It is a Python package with a Rust engine, a
command-line tool and a <a href="../">web app</a> for playing in the browser.

Every generated puzzle has exactly one solution, and a brute-force solver
proves it. A puzzle's difficulty is the level of the hardest technique a
person needs to solve it. An expert sudoku needs a technique such as an X-wing.

```sh
pip install pencil-puzzles
```

```python
import pencil_puzzles as pp

game = pp.generate("sudoku", difficulty="hard")
print(game.puzzle)
```

Start with the [quickstart](quickstart.md). The user guide covers each
part of the package, and the [API reference](api.md) lists every name.

## Genres

| Genre | Goal | Standard size | Largest |
| --- | --- | --- | --- |
| [Aquarium](genres/aquarium.md) | Fill tanks with water to match the row and column clues; the water in a tank is level. | 8×8 | 15×15 |
| [Fillomino](genres/fillomino.md) | Divide the grid into regions whose cells hold the region's size; same-size regions never share a side. | 10×10 | 10×10 |
| [Hashi](genres/hashi.md) | Connect all islands with straight bridges that never cross, giving each island as many bridges as its number. | 10×10 | 15×15 |
| [Hitori](genres/hitori.md) | Shade cells so no unshaded number repeats in a row or column; shaded cells never share a side and unshaded cells stay connected. | 8×8 | 12×12 |
| [Kakurasu](genres/kakurasu.md) | Shade cells so each row and column clue is the sum of its shaded cells' positions. | 7×7 | 32×32 |
| [Kakuro](genres/kakuro.md) | Fill the white cells with digits 1–9 so each run adds up to its clue without repeating a digit. | 10×10 | 16×16 |
| [Nurikabe](genres/nurikabe.md) | Shade a connected sea, leaving islands the size of their numbers and no shaded 2×2 block. | 10×10 | 10×10 |
| [Sudoku](genres/sudoku.md) | Fill rows, columns and boxes with each digit exactly once. | 9×9 | 9×9 (also&nbsp;4×4) |

Each genre's page has its rules, techniques, parameters and formats.

```{toctree}
:hidden:
:caption: Getting started

install
quickstart
```

```{toctree}
:hidden:
:caption: User guide

Generating <guide/generating>
Saving and loading <guide/saving>
Solving <guide/solving>
Rating <guide/rating>
Answers and hints <guide/answers>
Long-running calls <guide/long-running>
```

```{toctree}
:hidden:
:caption: Genres

genres/aquarium
genres/fillomino
genres/hashi
genres/hitori
genres/kakurasu
genres/kakuro
genres/nurikabe
genres/sudoku
```

```{toctree}
:hidden:
:caption: Reference

api
How difficulty works <difficulty>
benchmarks/README
changelog
```

```{toctree}
:hidden:
:caption: Links

Web app <https://pencil-puzzles.org/>
GitHub <https://github.com/emieks/pencil-puzzles>
PyPI <https://pypi.org/project/pencil-puzzles/>
```
