# Hashi

Connect all islands with straight bridges that never cross, giving each
island as many bridges as its number.

## Rules

Islands are cells with a number from 1 to 8. A bridge is a straight
horizontal or vertical line between two islands with no island between
them. Two islands are joined by zero, one or two bridges.

- **R1. Island numbers.** Each island has as many bridges as its number.
- **R2. No crossings.** Bridges do not cross.
- **R3. Connected.** Every island can be reached from every other through
  bridges.

Islands are never orthogonally adjacent, so every bridge passes through at
least one cell. Puzzles from sources that allow adjacent islands can't be
written in this project's formats.

## Example

Generator version 1, shape `5x5`, easy, seed `2` produces
`hashi:5x5:2a1a3e4a2a6e3c3`:

```text
Clues    Solution
2.1.3    2.1-3
.....    H...H
4.2.6    4.2=6
.....    H...H
3...3    3---3
```

The 6 has islands above, to the left and below, and no more than two
bridges go to each, so it takes two in every direction. The 2 in the middle
is then finished, and the rest follows from the numbers and the need to
connect everything. `corpus/hashi/` has puzzles from easy to expert, with
solutions.

## Variables

Every slot is a variable: a pair of islands in the same row or column with
only empty cells between them. Slots are ordered by their first island in
row-major order, the slot to the right before the slot below. A slot's
candidates are 0, 1 and 2 bridges, up to the smaller of its islands'
numbers. Two slots cross when they pass through the same cell.

## Rule instances

| Instance | Rule | Broken when |
| --- | --- | --- |
| Each island | R1 | no choice of its slots' candidates adds up to its number |
| Each pair of crossing slots | R2 | both need at least one bridge |
| The whole board | R3 | the slots that may still hold bridges don't join every island |

With one candidate in every slot, the check is exactly "some rule is
broken".

## Techniques

Rater version 1 uses the profile `hashi-human` version 1.

| Technique | Level | What it does |
| --- | --- | --- |
| `island_complete` | bookkeeping | An island whose fixed bridges reach its number gets no more. |
| `crossing` | bookkeeping | A slot that needs a bridge blocks the slots it crosses. |
| `island_bounds` | Easy | Keeps only the counts that appear in some way to reach the island's number, such as an 8, a 6 on an edge or a 4 in a corner taking two bridges everywhere, a 7 taking at least one in every direction, or a single neighbour. |
| `isolated_pair` | Easy | Two 1s can't be joined by one bridge, and two 2s can't be joined by two, since the pair would be cut off; unless they are the only islands. |
| `only_connection` | Medium | A slot that is the only remaining connection between two parts of the puzzle needs at least one bridge. |
| `forced` | Medium | A count that breaks a rule as soon as it is written in, after its bookkeeping, is removed, such as a bridge that closes off a group of finished islands. |
| `short_contradiction` | Hard | If a slot had a count, easy and medium techniques would break a rule within 6 steps, so that count goes. |
| `contradiction` | Expert | The same within 20 steps, using techniques up to hard. |

Examples (`?` is a slot that is still undecided):

```text
island_bounds          isolated_pair
  4 ? . 3                1 ? . 1
  ?                      .
  .                      3
  2
                         the 1s can't be joined:
  the 4 has two          they would be cut off
  neighbours, so it      from the 3
  takes two bridges
  to each
```

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| rows | yes | 1-15 | 7 |
| columns | yes | 1-15 | 7 |

The shape is written `<rows>x<columns>`, such as `7x7`.

Generator version 1 grows a random network: starting from an island in a
random cell, it repeatedly runs one or two bridges from a random island to a
new island at a free cell in line with it, never next to another island,
until a third of the cells are islands or it has made 20 tries per island
wanted. It then adds single bridges between islands that see each other, one
in eight free slots (one in three for hard and expert), so that the network
has loops. The numbers come from the bridges.

Screening runs the exact solver first. If the puzzle has two solutions, the
slots where they differ count as unresolved; otherwise the techniques up to
the requested level decide. A change picks an unresolved slot (any slot
when the puzzle is too easy) and adds a bridge, changes one bridge to two or
two to one, removes the bridges if the network stays connected, or puts a
new island in the middle of them. The network starts over after 30 rejected
changes in a row or 40 changes without getting closer to the requested
level.

In testing, grids up to 4x4 gave only easy puzzles and 5x5 gave no expert
ones. Generation there stops at its attempt limit, which takes well under a
second.

## Formats

```text
genre: hashi
grid:
2.1.3
.....
4.2.6
.....
3...3
```

Rows have one character per cell: `.` for an empty cell or an island's
number. JSON:

```json
{"genre":"hashi","puzzle":{"grid":["2.1.3",".....","4.2.6",".....","3...3"]}}
```

IDs list the cells row-major: a digit for an island and a letter for a run
of empty cells (`a` is one cell, `z` is 26), such as
`hashi:5x5:2a1a3e4a2a6e3c3`.

Answers are grids with an island's number (or `.`) in island cells, and in
every other cell the bridges passing through it: `.` for none, `-` and `=`
for one and two horizontal bridges, `|` and `H` for one and two vertical
bridges. Every cell a slot passes through shows the same bridges. Where
bridges cross, which breaks R2, the cell shows both, horizontal first (`-|`,
`=H`), and every cell is written as a space-separated token.

A bridge that is not drawn is undecided. When the player says they have
finished, undrawn bridges count as none, which can break R1 and R3.
Violations carry `number` and `bridges` (the bridges drawn) at the island
for R1, nothing at the cell where bridges cross for R2, and `groups` (how
many groups the islands would form) on the whole board for R3.

In the web app and in steps, each slot is a link between its two islands,
with the value `.`, `1` or `2` (`0`, `1` or `2` in steps).

## Performance

Measured with `cargo run --release -p pencil-puzzles --example benchmark -- hashi 20`,
seeds 0-19 and default limits, in the run described in
[docs/benchmarks](../benchmarks/README.md). Failures count as infinite
time. Operation counts are in [`hashi.csv`](../benchmarks/hashi.csv).

| Size | Level | Found | Median | 95th percentile | Mean attempts |
| --- | --- | --- | --- | --- | --- |
| 5x5 | any | 20/20 | 0.1 ms | 0.3 ms | 2 |
| 5x5 | easy | 20/20 | 0.1 ms | 0.4 ms | 3 |
| 5x5 | medium | 20/20 | 8.4 ms | 39 ms | 414 |
| 5x5 | hard | 20/20 | 35 ms | 155 ms | 1330 |
| 5x5 | expert | 0/20 | — | — | — |
| 7x7 | any | 20/20 | 0.3 ms | 0.9 ms | 5 |
| 7x7 | easy | 20/20 | 0.3 ms | 1.2 ms | 5 |
| 7x7 | medium | 20/20 | 7.0 ms | 53 ms | 103 |
| 7x7 | hard | 20/20 | 18 ms | 60 ms | 149 |
| 7x7 | expert | 20/20 | 355 ms | 961 ms | 2194 |
| 10x10 | any | 20/20 | 0.8 ms | 1.4 ms | 2 |
| 10x10 | easy | 20/20 | 0.9 ms | 11 ms | 6 |
| 10x10 | medium | 20/20 | 20 ms | 126 ms | 82 |
| 10x10 | hard | 20/20 | 38 ms | 232 ms | 123 |
| 10x10 | expert | 20/20 | 250 ms | 1.1 s | 415 |
| 12x12 | any | 20/20 | 1.7 ms | 3.6 ms | 8 |
| 12x12 | easy | 20/20 | 1.8 ms | 6.0 ms | 10 |
| 12x12 | medium | 20/20 | 31 ms | 85 ms | 61 |
| 12x12 | hard | 20/20 | 74 ms | 336 ms | 116 |
| 12x12 | expert | 18/20 | 297 ms | — | 257 |
| 15x15 | any | 20/20 | 3.7 ms | 22 ms | 5 |
| 15x15 | easy | 20/20 | 5.8 ms | 34 ms | 14 |
| 15x15 | medium | 20/20 | 38 ms | 480 ms | 56 |
| 15x15 | hard | 20/20 | 150 ms | 577 ms | 119 |
| 15x15 | expert | 14/20 | 1.3 s | — | 294 |

Largest practical size (B1): easy 15x15, medium 15x15, hard 15x15, expert
10x10. Rating a corpus puzzle takes at most 3.5 ms. In WebAssembly
([`web/benchmark.mjs`](../../web/benchmark.mjs), 7x7), generation took 0.84
times the native time.

## References

Hashi (Hashiwokakero, "build bridges") was first published by Nikoli. It
is also known as Bridges (`bridges`) and Hashiwokakero (`hashiwokakero`).
