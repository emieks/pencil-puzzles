# Kakurasu

Shade cells so each row and column clue is the sum of its shaded cells'
positions.

## Rules

- **R1. Row totals.** In each row, add the column numbers of its shaded
  cells, counting columns from 1. The sum must equal that row's clue.
- **R2. Column totals.** In each column, add the row numbers of its shaded
  cells, counting rows from 1. The sum must equal that column's clue.

A zero clue means its line has no shaded cells.

## Example

Generator version 3, shape `3x5`, easy, seed `0` produces
`kakurasu:3x5:8.12.3/0.0.6.2.3`:

```text
genre: kakurasu
rows: 8 12 3
cols: 0 0 6 2 3
```

Solution:

```text
--#-#
--###
--#--
```

Row 1 is 3 + 5 = 8. Column 3 is 1 + 2 + 3 = 6. Columns 1 and 2
have zero clues, so neither contains a shaded cell.

## Variables

Each cell is a variable, shaded (`#`) or unshaded (`-`), ordered row-major.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each row | R1 | its shaded cells exceed the clue, or its open cells cannot reach it |
| Each column | R2 | its shaded cells exceed the clue, or its open cells cannot reach it |

## Techniques

Rater version 3 uses the profile `kakurasu-human` version 1. Within a line,
"need" is the clue minus the shaded cells' weights.

| Technique | Level | What it does |
| --- | --- | --- |
| `line_complete` | bookkeeping | A line whose need is 0 has its open cells unshaded. |
| `too_heavy` | Easy | An open cell worth more than the need is unshaded. In a line worth 1, 2, 3 with clue 2, the third cell is unshaded. |
| `needed` | Easy | An open cell without which the others fall short is shaded. In a line worth 1, 2, 3 with clue 5, cells 2 and 3 are shaded. |
| `combinations` | Medium | The open cells can reach the need in at most 4 ways; cells shaded in every way are shaded, cells in none are unshaded. With 1, 2, 3, 4 and need 6, the ways are {2,4} and {1,2,3}, so cell 2 is shaded. |
| `many_combinations` | Hard | The same with more than 4 ways, found with a subset-sum table. |
| `short_contradiction` | Hard | If a cell were shaded (or unshaded), easy and medium techniques would break a line within 6 steps, so it is the other way. |
| `contradiction` | Expert | The same, also using `many_combinations`, within 20 steps. |

Shorter contradiction chains are tried first.

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-32 | 6 |
| columns | yes | 1-32 | 6 |

The shape is written `<rows>x<columns>`, such as `5x7`.

Generator version 3 shades 35% to 65% of the cells at random, and the clues
follow from the shading. The puzzle is screened with the techniques up to the
requested level. A cell they could not decide (or any cell, if the puzzle was
too easy) is flipped, and the flip is kept unless the puzzle moved further
from the requested level. After 2 x (rows + columns) rejected flips it starts
over with a new grid.

## Formats

The puzzle text is shown above: `rows:` and `cols:` hold space-separated
clues. An ID is `kakurasu:<rows>x<columns>:<row clues>/<column clues>`, with
clues separated by dots. JSON:

```json
{"genre":"kakurasu","puzzle":{"row_clues":[1,2],"col_clues":[1,2]}}
```

Answers use one row per line, or a JSON array of row strings such as
`["#-",".#"]`: `#` shaded, `-` unshaded, `.` unmarked. Finishing an answer
makes unmarked cells unshaded. Violations carry `total` (the shaded weight so
far) and `clue`.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- kakurasu 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`kakurasu.csv`](../benchmarks/kakurasu.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 6x6 | any | 20/20 | 0.4 ms | 8.7 ms | 1 |
| 6x6 | easy | 20/20 | 0.4 ms | 0.9 ms | 2 |
| 6x6 | medium | 20/20 | 0.7 ms | 6.2 ms | 8 |
| 6x6 | hard | 20/20 | 19 ms | 40 ms | 26 |
| 6x6 | expert | 20/20 | 307 ms | 834 ms | 116 |
| 8x8 | any | 20/20 | 60 ms | 873 ms | 4 |
| 8x8 | easy | 20/20 | 2.1 ms | 6.1 ms | 15 |
| 8x8 | medium | 20/20 | 1.5 ms | 3.7 ms | 10 |
| 8x8 | hard | 20/20 | 65 ms | 217 ms | 7 |
| 8x8 | expert | 20/20 | 272 ms | 865 ms | 12 |
| 10x10 | any | 20/20 | 1.8 s | 3.5 s | 38 |
| 10x10 | easy | 20/20 | 12 ms | 21 ms | 66 |
| 10x10 | medium | 20/20 | 12 ms | 25 ms | 56 |
| 10x10 | hard | 20/20 | 533 ms | 1.5 s | 48 |
| 10x10 | expert | 20/20 | 2.1 s | 3.5 s | 39 |
| 12x12 | any | 20/20 | 6.7 s | 15.0 s | 127 |
| 12x12 | easy | 20/20 | 51 ms | 81 ms | 198 |
| 12x12 | medium | 20/20 | 38 ms | 63 ms | 144 |
| 12x12 | hard | 20/20 | 2.6 s | 6.4 s | 146 |
| 12x12 | expert | 20/20 | 6.9 s | 15.0 s | 128 |

Largest practical size (B1): easy 12x12, medium 12x12, hard 10x10, expert
8x8. Rating a corpus puzzle takes at most 186 ms. In WebAssembly
([`web/benchmark.mjs`](../../web/benchmark.mjs), 6x6), generation took 0.65
times the native time. Memory is O(N^2) for N cells. Generating puzzles near
the 32x32 limit is slow.

## References

Also known as Index Sums (`index-sums`).
