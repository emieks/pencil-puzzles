# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/). Changes that
alter generated puzzles are marked **(new puzzles)**, and changes that alter
ratings **(new ratings)**.

## [Unreleased]

### Added

- `llms.txt` at https://pencil-puzzles.org/llms.txt, describing the package
  and linking to the documentation.

### Changed

- Web app: new layout. The list of genres shows an example board for each
  genre. Puzzle pages keep the title, board and controls in the same place
  on every genre and in both modes, and the rules, keyboard shortcuts and
  loading a puzzle are in the side column.
- Web app: pages change without reloading.
- Web app: New puzzle asks for a second press once a game has moves.
- Web app: sudoku offers 4x4 and 9x9 with no custom size.
- Web app and documentation: new site icon, page titles and descriptions,
  link previews and sitemaps.

## [0.4.0] - 2026-11-09

### Added

- Documentation site at https://pencil-puzzles.org/docs/.

### Changed

- Sudoku: levels follow common sudoku grading, with X-wing and similar
  techniques at expert. **(new ratings)** **(new puzzles)**
- The web app moved to https://pencil-puzzles.org/.

## [0.3.1] - 2026-11-08

### Added

- Python: `print(puzzle)` shows the grid.
- Python: `generate` without a shape gives the genre's standard size.
- Python: every puzzle has `clues`; kakurasu's is a `KakurasuClues` with
  `rows` and `cols`.
- Rust: the crates are on crates.io.

### Changed

- Generating without a size gives hashi and kakuro 10x10 puzzles (were 7x7
  and 8x8), kakurasu 7x7 (was 6x6) and fillomino 10x10 (was 6x6).
  **(new puzzles)**
- Python: `generate` without a seed picks a random one, kept in
  `Generated.seed` (was 0).
- Sudoku: box lines cross with `+` when printed, also in the command-line
  tool's `--format pretty`.
- README rewritten around the Python package.

### Deprecated

- Python: `givens`, `numbers`, `cells`, `row_clues` and `col_clues`; use
  `clues`.

## [0.3.0] - 2026-11-08

### Added

- Nurikabe: grids up to 10x10 (was 8x8).
- Web app: per-puzzle saves with undo history across reloads, Show mistakes,
  Show solution and Pause, difficulty next to the timer and size presets.
- Web app: selecting several cells at once, keyboard input, kakuro
  combinations for the selected run and easier hashi bridges.
- Rust: `Candidates.first_only`.

### Changed

- Nurikabe: new generator, and solving and rating on bitboards. Generation is
  ~463% faster on average. **(new puzzles)**
- Nurikabe: generating without a size gives 10x10 puzzles (was 6x6).
  **(new puzzles)**
- Sudoku: new generator and rating on bitboards. Generation is ~1791% faster
  on average. **(new puzzles)**
- Kakuro: new generator with symmetric layouts that finds more puzzles at
  12x12 and 16x16. Generation is ~11% slower on average at the sizes where
  both versions find every puzzle. **(new puzzles)**
- Web app: each request may use 100 million operations (was 50 million).
- Genre summaries and rule texts read the same in the library, docs and
  README.

### Fixed

- Web app: highlighting follows each genre's rules.

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
