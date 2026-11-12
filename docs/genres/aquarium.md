# Aquarium

Fill tanks with water to match the row and column clues; the water in a tank
is level.

## Rules

The grid is divided into tanks: groups of cells joined by their sides. Each
cell is water (shaded) or air (unshaded). Every row and every column has a
clue.

- **R1. Row counts.** The number of water cells in each row equals that
  row's clue.
- **R2. Column counts.** The number of water cells in each column equals
  that column's clue.
- **R3. One level.** If a cell is water, every cell of its tank in the same
  row and in every row below is water.

R3 applies to the whole tank, including parts that only join above the
water. A tank may be empty or full.

## Example

Shape `4x4`, easy, seed `2` produces `aquarium:4x4:3341/4331/JXAn`:

```text
Puzzle             Solution
   4 3 3 1            4 3 3 1
  +-+-+-+-+          +-+-+-+-+
3 |     | |        3 |# # #|-|
  + + + + +          + + + + +
3 |     | |        3 |# # #|-|
  + + +-+ +          + + +-+ +
4 |   |   |        4 |# #|# #|
  + +-+-+-+          + +-+-+-+
1 | | | | |        1 |#|-|-|-|
  +-+-+-+-+          +-+-+-+-+
```

Column clues are above the grid and row clues to the left. Row 3 has the
clue 4, so all four of its cells are water. Column 4 then has its one water
cell, and the rest of it is air. Column 1 needs all four of its cells, which
fills the large tank from row 1 down. `corpus/aquarium/` has puzzles from
easy to expert, with solutions.

## Variables

Every cell is a variable: air (`-`) or water (`#`), ordered row-major. No
cell starts decided. A cell is open while it can still be either.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each row | R1 | its water cells outnumber the clue, or its water and open cells together fall short of it |
| Each column | R2 | its water cells outnumber the clue, or its water and open cells together fall short of it |
| Each tank | R3 | an air cell is level with or below a water cell of the tank |

The rater's broken check also fires when a cell has no candidate left. With
every cell decided, it is exactly "some rule is broken".

## Techniques

The rater uses the profile `aquarium-human`. A line is a row or a column,
and its need is its clue minus its water cells.

| Technique | Level | What it does |
| --- | --- | --- |
| `routine` | bookkeeping | Water spreads sideways and down in its tank, and air sideways and up. A line with all its water gets air in its open cells, and a line that needs all its open cells gets water. One step holds every such consequence. |
| `too_big` | Easy | A tank with more open cells in a line than the line needs cannot fill them all. In a row they share one level, so all of them are air. In a column they fill from the bottom, so the top ones beyond the need are air. |
| `needed` | Easy | A tank whose open cells a line cannot do without holds water there. In a row all of them are water. In a column the bottom ones are, as many as the other tanks fall short. |
| `combinations` | Medium | In a row, each tank's open cells are all water or all air. Tanks that are water in every way of making up the need get water, and tanks that are water in no way get air. |
| `short_contradiction` | Hard | If a cell were water (or air), `routine` and the easy and medium techniques would break a rule within 6 steps, so it is the other way. |
| `contradiction` | Expert | The same within 20 steps. |

Shorter contradiction chains are tried first. Within one row of a tank only
the leftmost open cell is tried, since the cells beside it share its value.

Examples, each showing one line of an untouched grid:

```text
too_big, row           too_big, column     needed, row
  need 2                 need 1              need 4
  |a a a|b|c|            |a|  air            |a a a|b|c|
  a is air               |a|  air            a is water
                         |a|

needed, column         combinations        combinations
  need 3                 need 4              need 4
  |a|                    |a a a|b b b|c|     |a a|b b|c|
  |a|  water             3+1 either way:     2+2 only:
  |b|                    c is water          a and b are
  |b|  water                                 water, c is air
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-15 | 8 |
| columns | yes | 1-15 | 8 |

The shape is written `<rows>x<columns>`, such as `8x8`. Puzzles with no
solution or several solutions still parse.

The generator grows tanks one at a time, each from the first free cell in
reading order, to a random size from 3 to 11 cells, most often 5 to 8. A
free cell beside one cell of the growing tank is three times as likely to
join as a cell beside two or more. When the grid's rows and columns average
fewer than 12, the sizes shrink in proportion, to no fewer than 3 cells. A
tank left with fewer than 3 cells joins its smallest neighbouring tank. Each
tank gets a random water level, and the clues count the water. The puzzle is
screened with the techniques up to the requested level. A change at a cell
they left open (or at any cell, if the puzzle was too easy) moves the water
level of its tank by one row, or moves the cell to a neighbouring tank when
its own tank stays in one piece and both tanks stay between 3 and 12 cells.
A solved puzzle keeps a change if its level rises or its rating score does
not fall. A candidate is rebuilt after twice as many changes as it has
cells.

Grids up to 3x3 are always easy, and 4x4 and 5x5 grids are at most hard.
Expert needs a grid of at least 64 cells, such as 8x8.

## Formats

```text
genre: aquarium
rows: 3 3 4 1
cols: 4 3 3 1
regions:
aaab
aaab
aabb
acde
```

`rows:` and `cols:` hold the clues, separated by spaces. `regions:` gives
each cell's tank as a letter. Cells with the same letter form a tank. A
puzzle with more than 52 tanks uses numbers from 1, separated by spaces.
Readers accept any letters and any numbers up to 999 as tank names; writers
name the tanks `a`-`z`, `A`-`Z` (or 1, 2, ...) in the order their first
cells appear, reading row by row. JSON:

```json
{"genre":"aquarium","puzzle":{"row_clues":[3,3,4,1],"col_clues":[4,3,3,1],"regions":["aaab","aaab","aabb","acde"]}}
```

An ID is `aquarium:<rows>x<columns>:<row clues>/<column clues>/<walls>`,
such as `aquarium:4x4:3341/4331/JXAn`. Each clue is one character: `0`-`9`,
then `a`-`f` for 10 to 15. The walls are one bit for each pair of
neighbouring cells, 1 when they are in different tanks: first each cell and
the cell to its right, row by row, then each cell and the cell below it, row
by row. Six bits make one character, first bit highest, with values 0 to 63
written `A`-`Z`, `a`-`z`, `0`-`9`, `-`, `_`. Unused bits of the last
character are 0. A 1x1 grid has no walls, and its ID ends with the second
`/`. An ID with a wall between two cells of one tank is rejected.

Answers use `#` for water, `-` for air and `.` for unmarked. Finishing makes
unmarked cells air. Violations of R1 and R2 carry `count` (the line's water
cells) and `clue`, at the row or column. R3 is reported at the tank, as a
region numbered from 0 in the order the tanks' first cells appear.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- aquarium 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`aquarium.csv`](../benchmarks/aquarium.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 6x6 | any | 20/20 | 0.1 ms | 0.4 ms | 1 |
| 6x6 | easy | 20/20 | 0.1 ms | 0.2 ms | 1 |
| 6x6 | medium | 20/20 | 6.3 ms | 60 ms | 254 |
| 6x6 | hard | 20/20 | 3.5 ms | 8.5 ms | 89 |
| 8x8 | any | 20/20 | 0.2 ms | 2.4 ms | 1 |
| 8x8 | easy | 20/20 | 0.2 ms | 0.4 ms | 1 |
| 8x8 | medium | 20/20 | 5.2 ms | 36 ms | 115 |
| 8x8 | hard | 20/20 | 4.4 ms | 13 ms | 42 |
| 8x8 | expert | 20/20 | 228 ms | 785 ms | 339 |
| 10x10 | any | 20/20 | 0.4 ms | 1.2 ms | 1 |
| 10x10 | easy | 20/20 | 0.4 ms | 0.9 ms | 2 |
| 10x10 | medium | 20/20 | 3.5 ms | 29 ms | 50 |
| 10x10 | hard | 20/20 | 5.2 ms | 13 ms | 22 |
| 10x10 | expert | 20/20 | 364 ms | 1.2 s | 149 |
| 12x12 | any | 20/20 | 0.6 ms | 18 ms | 1 |
| 12x12 | easy | 20/20 | 0.6 ms | 2.2 ms | 2 |
| 12x12 | medium | 20/20 | 4.6 ms | 60 ms | 60 |
| 12x12 | hard | 20/20 | 12 ms | 82 ms | 63 |
| 12x12 | expert | 20/20 | 266 ms | 5.1 s | 209 |
| 15x15 | any | 20/20 | 73 ms | 248 ms | 1 |
| 15x15 | easy | 20/20 | 4.7 ms | 17 ms | 30 |
| 15x15 | medium | 20/20 | 8.2 ms | 53 ms | 39 |
| 15x15 | hard | 20/20 | 74 ms | 599 ms | 3 |
| 15x15 | expert | 20/20 | 345 ms | 1.9 s | 14 |

6x6 has no expert row, since Expert needs at least 64 cells.

Largest practical size (B1): 15x15 at every level. Rating a corpus puzzle
takes at most 35 ms. In WebAssembly
([`web/benchmark.mjs`](https://github.com/emieks/pencil-puzzles/blob/main/web/benchmark.mjs), 10x10), generation took 1.08
times the native time. Memory is O(N^2) for N cells.

## References

Aquarium is best known from
[puzzle-aquarium.com](https://www.puzzle-aquarium.com/).
