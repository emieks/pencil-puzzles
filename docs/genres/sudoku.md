# Sudoku

Fill rows, columns and boxes with each digit exactly once.

## Rules

- **R1. Rows.** Each row contains every digit from 1 to the grid size exactly once.
- **R2. Columns.** Each column contains every digit exactly once.
- **R3. Boxes.** Each square box contains every digit exactly once.
- **R4. Givens.** Printed digits cannot be changed.

## Example

Shape `4x4`, easy, seed `0` produces `sudoku:4x4:....2.3..3.4....`:

```text
Puzzle    Solution
....      3142
2.3.      2431
.3.4      1324
....      4213
```

`corpus/sudoku/` has this puzzle and larger ones, with solutions.

## Variables

Each empty cell is a variable whose candidates are the digits 1 to the grid
size. Printed cells are fixed. Variables are ordered row-major, and digits in
ascending order. R4 holds from the start, so the rater does not check it.

## Rule instances

Rows, columns and boxes (R1-R3), in that order. An instance is broken when a
cell has no candidates left or a unit can no longer hold every digit. The
brute-force solver uses all-different matching per unit (O(N^3) for N cells);
the rater does not use it.

## Techniques

The rater uses the profile `sudoku-human`, in this order:

| Level | Techniques |
| --- | --- |
| bookkeeping | `peer_elimination`: a written-in digit (given, player mark or found single) is removed from the candidates of its row, column and box |
| Easy | `hidden_single` (the only place for a digit in a unit), `naked_single` (a cell left with one candidate by elimination) |
| Medium | `pointing` and `claiming` (locked candidates), `naked_pair` |
| Hard | `hidden_pair`, naked and hidden triples and quads |
| Expert | `x_wing`, `swordfish`, `skyscraper`, `two_string_kite`, `turbot_fish`, `xy_wing`, `xyz_wing`, `w_wing`, `simple_coloring` (components of up to 6 cells), `jellyfish`, `extended_coloring` (up to 12 cells), `x_chain` (up to 3 strong links), `xy_chain` (up to 6 cells) |

Examples:

- **Hidden single:** if a row's only cell that can still hold 4 is its last
  empty cell, that cell is 4.
- **Naked single:** a cell whose row, column and box already contain every
  digit but 7 is 7.
- **Pointing:** if a box's only places for 5 lie in one row, no other cell
  of that row can be 5.
- **Naked pair:** if two cells of a column can each only be 1 or 2, no other
  cell of that column can be 1 or 2.
- **Hidden pair:** if 6 and 8 can only go in the same two cells of a row,
  those two cells lose every other candidate.
- **X-wing:** if 3 can only go in columns 2 and 6 in both row 1 and row 8,
  it is in those columns in those rows, so the rest of columns 2 and 6 lose 3.
- **XY-chain:** a chain of two-candidate cells, each seeing the next, that
  starts and ends with the same free digit z: one end is z, so any cell
  seeing both ends loses z.

`locked_candidates` reports its steps as `pointing` or `claiming`, and
`turbot_family` as `skyscraper`, `two_string_kite` or `turbot_fish`. A cell
that is both a hidden and a naked single counts as hidden. No technique
assumes the puzzle is unique, so there are no unique rectangles. General
AIC, finned fish, ALS and forcing chains are not implemented; puzzles that
need them are unrated.

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| size | yes | 1x1, 4x4, 9x9 | 9x9 |

Boxes are 1x1, 2x2 and 3x3 respectively. 6x6 grids with 2x3 boxes are not
supported yet.

The generator fills a grid in seeded order, then removes clues while the
puzzle stays unique and solvable at the requested level. Below expert it
removes clues in 180-degree rotational pairs; expert removes single clues.
Each removal is checked for uniqueness with the brute-force solver before the
techniques run. When every clue has been tried and the puzzle is still too
easy, 4-8 clues (or pairs) go back into the best candidate so far (by level,
then score) and removal starts over in a new order, up to 8 rounds before a
new grid. In the result, removing any further clue (or pair, below expert)
would change the level or the number of solutions.

## Formats

```text
genre: sudoku
grid:
1..4
.4..
..4.
4..1
```

`grid:` has one character per cell: a digit for a given and `.` for a blank.
Readers also accept `0` for a blank.

An ID is `sudoku:<shape>:<row-major grid>`, for example
`sudoku:4x4:1..4.4....4.4..1`. JSON:

```json
{"genre":"sudoku","puzzle":{"grid":["1..4",".4..","..4.","4..1"]}}
```

Answers have one row per line, or a JSON array of row strings. `.` is
unmarked and `0` is explicitly empty; finishing turns unmarked cells into
empty ones, which break their units. A solution contains only digits.

R1-R3 violations carry `filled` and `size`; R4 carries `given` and `actual`.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- sudoku 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`sudoku.csv`](../benchmarks/sudoku.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 9x9 | any | 20/20 | 15 ms | 27 ms | 42 |
| 9x9 | easy | 20/20 | 14 ms | 16 ms | 42 |
| 9x9 | medium | 20/20 | 50 ms | 302 ms | 219 |
| 9x9 | hard | 20/20 | 361 ms | 2.6 s | 1907 |
| 9x9 | expert | 20/20 | 63 ms | 239 ms | 209 |

Largest practical size (B1): easy 9x9, medium 9x9, hard 9x9, expert 9x9.
Rating a corpus puzzle takes at most 3.9 ms. In WebAssembly
([`web/benchmark.mjs`](https://github.com/emieks/pencil-puzzles/blob/main/web/benchmark.mjs), 9x9), generation took 1.03
times the native time. Memory is O(N^4) for N cells.

## References

Also known as Number Place (`number-place`) and Nanpure (`nanpure`). See
[Nikoli's history of the puzzle](https://www.nikoli.co.jp/en/puzzles/sudoku/).
