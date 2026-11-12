# Binairo

Fill the grid with 0s and 1s: no three alike in a row, as many 0s as 1s in
each line, no two lines the same.

## Rules

Some cells start with a given digit. Every other cell gets a 0 or a 1. A
line is a row or a column.

- **R1. No three in a row.** No three cells next to each other in a line
  hold the same digit.
- **R2. As many 0s as 1s.** Every line holds as many 0s as 1s.
- **R3. Different lines.** No two rows are the same, and no two columns are
  the same.
- **R4. Givens.** Given digits cannot be changed.

The grid has an even number of rows and an even number of columns, so that
R2 can hold. R3 compares rows with rows and columns with columns only.

## Example

Shape `6x6`, easy, seed `0` produces `binairo:6x6:1a1c1h00a1a0a01i10a`:

```text
Puzzle        Solution
1 . 1 . . .   1 0 1 0 1 0
1 . . . . .   1 0 0 1 1 0
. . . 0 0 .   0 1 1 0 0 1
1 . 0 . 0 1   1 0 0 1 0 1
. . . . . .   0 1 1 0 1 0
. . . 1 0 .   0 1 0 1 0 1
```

Row 1 has a 1 on each side of its second cell, so that cell is 0: a 1 there
would make three 1s in a row. Column 1 starts with two 1s, so the cell below
them is 0. `corpus/binairo/` has puzzles from easy to expert, with solutions.

## Variables

Every cell is a variable: `0` or `1`, ordered row-major. Given cells start
decided. A cell is open while it can still be either. A line is finished
when none of its cells is open.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each row and column | R1 | three cells next to each other hold the same digit |
| Each row and column | R2 | more than half its cells hold the same digit |
| Each pair of rows, and each pair of columns | R3 | both are finished and the same |

R4 holds from the start, so the rater does not check it. The rater's broken
check also fires when a cell has no candidate left. With every cell decided,
it is exactly "some rule is broken".

## Techniques

The rater uses the profile `binairo-human`. A line needs one more of a digit
when it holds one fewer than half its cells of that digit.

| Technique | Level | What it does |
| --- | --- | --- |
| `routine` | bookkeeping | A line with half its cells one digit gets the other digit in its open cells. One step holds every such line. |
| `pair` | Easy | Two equal digits next to each other: the cells on both ends hold the other digit. |
| `gap` | Easy | Two equal digits with one cell between them: that cell holds the other digit. |
| `one_left` | Medium | In a line that needs one more of a digit, the other digit fills every open cell but one. Open cells where the digit would leave three of the other digit in a row get the other digit; if it fits in only one cell, it goes there. |
| `duplicate` | Medium | The same, also ruling out the cells where the digit would make the line a copy of a finished line (R3). |
| `short_contradiction` | Hard | If a cell held one digit, `routine` and the easy and medium techniques would break a rule within 6 steps, so it holds the other. |
| `contradiction` | Expert | The same within 20 steps. |

Shorter contradiction chains are tried first. Easy and medium techniques
take one step per line.

Examples, each a row of six cells:

```text
pair            gap             one_left
. 1 1 . . .     0 . 0 . . .     1 . . . 0 1
0 1 1 0 . .     0 1 0 . . .     1 0 . . 0 1

                                one more 1 is needed; in the
                                second cell it would leave
                                0 0 0, so that cell is 0

duplicate
0 1 0 1   finished row
0 1 . .   one more 0 and one more 1: 0 1 0 1 would copy the
0 1 1 0   finished row, so it is 0 1 1 0
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 2-16, even | 10 |
| columns | yes | 2-16, even | 10 |

The shape is written `<rows>x<columns>`, such as `10x10`. R3 needs as many
different lines as a grid has rows or columns, so a side of 2 needs the other
side to be 2, a side of 4 allows at most 6 on the other side, and a side of 6
at most 14. Puzzles with no solution or several solutions still parse.

The generator fills a random complete grid and starts with every cell given.
It removes givens in a random order: one at first, and twice as many after
each kept removal, up to 32. A removal is kept if the puzzle still has one
solution and the techniques up to the requested level solve it. After a
rejected removal of several givens, each of them is tried alone. When every
given has been tried, a puzzle at the requested level is returned, and one
below it starts over from a new grid.

2x2 grids are always easy, and 4x4 grids are at most medium. Every other
shape reaches every level.

## Formats

```text
genre: binairo
grid:
1.1...
1.....
...00.
1.0.01
......
...10.
```

`grid:` has one character per cell: `0` or `1` for a given and `.` for an
empty cell. JSON:

```json
{"genre":"binairo","puzzle":{"grid":["1.1...","1.....","...00.","1.0.01","......","...10."]}}
```

An ID is `binairo:<rows>x<columns>:<cells>`, such as
`binairo:6x6:1a1c1h00a1a0a01i10a`. The cells are read row by row: `0` and
`1` are givens, and a letter stands for a run of empty cells, `a` for 1 up to
`z` for 26. A longer run is written as `z`s followed by one letter for the
rest, so only a `z` may be followed by another letter.

Answers use `0`, `1` and `.` for unmarked, one row per line, or a JSON array
of row strings. A cell holds the answer's digit, or its given digit if the
answer leaves it unmarked. Finishing makes unmarked cells 0. R1 violations
are reported at the row or column with `start` (the first cell of the run
along the line, from 0), `length` and `digit`. R2 violations carry `zeros`
and `ones`. R3 is reported at the later of two equal lines, with `same_as`
(the earlier one, from 0). R4 is reported at the cell, with `given` and
`actual`.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- binairo 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`binairo.csv`](../benchmarks/binairo.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 6x6 | any | 20/20 | 0.7 ms | 2.0 ms | 31 |
| 6x6 | easy | 20/20 | 0.5 ms | 0.8 ms | 35 |
| 6x6 | medium | 20/20 | 0.5 ms | 0.6 ms | 31 |
| 6x6 | hard | 20/20 | 1.7 ms | 7.4 ms | 105 |
| 6x6 | expert | 20/20 | 15 ms | 69 ms | 754 |
| 8x8 | any | 20/20 | 3.9 ms | 50 ms | 51 |
| 8x8 | easy | 20/20 | 1.7 ms | 1.9 ms | 57 |
| 8x8 | medium | 20/20 | 1.7 ms | 2.1 ms | 50 |
| 8x8 | hard | 20/20 | 4.2 ms | 18 ms | 74 |
| 8x8 | expert | 20/20 | 16 ms | 49 ms | 122 |
| 10x10 | any | 20/20 | 17 ms | 80 ms | 82 |
| 10x10 | easy | 20/20 | 4.8 ms | 6.5 ms | 87 |
| 10x10 | medium | 20/20 | 4.7 ms | 5.4 ms | 82 |
| 10x10 | hard | 20/20 | 14 ms | 34 ms | 106 |
| 10x10 | expert | 20/20 | 40 ms | 72 ms | 133 |
| 12x12 | any | 20/20 | 79 ms | 297 ms | 115 |
| 12x12 | easy | 20/20 | 11 ms | 12 ms | 121 |
| 12x12 | medium | 20/20 | 12 ms | 14 ms | 116 |
| 12x12 | hard | 20/20 | 23 ms | 76 ms | 115 |
| 12x12 | expert | 20/20 | 105 ms | 472 ms | 169 |
| 14x14 | any | 20/20 | 118 ms | 524 ms | 155 |
| 14x14 | easy | 20/20 | 24 ms | 26 ms | 161 |
| 14x14 | medium | 20/20 | 26 ms | 30 ms | 157 |
| 14x14 | hard | 20/20 | 78 ms | 190 ms | 156 |
| 14x14 | expert | 20/20 | 247 ms | 881 ms | 232 |
| 16x16 | any | 20/20 | 408 ms | 647 ms | 197 |
| 16x16 | easy | 20/20 | 55 ms | 66 ms | 211 |
| 16x16 | medium | 20/20 | 60 ms | 88 ms | 200 |
| 16x16 | hard | 20/20 | 254 ms | 467 ms | 217 |
| 16x16 | expert | 20/20 | 465 ms | 973 ms | 264 |

Largest practical size (B1): 16x16 at every level. Rating a corpus puzzle
takes at most 3.9 ms. In WebAssembly
([`web/benchmark.mjs`](https://github.com/emieks/pencil-puzzles/blob/main/web/benchmark.mjs), 10x10), generation took 1.42
times the native time.

## References

Also known as Takuzu (`takuzu`).
