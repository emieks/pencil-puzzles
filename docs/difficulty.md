# Difficulty

Every genre rates puzzles by the human techniques they need, on one shared
scale. This page defines the scale and the rules every rater follows. The
engine is described in [the framework design](design/framework.md), and each
genre lists its techniques on its own page. A rating names its genre,
technique profile and rater version.

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
| [Sudoku](genres/sudoku.md) | singles | pointing, claiming, pairs | triples, quads, X-wing, swordfish, short chains, wings, simple coloring | jellyfish, extended coloring, X- and XY-chains |
| [Nurikabe](genres/nurikabe.md) | separating numbers, pools, single exits | unreachable cells, immediate rule breaks | contradiction chains of up to 6 steps | up to 20 steps |
| [Kakurasu](genres/kakurasu.md) | weight bounds | short lists of combinations | long lists, contradiction chains of up to 6 steps | up to 20 steps |

## Rating procedure

The numbers (L1, L14, ...) are cited from code and tests.

**L1-L2. Variables and literals.** A genre's candidates are a mask per
variable (at most 16 values). A literal names one value of one variable in
the genre's answer format.

**L14. Canonical solving.** From the printed puzzle, the solver repeatedly
applies the first step of the first technique, in the profile's order, that
makes progress. Bookkeeping comes first. A puzzle's level is the level of the
hardest technique on this path. The level is exact for this rater version; a
person using other strategies may find a puzzle easier or harder.

**L15-L16. Determinism.** Optimizations must not change results, and a
budget that runs out is an error. No random choice, wall-clock limit, access
to the solution or assumption of uniqueness may justify a deduction.
Techniques must be sound.

**L17-L18. Score.** The score is the total size of the step explanations. It
changes between rater versions and does not estimate solving time. Ratings
may also carry unstable research metrics.

## Rating results

**L19-L20.** Only puzzles with exactly one solution, proved by the genre's
independent solver, are rated. If the techniques cannot finish a unique
puzzle, `rate` returns an "unrated" error. Each step records its technique,
level, removed candidates, the candidates it relies on and the earlier steps
that established them. Hints give the first skill step after bookkeeping and
do not use the solution.

## Generating by difficulty

**L21.** A requested level must equal the full rating of every returned
puzzle. With no level requested, any level from easy to expert is accepted.

**L22.** Internal search limits belong to the generator version. Attempt
limits, work budgets and cancellation can only make generation fail; they
do not change which puzzle is returned.

**L24/V3.** Changes that affect ratings or generated puzzles bump the rater or
generator version, and old versions are removed. A seed reproduces a puzzle
within one generator version, so puzzles are shared by ID.

## Checking a genre's rater

**L23.** Every genre tests that:

1. every technique keeps every solution of random partial states, compared
   with a brute-force enumeration;
2. its "broken" check does not fire while a solution remains, and is exact
   on complete grids;
3. ratings, step traces and generated puzzles are deterministic and match
   the corpus and golden files;
4. work budgets bound every operation.

## Choices made by the genre

**L25.** The genre page lists its technique profile in order, with each
technique's level and an example, says what counts as bookkeeping, and
says how the genre's "broken" check works.
