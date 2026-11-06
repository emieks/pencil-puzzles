# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/). Changes that
alter generated puzzles are marked **(new puzzles)**, and changes that alter
ratings **(new ratings)**.

## [0.2.0] - 2026-11-06

### Added

- Fillomino (up to 10x10), hashi (up to 15x15), hitori (up to 12x12) and
  kakuro (up to 16x16): generate, solve, rate, check and hint.
- Web app: hashi bridges (click between two islands, or Shift with an arrow
  key), hitori numbers under the marks and kakuro blocks with their sums.
  Selecting a kakuro cell highlights its two runs.
- Web app: WASD to move, and Q, E and R to shade, dot and clear a cell.
- Python: `check(..., finished=True)` checks an answer as finished, with
  unmarked cells and undrawn bridges at the genre's default. Hashi steps list
  bridges as `(row, col, row2, col2, value)`.
- Rust: links between cells (`Place::Link`, `View.links`, `ViewLink`),
  `ViewCell.circled`, `ViewCell.label`, `ViewCell.block` with `BlockCell`,
  `ClueInput::text`, `Profile::contradictions_where`, the `grid` module
  (`neighbours`, `flood`, `flood_from`) and `Rng::shuffle`.

### Changed

- Nurikabe: generating without a size gives 6x6 puzzles (was 5x5).
  **(new puzzles)**
- Web app: click shades a cell and right-click dots it, where both used to
  cycle through the marks. Dots are larger.
- Web app: each request may use 50 million operations (was 2 million).

## [0.1.0] - 2026-11-05

### Added

- Sudoku, nurikabe and kakurasu: generate, solve, rate, check and hint.
- Difficulty from easy to expert, set by the hardest technique a puzzle needs.
- Python package, command-line tool and web app.
- Text, ID and JSON formats.
- Work budgets, attempt limits and cancellation.
- Corpus and benchmarks for every genre.
