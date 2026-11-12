# API reference

Every public name in the `pencil_puzzles` package. The [user
guide](quickstart.md) shows how they fit together.

```{eval-rst}
.. module:: pencil_puzzles
```

## Functions

```{eval-rst}
.. autofunction:: generate
.. autofunction:: solve
.. autofunction:: rate
.. autofunction:: check
.. autofunction:: hint
.. autofunction:: genres
```

## Puzzles

```{eval-rst}
.. autoclass:: Puzzle
   :members: genre, shape, id, from_id, from_text, to_text
   :undoc-members:

.. autoclass:: SudokuPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: NurikabePuzzle
   :members: clues
   :undoc-members:

.. autoclass:: FillominoPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: HashiPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: HitoriPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: KakuroPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: AquariumPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: AquariumClues
   :members: rows, cols, tanks

.. autoclass:: BinairoPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: ChoconaPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: ChoconaClues
   :members: regions, numbers

.. autoclass:: KakurasuPuzzle
   :members: clues
   :undoc-members:

.. autoclass:: KakurasuClues
   :members: rows, cols
```

Every puzzle class is a frozen dataclass. `str(puzzle)` gives the grid laid
out for reading. The names `givens`, `numbers`, `cells`, `row_clues` and
`col_clues` still work and are deprecated; use `clues`.

## Answers and results

```{eval-rst}
.. autoclass:: Solution
   :members: genre, cells, shape
   :undoc-members:

.. autoclass:: Generated
   :members:
   :undoc-members:

.. autoclass:: Rating
   :members:
   :undoc-members:

.. autoclass:: Step
   :members:
   :undoc-members:

.. autoclass:: Violation
   :members:
   :undoc-members:

.. autoclass:: Difficulty
   :members:
   :undoc-members:

.. autoclass:: Mark
   :members:
   :undoc-members:
```

## Limits and cancellation

```{eval-rst}
.. autoclass:: Limits
   :members:
   :undoc-members:

.. autoclass:: CancellationToken
   :members:
   :undoc-members:
```

Pass a `CancellationToken` as `cancel` to any function. Calling `cancel()`
from any thread stops every operation it was passed to with
`CancelledError`, and `cancelled` becomes `True`.

## Errors

Every error is a subclass of `PencilPuzzlesError`. Arguments of the wrong
type raise `TypeError` or `ValueError`.

| Error | Raised when |
| --- | --- |
| `UnsupportedRequestError` | The genre, shape, difficulty or another argument is not supported |
| `AttemptsExhaustedError` | `generate` found no puzzle within `Limits.attempts` |
| `BudgetExceededError` | The work ran past `Limits.work` |
| `CancelledError` | The `CancellationToken` was cancelled |
| `UnratedPuzzleError` | The supported techniques cannot finish the puzzle |
| `NoSolutionError` | The puzzle has no solution |
| `MultipleSolutionsError` | The puzzle has more than one solution |
| `ParseError` | Puzzle text, an ID or an answer could not be read |

```{eval-rst}
.. autoexception:: PencilPuzzlesError
.. autoexception:: UnsupportedRequestError
.. autoexception:: AttemptsExhaustedError
.. autoexception:: BudgetExceededError
.. autoexception:: CancelledError
.. autoexception:: UnratedPuzzleError
.. autoexception:: NoSolutionError
.. autoexception:: MultipleSolutionsError
.. autoexception:: ParseError
```
