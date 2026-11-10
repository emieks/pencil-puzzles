---
description: Install pencil-puzzles, generate a puzzle, read its clues, solution and rating, and solve or check a puzzle in a few lines of Python.
---

# Quickstart

Install the package:

```sh
pip install pencil-puzzles
```

Everything is in one module:

```python
import pencil_puzzles as pp
```

## Generate a puzzle

```pycon
>>> g = pp.generate("sudoku", (4, 4), difficulty="easy", seed=1)
>>> print(g.puzzle)
3 1 | . .
. . | . .
----+----
. . | . .
. . | 3 4
```

`generate` returns the puzzle together with its solution and rating. Leave
out the shape for the genre's standard size, the difficulty for any level and
the seed for a random puzzle.

```pycon
>>> g.solution.cells
((3, 1, 4, 2), (2, 4, 1, 3), (4, 3, 2, 1), (1, 2, 3, 4))
>>> g.rating.level
<Difficulty.EASY: 'easy'>
```

## Save and load it

A puzzle's ID holds the whole puzzle:

```pycon
>>> g.puzzle.id
'sudoku:4x4:31............34'
>>> puzzle = pp.Puzzle.from_id('sudoku:4x4:31............34')
```

## Solve and rate it

```pycon
>>> pp.solve(puzzle) == g.solution
True
>>> pp.rate(puzzle).techniques
('hidden_single',)
```

## Check a player's answer

An answer is a grid of rows, with 0 for an empty cell:

```pycon
>>> answer = pp.Solution("sudoku", ((3, 1, 3, 0), (0, 0, 0, 0), (0, 0, 0, 0), (0, 0, 3, 4)))
>>> pp.check(puzzle, answer)
[Violation(rule=1, place={'row': 0}, values={'filled': 3, 'size': 4}), Violation(rule=2, place={'col': 2}, values={'filled': 2, 'size': 4})]
```

`check` lists the rules the answer breaks: the 3 repeats in row 0 (rule 1)
and in column 2 (rule 2). The rule numbers are the ones on the [Sudoku
page](genres/sudoku.md#rules).

```pycon
>>> answer = pp.Solution("sudoku", ((3, 1, 4, 0), (0, 0, 0, 0), (0, 0, 0, 0), (0, 0, 3, 4)))
>>> pp.check(puzzle, answer)
[]
>>> pp.hint(puzzle, answer)
Step(level=<Difficulty.EASY: 'easy'>, technique='hidden_single', removed=((1, 2, '2'), (1, 3, '2'), (2, 3, '2')))
```

`hint` gives the next step a solver would take from the answer.

## Next

- [Generating](guide/generating.md): sizes, difficulty and seeds
- [Saving and loading](guide/saving.md): clues, IDs and the text format
- [Solving](guide/solving.md)
- [Rating](guide/rating.md)
- [Answers and hints](guide/answers.md)
- [Long-running calls](guide/long-running.md): limits, cancellation and threads
- [API reference](api.md)
