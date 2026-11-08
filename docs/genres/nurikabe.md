# Nurikabe

Shade a connected sea, leaving islands the size of their numbers and no
shaded 2×2 block.

## Rules

- **R1. Island size.** The unshaded component containing each number has exactly that many cells.
- **R2. One number per island.** Every unshaded component contains exactly one number.
- **R3. Connected sea.** Shaded cells form one nonempty orthogonally connected component.
- **R4. No pools.** No 2x2 block is entirely shaded.
- **R5. Printed cells.** Numbered cells are unshaded.

Cells connect through shared sides only. Because the sea must be nonempty, a
clue-free 1x1 board has the single solution `#`.

## Example

Generator version 3, shape `3x3`, easy, seed `0` produces
`nurikabe:3x3:.,.,.,.,.,.,5,.,1`:

```text
Clues     Solution
...       ---
...       -##
5.1       -#-
```

The 1 is an island of one cell, walled off by sea; the 5 fills the top row
and the left column. The three sea cells are connected, and no 2x2 block is
all sea. `corpus/nurikabe/` has puzzles from
easy to expert, with solutions.

## Variables

Every unnumbered cell is a variable: unshaded (land, `-`) or shaded (sea,
`#`), ordered row-major. Numbered cells are fixed land.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each number | R1 | the island is larger than its number, or cannot reach its size |
| The board | R2 | a group of land holds two numbers, or unnumbered land cannot reach any number |
| The board | R3 | the sea cells cannot all connect, or no cell can be sea |
| Each 2x2 block | R4 | all four are sea |

R5 holds from the start, so the rater does not check it.

## Techniques

Rater version 2 uses the profile `nurikabe-human` version 1. An island is a
connected group of land cells.

| Technique | Level | What it does |
| --- | --- | --- |
| `island_complete` | bookkeeping | A numbered island of its full size is walled off by sea. |
| `all_complete` | bookkeeping | Once every numbered island is complete, every open cell is sea. |
| `separate_clues` | Easy | A cell next to two different numbered islands is sea. |
| `no_pool` | Easy | A 2x2 block with three sea cells has land in the fourth. |
| `island_exit` | Easy | An island that must still grow, or unnumbered land that must still reach a number, with one way out extends through it. Cells next to another numbered island are not ways out for a numbered island. |
| `sea_exit` | Easy | A sea area that must join other sea, with one way out, extends through it. |
| `unreachable` | Medium | A cell no numbered island can reach within its remaining size is sea. Paths avoid sea, other numbered islands and cells next to them. |
| `forced` | Medium | A cell where one value breaks a rule at once (merging two islands, cutting the sea in two, leaving an island too little room) takes the other value. |
| `short_contradiction` | Hard | If a cell were land (or sea), easy and medium techniques would break a rule within 6 steps, so it is the other way. |
| `contradiction` | Expert | The same within 20 steps. |

Examples (`?` is undecided):

```text
separate_clues     no_pool      island_exit     unreachable
  2 ? 3            # #            3 #           1 ? ? ? ?
                   # ?  -> -      ? #           ? ? ? ? ?   the far corner
  middle is sea                   # #           ? ? ? ? ?   is beyond every
                                                ? ? ? ? ?   island's reach,
  between two      fourth cell    the 3 can     ? ? ? ? 2   so it is sea
  numbers          is land        only grow down
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-10 | 10 |
| columns | yes | 1-10 | 10 |

The shape is written `<rows>x<columns>`, such as `5x5`. Clues go from 1 to
the number of cells. Contradictory clues still parse and have no solution.

Generator version 3 grows islands from seeds inside a connected sea. A seed
is a cell not touching land whose removal keeps the sea connected; each
island grows one cell at a time toward a size drawn from 1 to 6 (1 to 3 for
easy), until 40-55% of the cells are land, and a cell of every all-sea 2x2
block then becomes land. Each island is numbered on a random cell. The
puzzle is screened with the techniques up to the requested level, at most
Medium; if they do not finish and the level is above Medium, the brute-force
solver looks for a second solution within the candidates they left before
the contradiction techniques run. A second solution is ruled out by moving a
clue onto a cell that is land in the intended solution and sea in the other.
Where the techniques got stuck, a sea cell touching at most one island
becomes land, or a clue moves there. A puzzle below the requested level gets
a clue moved within its island, two islands joined through one sea cell, or
an island grown by one cell. A candidate starts over after 12 times as many
changes without improvement as it has rows and columns.

## Formats

```text
genre: nurikabe
grid:
1.1
...
1.1
```

Rows have one character per cell; if any clue has two digits, every row
uses space-separated tokens. `.` means no clue. IDs use comma-separated
row-major tokens, such as `nurikabe:3x3:1,.,1,.,.,.,1,.,1`. JSON:

```json
{"genre":"nurikabe","puzzle":{"grid":["1.1","...","1.1"]}}
```

Answers use `.`, `#` and `-`. Finishing makes unmarked cells unshaded, and
clue cells left out of an answer count as unshaded. Violations carry
`minimum`, `maximum` and `clue` for R1, `components` for R2 and R3, `shaded`
for R4, and `clue` for R5.

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- nurikabe 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`nurikabe.csv`](../benchmarks/nurikabe.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 5x5 | any | 20/20 | 2.8 ms | 31 ms | 29 |
| 5x5 | easy | 20/20 | 0.3 ms | 14 ms | 92 |
| 5x5 | medium | 20/20 | 1.0 ms | 5.6 ms | 35 |
| 5x5 | hard | 20/20 | 5.5 ms | 31 ms | 34 |
| 5x5 | expert | 20/20 | 51 ms | 462 ms | 191 |
| 7x7 | any | 20/20 | 22 ms | 198 ms | 70 |
| 7x7 | easy | 20/20 | 17 ms | 46 ms | 204 |
| 7x7 | medium | 20/20 | 14 ms | 83 ms | 91 |
| 7x7 | hard | 20/20 | 23 ms | 203 ms | 72 |
| 7x7 | expert | 20/20 | 147 ms | 480 ms | 111 |
| 8x8 | any | 20/20 | 146 ms | 682 ms | 100 |
| 8x8 | easy | 20/20 | 27 ms | 96 ms | 200 |
| 8x8 | medium | 20/20 | 31 ms | 173 ms | 163 |
| 8x8 | hard | 20/20 | 169 ms | 548 ms | 133 |
| 8x8 | expert | 20/20 | 256 ms | 750 ms | 114 |
| 10x10 | any | 20/20 | 498 ms | 5.1 s | 135 |
| 10x10 | easy | 20/20 | 125 ms | 769 ms | 529 |
| 10x10 | medium | 20/20 | 394 ms | 1.7 s | 570 |
| 10x10 | hard | 20/20 | 562 ms | 7.8 s | 165 |
| 10x10 | expert | 20/20 | 1.3 s | 5.1 s | 187 |

Largest practical size (B1): easy 10x10, medium 10x10, hard 8x8, expert
8x8. Rating a corpus puzzle takes at most 8.5 ms. In WebAssembly
([`web/benchmark.mjs`](../../web/benchmark.mjs), 10x10), generation took 1.41
times the native time. Memory is O(N^2) for N cells.

## References

The rules are based on [Nikoli's nurikabe rules](https://www.nikoli.co.jp/en/puzzles/nurikabe/).
