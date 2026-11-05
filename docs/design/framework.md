# Solving and generation framework

How every genre solves, rates and generates puzzles. The shared parts live in
`pencil-puzzles-core`; a genre supplies its variables, techniques and edit
moves.

| Shared (core) | Per genre |
| --- | --- |
| Candidates, step record, solve loop, level assignment | Variables, values and the starting candidates |
| Bookkeeping versus skill | Its bookkeeping technique |
| Rating, hints, step trace | Its technique list, each with a level |
| Generator loop: build, screen, edit, certify, search | How to build a candidate and how to change one |
| | A brute-force solver for uniqueness |

## Solving

**Candidates.** One candidate mask per variable (at most 16 values), which
variables are placed, and the genre's read-only context (the puzzle, its
lines or regions). Techniques read candidates; only the solve loop changes
them.

**Technique.** A stable id, a level (easy to expert) and a kind: bookkeeping
(routine consequences of placed values, such as removing a placed digit from
its peers) or skill. A technique returns every productive step it finds, in a
fixed order. It must be sound and must not look at the solution. Techniques
that test a hypothesis ("if this cell were X, a rule breaks") are marked as
probing and are not used inside another hypothesis.

**Step.** Technique name, level, the candidate changes, the cells it relies
on, what it places (if anything), and which earlier steps it depends on.

**Profile.** A genre's techniques in order: bookkeeping first, then easiest to
hardest. It also holds the genre's "broken" check, which must not fire while a
solution remains and must be exact on complete grids.

**Solve loop.** Repeat: ask each technique in profile order, apply the first
step of the first technique that finds one, and record it. Stop when solved,
broken or stuck. The level is the hardest technique used (see
[difficulty.md](../difficulty.md)). Every step is sound, so solving to the end
also proves the puzzle has one solution; the brute-force solver checks every
generated puzzle as well.

**Rating and hints.** A rating lists the level, the skill techniques used, the
number of skill steps and the rater version. A hint applies bookkeeping from
the player's marks and returns the first skill step.

## Generating

One loop for every genre:

1. **Build** a candidate (a full sudoku grid with all clues, a shaded
   kakurasu grid, a grown nurikabe sea).
2. **Screen** it with the solve loop capped at the requested level. If it gets
   stuck, the unresolved variables show where to edit.
3. **Edit** (remove or restore a clue, flip a cell, reshape an island) and
   screen again, keeping edits that do not make the result worse.
4. **Certify** the final candidate: full rating at the requested level and the
   brute-force uniqueness check.
5. **Search** when the result is too easy: keep the best candidate so far,
   perturb it, and restart after a fixed number of tries.

Seeds, attempt counts and restart limits are part of the generator version.
The only quality rule so far is sudoku's clue symmetry below expert.

| | Sudoku | Kakurasu | Nurikabe |
| --- | --- | --- | --- |
| Variables / values | cells / digits | cells / shaded or not | cells / land or sea |
| Bookkeeping | a placed digit leaves its peers | a line already at its total | wall off a completed island; the rest is sea once every island is complete |
| Build / edit | full grid / remove or restore a clue | shaded grid / flip a cell | grown sea / reshape an island |

## Interfaces

- `pencil_puzzles_core::human`: `Technique`, `Profile` (with `next`,
  `analyze`, `refute` and `contradictions`), `Candidates`, and helpers that
  build the public rating, step trace and hint.
- `pencil_puzzles_core::search`: the `Construct` trait (`build`, `puzzle`,
  `screen`, `change`, `keep`, `finished`, `patience`, `certify`) and the
  `search` loop.

The Rust crates are not published, so these interfaces are internal. Only the
Python API and the file formats are public.
