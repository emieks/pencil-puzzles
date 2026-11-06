# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/). Changes that
alter generated puzzles are marked **(new puzzles)**, and changes that alter
ratings **(new ratings)**.

## [Unreleased]

### Added

- Fillomino: generate, solve, rate, check and hint, for
  grids up to 10x10.
- Hashi: generate, solve, rate, check and hint, for grids up to 15x15.
- Hitori: generate, solve, rate, check and hint, for grids up to 12x12.
- Links between cells in steps, violations and the web app's boards
  (`Place::Link`, `View.links`). The web app draws them. Players click or
  right-click a link to change it, or select a cell at its end and press
  Shift with an arrow key or WASD.
- `ViewCell.label`: text printed in a cell under its mark. The web app draws
  it, circles it for the unshaded mark and highlights cells with the same
  label as the selected one.
- Python: `check(..., finished=True)` checks an answer as finished, with
  unmarked cells and undrawn bridges at the genre's default. Steps that
  remove Hashi bridge counts list them as `(row, col, row2, col2, value)`.

## [0.1.0] - 2026-11-05

### Added

- Sudoku, nurikabe and kakurasu: generate, solve, rate, check and hint.
- Difficulty from easy to expert, set by the hardest technique a puzzle needs.
- Python package, command-line tool and web app.
- Text, ID and JSON formats.
- Work budgets, attempt limits and cancellation.
- Corpus and benchmarks for every genre.
