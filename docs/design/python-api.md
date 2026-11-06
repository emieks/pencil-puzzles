# Python package API

Every public name in the `pencil-puzzles` Python package. The API is small on
purpose, since removing things later is harder than adding them. For an
example, see [python/README.md](../../python/README.md).

## Names

| Name | What it is |
| --- | --- |
| `generate(genre, shape, *, difficulty=None, seed=0, limits=Limits(), cancel=None)` | A puzzle with exactly one solution at the requested level (easy to expert), or any level when `difficulty` is `None`. Returns `Generated`. |
| `solve(puzzle, ...)` | The solution; raises `NoSolutionError` or `MultipleSolutionsError` (with two `witnesses`). |
| `rate(puzzle, ...)` | The `Rating`; raises `UnratedPuzzleError` if the supported techniques cannot finish it. |
| `check(puzzle, answer, *, finished=False, ...)` | A list of `Violation`s; empty when the answer breaks no rule. With `finished`, unmarked cells and undrawn bridges first take the genre's finished default. |
| `hint(puzzle, answer=None, ...)` | The next `Step` after routine bookkeeping, or `None` when solved. |
| `genres()` | The genre ids. |
| `Puzzle` (`SudokuPuzzle`, `NurikabePuzzle`, `KakurasuPuzzle`, `FillominoPuzzle`, `HashiPuzzle`, `HitoriPuzzle`, `KakuroPuzzle`) | Immutable clues; `id`, `genre`, `shape`, `to_text()`, `Puzzle.from_id()`, `Puzzle.from_text()`. `KakuroPuzzle.cells` has `None` for a white cell and `(down, across)` for a block, 0 for a missing sum. |
| `Solution` | `genre` and `cells`: numbers for sudoku, fillomino and kakuro (0 unmarked, `None` for kakuro blocks), each cell's text for hashi (`"."`, an island number, `"-"`, `"="`, `"\|"`, `"H"`), `Mark` values otherwise. Also used for partial answers. |
| `Generated` | `puzzle`, `solution`, `rating`, `seed`, `generator_version`, `attempts`. |
| `Rating` | `level`, `techniques`, `steps`, `score` (unstable; see [difficulty.md](../difficulty.md)), `rater_version`. |
| `Step` | `level`, `technique`, `removed` (each `(row, col, value)`, or `(row, col, row2, col2, value)` for the bridges between two hashi islands). |
| `Violation` | `rule` number, `place` and `values`. |
| `Difficulty`, `Mark` | Enums. |
| `Limits(work, attempts)`, `CancellationToken` | Bounds and cancellation for every operation. |
| Errors | `PencilPuzzlesError` and subclasses: `UnsupportedRequestError`, `AttemptsExhaustedError`, `BudgetExceededError`, `CancelledError`, `UnratedPuzzleError`, `NoSolutionError`, `MultipleSolutionsError`, `ParseError`. |

Everything else is private. Native work runs without holding the GIL.
