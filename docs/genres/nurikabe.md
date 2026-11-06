# Nurikabe

Shade a connected sea around numbered islands.

## Rules

- **R1. Island size.** The unshaded component containing each number has exactly that many cells.
- **R2. One number per island.** Every unshaded component contains exactly one number.
- **R3. Connected sea.** Shaded cells form one nonempty orthogonally connected component.
- **R4. No pools.** No 2x2 block is entirely shaded.
- **R5. Printed cells.** Numbered cells are unshaded.

Cells connect through shared sides only. Because the sea must be nonempty, a
clue-free 1x1 board has the single solution `#`.

## Example

Generator version 2, shape `3x3`, easy, seed `0` produces
`nurikabe:3x3:.,1,.,.,.,.,1,.,1`:

```text
Clues     Solution
.1.       #-#
...       ###
1.1       -#-
```

Each 1 is an island of one cell, walled off by sea; the six sea cells are
connected, and no 2x2 block is all sea. `corpus/nurikabe/` has puzzles from
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
| rows | yes | 1-8 | 6 |
| columns | yes | 1-8 | 6 |

The shape is written `<rows>x<columns>`, such as `5x5`. Clues go from 1 to
the number of cells. Contradictory clues still parse and have no solution.

Generator version 2 grows a seeded connected sea without pools and gives each
island one number, its size. The puzzle is screened with the techniques up to
the requested level. A cell they could not decide (or any cell, if the puzzle
was too easy) is flipped, keeping the sea connected and pool-free, and the
islands are renumbered.

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
| 5x5 | any | 20/20 | 5.2 ms | 1.3 s | 39 |
| 5x5 | easy | 20/20 | 1.2 ms | 5.0 ms | 25 |
| 5x5 | medium | 20/20 | 2.7 ms | 18 ms | 16 |
| 5x5 | hard | 20/20 | 85 ms | 1.3 s | 54 |
| 5x5 | expert | 20/20 | 2.2 s | 11.2 s | 316 |
| 6x6 | any | 20/20 | 283 ms | 2.9 s | 34 |
| 6x6 | easy | 20/20 | 3.4 ms | 13 ms | 35 |
| 6x6 | medium | 20/20 | 8.7 ms | 32 ms | 28 |
| 6x6 | hard | 20/20 | 306 ms | 1.9 s | 47 |
| 6x6 | expert | 20/20 | 4.3 s | 16.4 s | 207 |
| 7x7 | any | 20/20 | 812 ms | 16.1 s | 62 |
| 7x7 | easy | 20/20 | 7.3 ms | 28 ms | 42 |
| 7x7 | medium | 20/20 | 12 ms | 106 ms | 28 |
| 7x7 | hard | 20/20 | 753 ms | 9.6 s | 90 |
| 7x7 | expert | 20/20 | 2.9 s | 26.1 s | 126 |
| 8x8 | any | 20/20 | 4.4 s | 48.2 s | 30 |
| 8x8 | easy | 20/20 | 15 ms | 76 ms | 62 |
| 8x8 | medium | 20/20 | 38 ms | 273 ms | 37 |
| 8x8 | hard | 20/20 | 2.3 s | 13.6 s | 50 |
| 8x8 | expert | 20/20 | 15.4 s | 59.7 s | 161 |

Largest practical size (B1): easy 8x8, medium 8x8, hard 6x6; no measured
size is practical for expert. Rating a corpus puzzle takes at most 39 ms. In
WebAssembly ([`web/benchmark.mjs`](../../web/benchmark.mjs), 6x6),
generation took 0.60 times the native time. Memory is O(N^2) for N cells.

## References

The rules are based on [Nikoli's nurikabe rules](https://www.nikoli.co.jp/en/puzzles/nurikabe/).
