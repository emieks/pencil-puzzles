# Hashi

Join the islands with bridges so that each island has as many bridges as
its number and every island is connected.

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
wanted. It then adds single
bridges between islands that see each other, one in eight free slots (one in
three for hard and expert), so that the network has loops. The numbers come
from the bridges.

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

Not measured yet. The benchmark
(`cargo run --release -p pencil-puzzles --example benchmark -- hashi 20`)
will be recorded in `docs/benchmarks/hashi.csv` and summarised here.

## References

Hashi (Hashiwokakero, "build bridges") was first published by Nikoli. It
is also known as Bridges (`bridges`) and Hashiwokakero (`hashiwokakero`).
