# Fillomino

Divide the grid into regions whose cells hold the region's size; same-size
regions never share a side.

## Rules

A group is a set of cells holding the same number, connected through
shared sides, that is as large as possible.

- **R1. Region size.** Every group has exactly as many cells as its number.
- **R2. Every cell numbered.** Every cell holds a positive number.
- **R3. Printed numbers.** A printed number stays in its cell.

In the usual terms, each group is a region, each cell holds its region's
size, and regions of the same size cannot share a side. A region may hold
several printed numbers or none.

## Example

Generator version 1, shape `3x3`, easy, seed `0` produces
`fillomino:3x3:a2b234b`:

```text
Clues     Solution
. 2 .     4 2 3
. 2 3     4 2 3
4 . .     4 4 3
```

The two printed 2s touch, so they are one complete region. The 3 can only
fill the right column, and the four cells left over form the 4.
`corpus/fillomino/` has puzzles from easy to expert, with solutions.

## Variables

Every cell is a variable, ordered row-major. Its candidates are the sizes
1 to 15, plus one candidate standing for every size from 16 up, written
`16+`. Printed cells start fixed. A cell is fixed when one candidate is
left; a group of fixed cells with the same exact size is one region.

A puzzle whose solution has a region of 16 or more cells solves normally,
but `rate` reports it unrated, since the candidates cannot tell such sizes
apart. The generator never builds regions larger than 9.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each group of fixed cells | R1 | it has more cells than its size, or the cells that still allow its size around it are too few to reach it |
| Each cell | R2 | it has no candidate left |

R3 holds from the start, so the rater does not check it. With one candidate
everywhere and no `16+` cells, the check is exactly "some group's size is
not its number".

## Techniques

Rater version 1 uses the profile `fillomino-human` version 1. The room of
a group is the set of cells connected to it through cells that still allow
its size.

| Technique | Level | What it does |
| --- | --- | --- |
| `region_complete` | bookkeeping | A group of its full size is walled off: open neighbours lose its size. |
| `too_big` | Easy | A size is removed from a cell if writing it there would join groups of that size into one with too many cells, such as a cell between two 1s or next to two diagonal 2s. |
| `only_exit` | Easy | A group that must grow, with one open neighbour that allows its size, grows there. |
| `exact_room` | Easy | A group whose room has exactly as many cells as its size fills the room. |
| `no_room` | Easy | A size is removed from every area of cells allowing it that is smaller than the size. |
| `pair` | Easy | For two open cells surrounded by fixed cells, every way to fill them is tried and only sizes that break no rule are kept. An enclosed pair next to no 2 is a 2. |
| `must_include` | Medium | A group that cannot reach its size without some open cell takes it. |
| `completions` | Medium | A group missing one or two cells: cells used by every way to finish it take its size, and cells next to the finished region in every way lose it. |
| `pocket` | Medium | The same as `pair` for leftover areas of up to 5 cells. |
| `forced` | Medium | A size that breaks a rule as soon as it is written in is removed from the cell. Only cells next to a fixed cell are tried. |
| `short_contradiction` | Hard | If a cell with at most 2 candidates had a size, easy and medium techniques would break a rule within 6 steps, so that size goes. |
| `contradiction` | Expert | The same for cells with at most 3 candidates, within 20 steps, using techniques up to hard. |

The contradiction techniques remove every refuted size of a cell in one
step.

Examples (`?` is undecided):

```text
too_big            only_exit
  2 ?                3 3 2
  ? 2                1 ? 2

  neither ? is 2:    the 3 can only
  it would join      grow into ?,
  three cells        so ? is 3
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-10 | 10 |
| columns | yes | 1-10 | 10 |

The shape is written `<rows>x<columns>`, such as `6x6`. Clues go from 1 to the number
of cells. Contradictory clues still parse and have no solution.

Generator version 1 grows a random partition: it visits the cells in a
seeded order and grows a region from each free one toward a size drawn
from 1 to 9 (weights 1, 3, 5, 6, 6, 5, 3, 2, 1). A region that touches a
finished region of its size takes one more cell or joins a neighbouring
region if that removes the clash; otherwise the partition starts over. Every
cell starts as a clue. Clues are then removed in a random order, keeping
each removal while the techniques up to the requested level still solve
the puzzle. If the puzzle is still too easy when no clue is left to try, a
few clues go back into the best clue set so far and a new order starts, up
to 24 times before a new partition.

## Formats

```text
genre: fillomino
grid:
.2.
.23
4..
```

Rows have one character per cell, `.` for no clue. If any clue has two
digits, every row uses space-separated tokens. JSON:

```json
{"genre":"fillomino","puzzle":{"grid":[".2.",".23","4.."]}}
```

IDs list the cells row-major: a digit for a clue from 1 to 9, `-` and two
digits for a larger clue (`-12`; 100 is `-00`), and a letter for a run of
cells without clues (`a` is one cell, `z` is 26), such as
`fillomino:3x3:a2b234b`.

Answers use `.` for unmarked, `0` for a cell left empty, and numbers,
spaced like the grid when any number has two digits. Finishing turns
unmarked cells into empty ones, which breaks R2. Printed cells left out of
an answer count as their clue. Violations carry `number` and `cells` (the
group's size) for R1, nothing for R2, and `clue` for R3.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- fillomino 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`fillomino.csv`](../benchmarks/fillomino.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 5x5 | any | 20/20 | 20 ms | 69 ms | 26 |
| 5x5 | easy | 20/20 | 5.3 ms | 11 ms | 26 |
| 5x5 | medium | 20/20 | 9.4 ms | 18 ms | 26 |
| 5x5 | hard | 20/20 | 826 ms | 1.7 s | 940 |
| 5x5 | expert | 20/20 | 468 ms | 2.7 s | 434 |
| 6x6 | any | 20/20 | 60 ms | 184 ms | 37 |
| 6x6 | easy | 20/20 | 13 ms | 19 ms | 37 |
| 6x6 | medium | 20/20 | 24 ms | 50 ms | 37 |
| 6x6 | hard | 20/20 | 1.2 s | 3.1 s | 764 |
| 6x6 | expert | 20/20 | 950 ms | 3.2 s | 291 |
| 7x7 | any | 20/20 | 185 ms | 1.5 s | 50 |
| 7x7 | easy | 20/20 | 30 ms | 48 ms | 50 |
| 7x7 | medium | 20/20 | 56 ms | 85 ms | 50 |
| 7x7 | hard | 20/20 | 1.5 s | 7.4 s | 700 |
| 7x7 | expert | 20/20 | 1.1 s | 7.1 s | 288 |
| 8x8 | any | 20/20 | 544 ms | 1.8 s | 65 |
| 8x8 | easy | 20/20 | 63 ms | 125 ms | 65 |
| 8x8 | medium | 20/20 | 118 ms | 254 ms | 65 |
| 8x8 | hard | 20/20 | 2.4 s | 12.8 s | 642 |
| 8x8 | expert | 20/20 | 1.8 s | 22.8 s | 258 |
| 10x10 | any | 20/20 | 3.2 s | 12.8 s | 101 |
| 10x10 | easy | 20/20 | 218 ms | 573 ms | 101 |
| 10x10 | medium | 20/20 | 506 ms | 836 ms | 101 |
| 10x10 | hard | 20/20 | 2.2 s | 38.7 s | 404 |
| 10x10 | expert | 20/20 | 6.0 s | 22.8 s | 150 |

Largest practical size (B1): easy 10x10, medium 10x10, hard 5x5, expert 6x6.
Rating a corpus puzzle takes at most 15 ms. In WebAssembly
([`web/benchmark.mjs`](../../web/benchmark.mjs), 6x6), generation took 0.76
times the native time. Memory is O(N^2) for N cells.

## References

Fillomino was first published by Nikoli; it is also known as Allied
Occupation (`allied-occupation`).
