---
description: Solve a puzzle with pencil-puzzles and get its solution, proven to be the only one.
---

# Solving puzzles

`solve(puzzle)` returns the puzzle's solution as a `Solution`:

```pycon
>>> puzzle = pp.Puzzle.from_id('sudoku:4x4:31............34')
>>> pp.solve(puzzle).cells
((3, 1, 4, 2), (2, 4, 1, 3), (4, 3, 2, 1), (1, 2, 3, 4))
```

The solver searches the whole puzzle, so it also proves the solution is the
only one. A puzzle without exactly one solution raises an error:

- `NoSolutionError` when there is no solution.
- `MultipleSolutionsError` when there is more than one. Its `witnesses`
  attribute holds two different solutions.

```python
puzzle = pp.Puzzle.from_text("genre: sudoku\ngrid:\n12..\n....\n....\n....\n")
try:
    pp.solve(puzzle)
except pp.MultipleSolutionsError as error:
    first, second = error.witnesses
```

This makes `solve` a check for puzzles you write or import yourself.

To find out how hard a puzzle is, see [Rating](rating.md).
