# Chocona

Shade rectangles so each region has as many shaded cells as its number.

## Rules

The grid is divided into regions: groups of cells joined by their sides.
Some regions have a number. Each cell is shaded or unshaded.

- **R1. Region counts.** A region with a number has exactly that many
  shaded cells. A region without a number may have any number of shaded
  cells, including none.
- **R2. Rectangles.** Every group of shaded cells joined by their sides
  fills a rectangle.

Rectangles may cross region borders. Two rectangles may touch at a corner,
but not along a side, since they would then form one group.

## Example

Shape `4x5`, easy, seed `0` produces `chocona:4x5:RmYe9g/0422022`:

```text
Puzzle         Solution
+-+-+-+-+-+    +-+-+-+-+-+
|0  |4    |    |- -|# # #|
+ + + +-+-+    + + + +-+-+
|   | |2  |    |- -|#|# #|
+-+-+ +-+-+    +-+-+ +-+-+
|2  | |0  |    |# #|-|- -|
+-+-+ +-+-+    +-+-+ +-+-+
|2  | |2  |    |# #|-|# #|
+-+-+-+-+-+    +-+-+-+-+-+
```

Each number is in its region's first cell. In the solution, every region
holds as many shaded cells as its number, and the shaded cells form three
rectangles: 2x3 at the top right, across the regions with 4 and the 2 below
them, 2x2 at the bottom left, across the two regions with 2, and 1x2 at the
bottom right. `corpus/chocona/` has puzzles from easy to expert, with
solutions.

## Variables

Every cell is a variable: unshaded (`-`) or shaded (`#`), ordered row-major.
No cell starts decided. A cell is open while it can still be either.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each numbered region | R1 | its shaded cells outnumber its number, or its shaded and open cells together fall short of it |
| Each 2x2 block | R2 | three of its cells are shaded and the fourth is unshaded |

The shaded cells form rectangles exactly when no 2x2 block has three shaded
cells. The rater's broken check also fires when a cell has no candidate
left. With every cell decided, it is exactly "some rule is broken".

## Techniques

The rater uses the profile `chocona-human`. A group is a set of shaded cells
joined by their sides, and its box is the smallest rectangle around it. A
region's need is its number minus its shaded cells.

| Technique | Level | What it does |
| --- | --- | --- |
| `routine` | bookkeeping | A numbered region with all its shaded cells gets its open cells unshaded, and one that needs all of them gets them shaded. Every group's box is shaded. One step holds every such consequence. |
| `bend` | Easy | An open cell next to a group is unshaded if the box of the cell and the groups beside it covers an unshaded cell: shaded, the cell would join them into a group that is not a rectangle. |
| `overflow` | Easy | An open cell next to a group is unshaded if that box would put more shaded cells into a numbered region than its number. |
| `combinations` | Medium | The ways to shade a numbered region's open cells with exactly its need, keeping every 2x2 block of decided cells from having three shaded cells. Cells shaded in every way are shaded, and cells shaded in no way are unshaded. Only regions with at most 70 such subsets of their open cells and at most 6 ways are compared. |
| `short_contradiction` | Hard | If an open cell next to a decided cell were shaded (or unshaded), `routine` and the easy and medium techniques would break a rule within 6 steps, so it is the other way. |
| `contradiction` | Expert | The same within 20 steps. |

Shorter contradiction chains are tried first.

Examples (`?` is undecided; each shows a region layout and its marks):

```text
bend, beside       bend, at a corner   overflow            combinations
  # #                # ?                 abb    # ? ?        aaa    ? ? ?
  - ?                - #                 abb    # ? ?        aaa    - - ?
                                         a is 2, b is 1      a is 3
  ? would make an    ? would join both   either middle ?     of the three ways,
  L of three around  groups in a 2x2     fills the 2x2 box   every one shades
  the -              box with the -      with two cells of b the top left ?
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-15 | 10 |
| columns | yes | 1-15 | 10 |

The shape is written `<rows>x<columns>`, such as `10x10`. Puzzles with no
solution or several solutions still parse.

The generator places rectangles that do not touch along a side, reading the
cells row by row: at each cell where one fits, except one in eight chosen at
random, a rectangle of a random size with sides of up to 4 cells starts
there, made shorter or narrower until it fits. About half of the cells end
up shaded. The grid is then split into regions of about 2 to 4 cells of one
shade each, which the region counts alone settle. Neighbouring regions are
joined, a region with shaded cells preferring one without and the other way
round, as long as the techniques up to Medium (or up to the requested level,
if lower) still solve the puzzle. This goes on until the regions average 4.5
cells (fewer on grids whose sides average under 10 cells, or on narrow
grids), at most one region in 50 cells has a single cell, and at most one
region in five has no shaded cells. A one-cell rectangle in a region of its
own is unshaded before its region is joined to another. Cells are then moved
across region borders while the same techniques solve the puzzle, one for
every 4 cells. Regions never grow past 12 cells. Every region shows its
number.

If the puzzle is not yet at the requested level, it is then changed until
it is: a cell moves to a neighbouring region (17 times in 20), two regions
are joined (twice in 20, while there are more than three quarters as many
regions as the goal), or a rectangle gains or loses a row or column, or a
cell becomes a rectangle of its own. A change is kept unless the level drops
or the techniques can no longer solve the puzzle. A candidate is rebuilt
after four times as many changes as it has cells, or after as many rejected
changes in a row as it has cells.

Medium needs at least 2 rows, 2 columns and 6 cells, Hard at least 2 rows, 2
columns and 9 cells, and Expert at least 3 rows, 3 columns and 15 cells.
Grids with one row or column are always easy.

## Formats

```text
genre: chocona
regions:
aab
acc
clues: 3 . 1
```

`regions:` gives each cell's region as a letter. Cells with the same letter
form a region. A puzzle with more than 52 regions uses numbers from 1,
separated by spaces. Readers accept any letters and any numbers up to 999 as
region names; writers name the regions `a`-`z`, `A`-`Z` (or 1, 2, ...) in
the order their first cells appear, reading row by row. `clues:` has one
entry per region in the same order: its number, or `.` for a region without
one. JSON:

```json
{"genre":"chocona","puzzle":{"regions":["aab","acc"],"clues":[3,null,1]}}
```

An ID is `chocona:<rows>x<columns>:<walls>/<clues>`, such as
`chocona:2x3:Zg/3.1`. The walls are one bit for each pair of neighbouring
cells, 1 when they are in different regions: first each cell and the cell to
its right, row by row, then each cell and the cell below it, row by row. Six
bits make one character, first bit highest, with values 0 to 63 written
`A`-`Z`, `a`-`z`, `0`-`9`, `-`, `_`. Unused bits of the last character are
0. A 1x1 grid has no walls, and its ID has nothing before the `/`. An ID
with a wall between two cells of one region is rejected. The clues follow
in region order, one character each: `.` for none, `0`-`9`, then `a`-`z` for
10 to 35, and `-` followed by two such characters (a base-36 number) for 36
and more.

Answers use `#` for shaded, `-` for unshaded and `.` for unmarked. Finishing
makes unmarked cells unshaded. Violations of R1 carry `count` (the region's
shaded cells) and `clue`, at the region, numbered from 0 in the order the
regions' first cells appear. R2 is reported at the first cell, reading row
by row, of a group of shaded cells whose rectangle holds a cell marked
unshaded.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- chocona 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`chocona.csv`](../benchmarks/chocona.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 6x6 | any | 20/20 | 1.1 ms | 2.3 ms | 1 |
| 6x6 | easy | 20/20 | 1.6 ms | 5.3 ms | 1 |
| 6x6 | medium | 20/20 | 1.1 ms | 2.2 ms | 1 |
| 6x6 | hard | 20/20 | 3.7 ms | 21 ms | 46 |
| 6x6 | expert | 20/20 | 13 ms | 54 ms | 157 |
| 8x8 | any | 20/20 | 5.7 ms | 21 ms | 1 |
| 8x8 | easy | 20/20 | 6.1 ms | 18 ms | 1 |
| 8x8 | medium | 20/20 | 5.8 ms | 23 ms | 2 |
| 8x8 | hard | 20/20 | 8.4 ms | 27 ms | 16 |
| 8x8 | expert | 20/20 | 36 ms | 136 ms | 117 |
| 10x10 | any | 20/20 | 16 ms | 62 ms | 1 |
| 10x10 | easy | 20/20 | 15 ms | 63 ms | 1 |
| 10x10 | medium | 20/20 | 17 ms | 60 ms | 1 |
| 10x10 | hard | 20/20 | 23 ms | 70 ms | 23 |
| 10x10 | expert | 20/20 | 70 ms | 369 ms | 100 |
| 12x12 | any | 20/20 | 32 ms | 71 ms | 1 |
| 12x12 | easy | 20/20 | 35 ms | 96 ms | 1 |
| 12x12 | medium | 20/20 | 32 ms | 71 ms | 1 |
| 12x12 | hard | 20/20 | 64 ms | 178 ms | 24 |
| 12x12 | expert | 20/20 | 118 ms | 1.4 s | 78 |
| 15x15 | any | 20/20 | 99 ms | 170 ms | 1 |
| 15x15 | easy | 20/20 | 102 ms | 346 ms | 1 |
| 15x15 | medium | 20/20 | 103 ms | 172 ms | 1 |
| 15x15 | hard | 20/20 | 129 ms | 238 ms | 23 |
| 15x15 | expert | 20/20 | 315 ms | 1.7 s | 61 |

Largest practical size (B1): 15x15 at every level. Rating a corpus puzzle
takes at most 18 ms. In WebAssembly
([`web/benchmark.mjs`](https://github.com/emieks/pencil-puzzles/blob/main/web/benchmark.mjs), 10x10), generation took 0.96
times the native time. Memory is O(N^2) for N cells.

## References

Chocona (チョコナ) is a Japanese genre.
[janko.at](https://www.janko.at/Raetsel/Chocona/) has a collection of
Chocona puzzles.
