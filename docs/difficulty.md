# Difficulty

Every genre rates puzzles by the human techniques they need, on one shared
scale. The engine is described in [the framework design](https://github.com/emieks/pencil-puzzles/blob/main/docs/design/framework.md),
and each genre lists its techniques on its own page.

## Terms

- **Rule instance:** one place a rule applies, such as a row, a 2x2 block or
  an island's number.
- **Technique:** a named, sound deduction that a solver can spot and
  explain, with a level and an order in the genre's profile.
- **Bookkeeping:** routine consequences of values already written in, such
  as removing a placed digit from its row. It is applied but does not count
  toward a rating's level or techniques.
- **Broken:** candidates that can no longer satisfy every rule.

## Levels

| Level | Meaning |
| --- | --- |
| Easy | Direct deductions from a single rule or clue. |
| Medium | Standard combinations and small local patterns. |
| Hard | Larger patterns, or short "if this cell were X, a rule would break" chains. |
| Expert | Combinations of several patterns, and longer chains. |
| Extreme | Reserved. No genre generates it yet. |

Levels are skill bands within a genre. An easy sudoku and an easy nurikabe
need not take the same time.

| Genre | Easy | Medium | Hard | Expert |
| --- | --- | --- | --- | --- |
| [Aquarium](genres/aquarium.md) | tanks too big for a line, tanks a line needs | ways to make up a row | contradiction chains of up to 6 steps | up to 20 steps |
| [Binairo](genres/binairo.md) | pairs, gaps | the last digit of a kind in a line, copies of finished lines | contradiction chains of up to 6 steps | up to 20 steps |
| [Chocona](genres/chocona.md) | cells that would bend a rectangle or overfill a region | ways to fill a region | contradiction chains of up to 6 steps | up to 20 steps |
| [Fillomino](genres/fillomino.md) | wall-offs, single exits, room counting, enclosed pairs | cells a region needs, ways to finish a region, small pockets, immediate rule breaks | contradiction chains of up to 6 steps | up to 20 steps |
| [Hashi](genres/hashi.md) | island bounds, isolated pairs | only connections, immediate rule breaks | contradiction chains of up to 6 steps | up to 20 steps |
| [Hitori](genres/hitori.md) | sandwiches, pairs, corners, cut cells | two rounds of routine consequences breaking a rule | contradiction chains of up to 6 steps | up to 20 steps |
| [Kakurasu](genres/kakurasu.md) | weight bounds | short lists of combinations | long lists, contradiction chains of up to 6 steps | up to 20 steps |
| [Kakuro](genres/kakuro.md) | sum combinations, singles, a digit with one place in a run | combinations that fit the candidates of a run | contradiction chains of up to 6 steps | up to 20 steps |
| [Nurikabe](genres/nurikabe.md) | separating numbers, pools, single exits | unreachable cells, immediate rule breaks | contradiction chains of up to 6 steps | up to 20 steps |
| [Sudoku](genres/sudoku.md) | singles | pointing, claiming, naked pairs | hidden pairs, triples, quads | fish, wings, kites, coloring, X- and XY-chains |

## Rating procedure

Code and tests cite these rules by number.

**L1-L2. Variables and literals.** A genre's candidates are a mask per
variable (at most 16 values). A literal names one value of one variable in
the genre's answer format.

**L14. Canonical solving.** From the printed puzzle, the solver repeatedly
applies the first step of the first technique, in the profile's order, that
makes progress. Bookkeeping comes first. A puzzle's level is the level of the
hardest technique on this path. The level is exact for this rater; a person
using other strategies may find a puzzle easier or harder.

**L15-L16. Determinism.** Optimizations must not change results, and a
budget that runs out is an error. A deduction cannot depend on randomness, a
time limit, the solution or an assumption that the puzzle is unique.

**L17-L18. Score.** The score is the total size of the step explanations. It
changes between releases and does not estimate solving time. Ratings can
also include research metrics, which may change in any release.

## Rating results

**L19-L20.** A puzzle is rated only after the genre's brute-force solver
proves it has exactly one solution. If the techniques cannot finish a unique
puzzle, `rate` returns an "unrated" error. Each step records its technique,
level, removed candidates, the candidates it relies on and the earlier steps
that established them. Hints give the first skill step after bookkeeping and
do not use the solution.

## Generating by difficulty

**L21.** A puzzle generated at a requested level has exactly that rating. With
no level requested, any level from easy to expert is accepted.

**L22.** Changing an internal search limit counts as a change to generated
puzzles (V3). Attempt limits, work budgets and cancellation can make
generation fail, and when it succeeds the puzzle is the same under any of
them.

**L24/V3.** Changes that affect ratings or generated puzzles are marked
**(new ratings)** or **(new puzzles)** in the changelog, and only the current
rater and generator are kept. A seed gives the same puzzle until a release
marked **(new puzzles)** for that genre, so puzzles are shared by ID.
