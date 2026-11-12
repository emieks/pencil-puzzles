# Answers and hints

These functions work with a player's answer, complete or partial. They are
the pieces of a puzzle app: check what the player has entered, and suggest
the next step.

## Answers

An answer is a `Solution`: `pp.Solution(genre, cells)`, with `cells` as a
tuple of rows. The values depend on the genre:

| Genre | Cell values |
| --- | --- |
| Sudoku, fillomino | Numbers, 0 for unmarked |
| Kakuro | Digits, 0 for unmarked, `None` for a block |
| Binairo | 0 or 1, `None` for unmarked |
| Nurikabe, hitori, kakurasu, aquarium | `pp.Mark.SHADED`, `pp.Mark.UNSHADED` or `pp.Mark.UNMARKED`; in aquarium, shaded cells are water |
| Hashi | Text: `"."` for water, the island number, `"-"` or `"="` for one or two horizontal bridges, `"\|"` or `"H"` for one or two vertical bridges |

The answer must have the puzzle's genre and shape. Solutions from `solve` and
`generate` use the same form, so a solution is also a complete answer.

```python
puzzle = pp.Puzzle.from_id('sudoku:4x4:31............34')
answer = pp.Solution("sudoku", ((3, 1, 4, 0),
                                (0, 0, 0, 0),
                                (0, 0, 0, 0),
                                (0, 0, 3, 4)))
```

## Checking

`check(puzzle, answer)` returns a list of `Violation`s, one for each rule the
answer breaks. The list is empty when it breaks none.

```pycon
>>> wrong = pp.Solution("sudoku", ((3, 1, 3, 0), (0, 0, 0, 0), (0, 0, 0, 0), (0, 0, 3, 4)))
>>> pp.check(puzzle, wrong)
[Violation(rule=1, place={'row': 0}, values={'filled': 3, 'size': 4}), Violation(rule=2, place={'col': 2}, values={'filled': 2, 'size': 4})]
```

Each `Violation` has:

| Field | Value |
| --- | --- |
| `rule` | The rule's number on the genre page, such as R1 |
| `place` | Where: a row, a column, a cell or another part of the grid |
| `values` | The numbers involved |

`check` compares the answer with the rules, not with the solution. An answer
can break no rule and still be wrong; compare it with `pp.solve(puzzle)` to
find that out.

### Unmarked cells

Unmarked cells break no rule, so a partial answer only reports what is
already wrong. With `finished=True`, unmarked cells and undrawn bridges first
take the genre's default for a finished answer, such as unshaded or no
bridge. The result then also reports what is still missing:

```pycon
>>> pp.check(puzzle, answer)
[]
>>> len(pp.check(puzzle, answer, finished=True))
12
```

## Hints

`hint(puzzle, answer=None)` returns the next `Step` a solver would take from
the answer, or `None` when the answer is already solved. Without an answer,
it starts from the clues.

```pycon
>>> pp.hint(puzzle, answer)
Step(level=<Difficulty.EASY: 'easy'>, technique='hidden_single', removed=((1, 2, '2'), (1, 3, '2'), (2, 3, '2')))
```

A `Step` has:

| Field | Value |
| --- | --- |
| `level` | The level of the technique |
| `technique` | The technique's name, as listed on the genre page |
| `removed` | The candidates the step rules out |

Each item in `removed` is `(row, col, value)` for a cell, or `(row, col,
row2, col2, value)` for the bridges between two hashi islands. Rows and
columns count from 0.
