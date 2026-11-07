# Hitori

Shade cells so no unshaded number repeats in a row or column; shaded cells
never share a side and unshaded cells stay connected.

## Rules

Every cell holds a printed number.

- **R1. No repeats.** In each row and each column, no number appears in two
  unshaded cells.
- **R2. Shaded cells apart.** Shaded cells do not share a side.
- **R3. Unshaded cells connected.** The unshaded cells form one nonempty
  group, connected through shared sides.

A 1x1 grid therefore has the single solution "unshaded". Shading a cell is
allowed even when its number does not repeat; a puzzle with one solution
never needs it.

## Example

Generator version 1, shape `4x4`, easy, seed `2` produces
`hitori:4x4:3222134241131133`:

```text
Puzzle     Solution
3 2 2 2    3 # 2 #
1 3 4 2    1 3 4 2
4 1 1 3    4 # 1 3
1 1 3 3    # 1 3 #
```

The 2 in the middle of the top row sits between two 2s, one of which is
shaded, so it stays unshaded and the other 2s in its row and column are
shaded. The bottom right 3 equals both its neighbours, so it is shaded.
`corpus/hitori/` has puzzles from easy to expert, with solutions.

## Variables

Every cell is a variable: unshaded (`-`) or shaded (`#`), ordered
row-major. No cell starts decided. The open area is every cell that may
still be unshaded.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each pair of cells in a line with the same number | R1 | both are unshaded |
| Each pair of neighbouring cells | R2 | both are shaded |
| The board | R3 | two unshaded cells are in different parts of the open area, or no cell can be unshaded |

The rater's broken check also fires when a cell has no candidate left. With
every cell decided, it is exactly "some rule is broken".

## Techniques

Rater version 1 uses the profile `hitori-human` version 1. The peers of a
cell are the other cells in its row and column with the same number.

| Technique | Level | What it does |
| --- | --- | --- |
| `routine` | bookkeeping | Neighbours of shaded cells are unshaded, and peers of unshaded cells are shaded. One step holds every such consequence. |
| `sandwich` | Easy | A cell between two equal numbers in its row or column is unshaded: one of the two is shaded, and shaded cells do not touch. |
| `pair` | Easy | Two neighbouring cells with the same number: one of them stays unshaded, so the other cells with that number in their line are shaded. |
| `corner` | Easy | A corner cell whose two neighbours hold its number is shaded. Unshaded, it would shade both and be cut off from the cell diagonal to it. |
| `cut` | Easy | An undecided cell whose shading would split the open area is unshaded, since its neighbours would be unshaded on different sides. So is the last cell of the open area. |
| `forced` | Medium | A value that breaks a rule within two rounds of `routine` is removed. |
| `short_contradiction` | Hard | If a cell were shaded (or unshaded), `routine` and the easy techniques would break a rule within 6 steps, so it is the other way. |
| `contradiction` | Expert | The same within 20 steps. |

Examples (`?` is undecided):

```text
sandwich       pair             corner          cut
  3 ? 3          2 2 ? 2          4 4 ?           - # ?
                                  4 ? ?           ? - ?
  middle is      the last 2       ? ? ?           ? ? ?
  unshaded       is shaded
                                  corner 4 is     top middle shaded:
                                  shaded          the top right cell
                                                  is cut off, so the
                                                  cell below it is
                                                  unshaded
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-12 | 8 |
| columns | yes | 1-12 | 8 |

The shape is written `<rows>x<columns>`, such as `8x8`. Numbers go from 1 to
the larger of the two sizes. Puzzles with no solution or several solutions
still parse.

Generator version 1 starts from a cyclic Latin square with its rows, columns
and symbols shuffled, cut to the shape. It shades a random quarter to two
fifths of the cells, keeping shaded cells apart and the unshaded cells
connected, and each shaded cell copies the number of an unshaded cell in its
row or column. The puzzle is screened with the exact solver and then with the
techniques up to the requested level. Changes near the cells left undecided
(or anywhere, if the puzzle was too easy) give a shaded cell another number,
unshade it, or shade a cell or its neighbour; a newly shaded cell's lines get
new copies where a shaded cell lost its number. A solved puzzle keeps a
change if its level rises or its rating score does not fall. A candidate is
rebuilt after twice as many changes as it has cells.

A 1x2 or 2x1 grid has no puzzle with exactly one solution. Single rows and
columns and 2x2 grids are always easy, and 3x3 grids are at most medium.

## Formats

```text
genre: hitori
grid:
3222
1342
4113
1133
```

Rows have one digit per cell. If any number has two digits, every row uses
space-separated numbers. JSON:

```json
{"genre":"hitori","puzzle":{"grid":["3222","1342","4113","1133"]}}
```

IDs list the cells row-major, one character each: `1`-`9`, then `a`-`z` for
10 to 35, such as `hitori:4x4:3222134241131133`.

Answers use `#` for shaded, `-` for unshaded and `.` for unmarked.
Finishing makes unmarked cells unshaded. Violations carry `number` and
`count` (unshaded cells holding it) for R1, at the row or column; R2 is
reported at each shaded cell next to another; R3 carries `groups`, the
number of parts of the open area holding unshaded cells.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- hitori 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`hitori.csv`](../benchmarks/hitori.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 5x5 | any | 20/20 | 0.2 ms | 1.1 ms | 6 |
| 5x5 | easy | 20/20 | 0.2 ms | 1.4 ms | 10 |
| 5x5 | medium | 20/20 | 0.7 ms | 4.7 ms | 22 |
| 5x5 | hard | 20/20 | 6.4 ms | 40 ms | 105 |
| 5x5 | expert | 17/20 | 312 ms | — | 3150 |
| 6x6 | any | 20/20 | 0.2 ms | 1.2 ms | 6 |
| 6x6 | easy | 20/20 | 0.2 ms | 5.2 ms | 13 |
| 6x6 | medium | 20/20 | 1.3 ms | 5.1 ms | 23 |
| 6x6 | hard | 20/20 | 20 ms | 83 ms | 162 |
| 6x6 | expert | 20/20 | 375 ms | 1.2 s | 1848 |
| 8x8 | any | 20/20 | 1.2 ms | 5.8 ms | 55 |
| 8x8 | easy | 20/20 | 1.6 ms | 9.7 ms | 63 |
| 8x8 | medium | 20/20 | 8.2 ms | 35 ms | 104 |
| 8x8 | hard | 20/20 | 50 ms | 123 ms | 221 |
| 8x8 | expert | 20/20 | 558 ms | 2.9 s | 1558 |
| 10x10 | any | 20/20 | 0.9 ms | 20 ms | 65 |
| 10x10 | easy | 20/20 | 1.0 ms | 19 ms | 67 |
| 10x10 | medium | 20/20 | 24 ms | 98 ms | 146 |
| 10x10 | hard | 20/20 | 200 ms | 621 ms | 491 |
| 10x10 | expert | 20/20 | 1.3 s | 8.2 s | 2085 |
| 12x12 | any | 20/20 | 24 ms | 59 ms | 186 |
| 12x12 | easy | 20/20 | 24 ms | 103 ms | 205 |
| 12x12 | medium | 20/20 | 106 ms | 322 ms | 385 |
| 12x12 | hard | 20/20 | 487 ms | 2.2 s | 1175 |
| 12x12 | expert | 14/20 | 6.0 s | — | 2522 |

Largest practical size (B1): easy 12x12, medium 12x12, hard 12x12, expert
8x8. Rating a corpus puzzle takes at most 3.8 ms. In WebAssembly
([`web/benchmark.mjs`](../../web/benchmark.mjs), 8x8), generation took 0.88
times the native time. Memory is O(N^2) for N cells.

## References

Hitori was first published by Nikoli.
