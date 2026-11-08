# Kakuro

Fill the white cells with digits 1–9 so each run adds up to its clue without
repeating a digit.

## Rules

The grid has white cells and blocks. A run is a line of white cells across
or down, as long as possible. The block just left of an across run holds its
sum, and the block just above a down run holds its sum. A block may hold an
across sum, a down sum, both or neither.

- **R1. Runs.** The digits in each run are all different and add up to its
  sum.
- **R2. Every cell filled.** Every white cell holds a digit from 1 to 9.

Every white cell is in an across run and a down run of 2 to 9 cells, and
every run has its sum. The first row and column hold only blocks.

## Example

Generator version 2, shape `5x5`, easy, seed `0` produces
`kakuro:5x5:HbBcAcBbB/7.5.8.3.3.6.8.6`:

```text
Puzzle                     Solution
  #   #   #  7/  5/        # # # # #
  #   # 8/3   .   .        # # # 1 2
  # 3/6   .   .   .        # # 1 2 3
 /8   .   .   .   #        # 1 3 4 #
 /6   .   .   #   #        # 2 4 # #
```

`7/` is a down sum of 7 and `/3` an across sum of 3. Two cells adding up to
3 hold 1 and 2, across in the second row and down below `3/`; the other
runs follow from there. `corpus/kakuro/` has puzzles from easy to expert, with
solutions.

## Variables

Every white cell is a variable, ordered row-major, with the digits 1 to 9 as
candidates. The combinations of a run are the sets of different digits with
as many digits as the run has cells and adding up to its sum. A cell starts
with the digits that are in some combination of each of its two runs. A cell
is fixed when one candidate is left.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each run | R1 | a fixed digit repeats, or no different digits unused in the run can fill its open cells to reach the rest of its sum |
| Each white cell | R2 | it has no candidate left |

With one candidate everywhere, the check is exactly "some run breaks R1".

## Techniques

Rater version 1 uses the profile `kakuro-human` version 1.

| Technique | Level | What it does |
| --- | --- | --- |
| `digit_used` | bookkeeping | A written-in digit leaves the other cells of its two runs. |
| `single` | Easy | A cell with one candidate is written in. |
| `sum_combinations` | Easy | A run's open cells keep only the digits of its combinations that contain its fixed digits. |
| `only_cell` | Easy | A digit that every such combination contains, and that only one open cell of the run allows, goes in that cell. |
| `combination_fit` | Medium | A combination survives only if its digits can go one per cell within the candidates; each cell keeps the digits it gets in some surviving way. This covers pairs and triples inside a run. |
| `short_contradiction` | Hard | If a cell with at most 3 candidates had a digit, Easy and Medium techniques would break a rule within 6 steps, so that digit goes. |
| `contradiction` | Expert | The same for cells with at most 4 candidates, within 20 steps, using techniques up to Hard. |

The contradiction techniques remove every refuted digit of a cell in one
step.

Examples (`?` is undecided):

```text
sum_combinations              combination_fit
 /12  9  ?  ?                  /23  ?   ?   ?
                                    89  89
 With 9 written in, the        23 in three cells is 6, 8 and
 other two add up to 3, so     9; two cells allow only 8 and
 they are 1 and 2.             9, so the third is 6.
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-16 | 10 |
| columns | yes | 1-16 | 10 |

The shape is written `<rows>x<columns>`, such as `8x8`, and counts the whole
grid, including the first row and column; a grid with fewer than 3 rows or
columns holds only blocks. Sums go from 1 to 45. Sums that no
digits can reach still parse and have no solution.

Generator version 2 starts from white cells everywhere below the first row
and right of the first column and makes cells blocks in pairs that are
symmetric under a half turn of that inner area. First every run longer than
the level's limit (4 cells up to medium and for any level, 5 at hard and
expert) is broken by a block inside it; then random cells become blocks
until the level's share of the inner cells is blocked (32% easy, 30% medium
and any level, 28% hard, 26% expert). A new block also blocks any white
cell left in a run of one cell; a placement is skipped if it would change
more than 6 cells, leave an inner row or column without white cells, or
split the white cells, and one that changes only the pair is preferred. If
no block can break a long run, the layout starts over; after 256 failed
layouts, runs one cell longer are allowed. Digits are filled in row-major
order by backtracking, each run preferring low or high digits at random,
more strictly at lower levels, never completing a 2x2 block of white cells
whose digits could be swapped; the layout starts over after 20,000 steps.

The sums follow from the digits. Each candidate is screened with the
techniques up to the requested level, at most Medium; if they do not solve
it, the brute-force solver looks for up to 4 solutions within the
candidates they left, trying at most 2,048 cells. While the puzzle has
several solutions or unresolved cells, cells change: every other time one
of the cells whose digits vary between the solutions found, otherwise any
unresolved cell; a cell gets a digit its runs do not have, chosen to leave
its runs the fewest combinations, or swaps digits with another cell of one
of its runs. One more cell changes for every 16 unresolved cells. A puzzle
solved below the requested level gets random digit changes. Above Medium,
the contradiction techniques run only once the brute-force solver has found
a single solution. A candidate starts over after 3 times as many changes
without improvement as it has inner cells, or 8 times as many once solved.

## Formats

```text
genre: kakuro
grid:
# 4/ 6/
/3 . .
/7 . .
```

Cells are separated by spaces: `.` for a white cell, `#` for a block without
sums, and the sums around a `/`, down sum first: `4/6` is down 4 and across
6, `4/` down only, `/6` across only. A grid without sums may use one
character per cell. JSON:

```json
{"genre":"kakuro","puzzle":{"grid":["# 4/ 6/","/3 . .","/7 . ."]}}
```

IDs list the layout row-major, an uppercase letter for a run of blocks and
a lowercase letter for a run of white cells (`A` or `a` is one cell, `Z` or
`z` is 26), then `/` and the sums separated by `.`, in the order of their
blocks row-major with the down sum first, such as
`kakuro:3x3:DbAb/4.6.3.7`.

Answers have one character per cell: `#` for every block, `.` for unmarked,
`0` for a cell left empty and `1`-`9` for a digit. Finishing turns unmarked
cells into empty ones, which breaks R2. Violations of R1 are placed at the
block holding the run's sum and carry `down` (1 for a down run, 0 for an
across run), `sum`, `total` (the run's digits added up) and, when a digit
repeats, `repeated`. Violations of R2 are placed at the cell and carry
nothing.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- kakuro 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`kakuro.csv`](../benchmarks/kakuro.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 6x6 | any | 20/20 | 12 ms | 61 ms | 96 |
| 6x6 | easy | 20/20 | 7.0 ms | 58 ms | 134 |
| 6x6 | medium | 20/20 | 14 ms | 54 ms | 116 |
| 6x6 | hard | 20/20 | 39 ms | 310 ms | 391 |
| 6x6 | expert | 20/20 | 163 ms | 1.1 s | 1115 |
| 8x8 | any | 20/20 | 12 ms | 122 ms | 40 |
| 8x8 | easy | 20/20 | 9.2 ms | 36 ms | 44 |
| 8x8 | medium | 20/20 | 10 ms | 104 ms | 35 |
| 8x8 | hard | 20/20 | 33 ms | 135 ms | 105 |
| 8x8 | expert | 20/20 | 222 ms | 757 ms | 306 |
| 10x10 | any | 20/20 | 18 ms | 259 ms | 41 |
| 10x10 | easy | 20/20 | 14 ms | 158 ms | 91 |
| 10x10 | medium | 20/20 | 23 ms | 193 ms | 47 |
| 10x10 | hard | 20/20 | 154 ms | 780 ms | 254 |
| 10x10 | expert | 20/20 | 377 ms | 2.8 s | 404 |
| 12x12 | any | 20/20 | 110 ms | 1.8 s | 166 |
| 12x12 | easy | 20/20 | 35 ms | 793 ms | 196 |
| 12x12 | medium | 20/20 | 48 ms | 1.0 s | 116 |
| 12x12 | hard | 20/20 | 687 ms | 3.4 s | 480 |
| 12x12 | expert | 19/20 | 1.8 s | — | 721 |
| 16x16 | any | 11/20 | 3.6 s | — | 192 |
| 16x16 | easy | 12/20 | 3.0 s | — | 594 |
| 16x16 | medium | 13/20 | 1.8 s | — | 207 |
| 16x16 | hard | 8/20 | — | — | 213 |
| 16x16 | expert | 5/20 | — | — | 226 |

Largest practical size (B1): easy 12x12, medium 12x12, hard 12x12, expert
10x10. Rating a corpus puzzle takes at most 9.5 ms. In WebAssembly
([`web/benchmark.mjs`](../../web/benchmark.mjs), 8x8), generation took 0.87
times the native time.

## References

Kakuro first appeared in Dell Magazines in the United States as Cross Sums
(`cross-sums`); Nikoli named it kakuro.
