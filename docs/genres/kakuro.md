# Kakuro

Write a digit in every white cell so that each run of white cells adds up
to the sum in the block before it.

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

Generator version 1, shape `5x5`, easy, seed `0` produces
`kakuro:5x5:FbCcCbF/8.19.4.3.17.9`:

```text
Puzzle                     Solution
  #  8/ 19/   #   #        # # # # #
 /4   .   .  3/   #        # 1 3 # #
/17   .   .   .   #        # 7 9 1 #
  #  /9   .   .   #        # # 7 2 #
  #   #   #   #   #        # # # # #
```

`8/` is a down sum of 8 and `/4` an across sum of 4. Two cells adding up to
4 hold 1 and 3, and two adding up to 3 hold 1 and 2; the other runs follow
from there. `corpus/kakuro/` has puzzles from easy to expert, with
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
| rows | yes | 1-16 | 8 |
| columns | yes | 1-16 | 8 |

The shape is written `<rows>x<columns>`, such as `8x8`, and counts the whole
grid, including the first row and column; a grid with fewer than 3 rows or
columns holds only blocks. Sums go from 1 to 45. Sums that no
digits can reach still parse and have no solution.

Generator version 1 starts from white cells everywhere below the first row
and right of the first column. It makes cells blocks in a seeded order until
a share of the inner cells set by the level is blocked (30% easy, 25%
medium, 22% hard, 20% expert, 25% for any level). A new block also blocks
any white cell left in a run of one cell; a block that would change more
than 3 cells or split the white cells is skipped. Runs longer than a level's
limit (5 cells up to medium, 6 at hard and any level, 7 at expert) are then
broken the same way, and the layout starts over if one cannot be. Digits are
filled in row-major order by backtracking, each run preferring low or high
digits at random, more strictly at lower levels; the layout starts over
after 20,000 steps.

The sums follow from the digits. While the techniques up to the requested
level leave cells unresolved, unresolved cells change: a cell gets a digit
its runs do not have, chosen to leave its runs the fewest combinations, or
swaps digits with another cell of one of its runs, or, one time in four,
becomes a block while the blocks stay within 20 points of the level's
share. One more cell changes for every 16 unresolved cells. A puzzle solved
below the requested level gets random digit changes. Above Medium, a
puzzle that Medium techniques do not solve is tried with the contradiction
techniques only if it has one solution. A candidate starts over after 3
times as many changes without improvement as it has inner cells, or 8
times as many once solved.

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
| 6x6 | any | 20/20 | 4.8 ms | 30 ms | 38 |
| 6x6 | easy | 20/20 | 0.8 ms | 3.2 ms | 35 |
| 6x6 | medium | 20/20 | 5.8 ms | 33 ms | 50 |
| 6x6 | hard | 20/20 | 26 ms | 190 ms | 194 |
| 6x6 | expert | 20/20 | 163 ms | 562 ms | 691 |
| 8x8 | any | 20/20 | 16 ms | 273 ms | 80 |
| 8x8 | easy | 20/20 | 2.6 ms | 54 ms | 73 |
| 8x8 | medium | 20/20 | 10 ms | 102 ms | 61 |
| 8x8 | hard | 20/20 | 67 ms | 254 ms | 144 |
| 8x8 | expert | 20/20 | 213 ms | 1.1 s | 233 |
| 10x10 | any | 20/20 | 72 ms | 784 ms | 105 |
| 10x10 | easy | 20/20 | 8.7 ms | 133 ms | 147 |
| 10x10 | medium | 20/20 | 22 ms | 414 ms | 116 |
| 10x10 | hard | 20/20 | 222 ms | 554 ms | 154 |
| 10x10 | expert | 20/20 | 425 ms | 1.9 s | 262 |
| 12x12 | any | 14/20 | 469 ms | — | 95 |
| 12x12 | easy | 20/20 | 159 ms | 1.4 s | 595 |
| 12x12 | medium | 20/20 | 127 ms | 2.4 s | 267 |
| 12x12 | hard | 12/20 | 1.2 s | — | 207 |
| 12x12 | expert | 6/20 | — | — | 101 |
| 16x16 | any | 2/20 | — | — | 39 |
| 16x16 | easy | 15/20 | 1.9 s | — | 914 |
| 16x16 | medium | 4/20 | — | — | 34 |
| 16x16 | hard | 2/20 | — | — | 74 |
| 16x16 | expert | 0/20 | — | — | — |

Largest practical size (B1): easy 12x12, medium 12x12, hard 10x10, expert
10x10. Rating a corpus puzzle takes at most 23 ms. In WebAssembly
([`web/benchmark.mjs`](../../web/benchmark.mjs), 8x8), generation took 0.91
times the native time.

## References

Kakuro first appeared in Dell Magazines in the United States as Cross Sums
(`cross-sums`); Nikoli named it kakuro.
