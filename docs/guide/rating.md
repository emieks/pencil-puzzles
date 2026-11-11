# Rating difficulty

`rate(puzzle)` solves the puzzle the way a person would. At each step it uses
the simplest technique that makes progress. The hardest technique it needs
sets the level.

```pycon
>>> rating = pp.rate(pp.generate("sudoku", difficulty="hard", seed=7).puzzle)
>>> rating.level
<Difficulty.HARD: 'hard'>
>>> rating.techniques
('claiming', 'hidden_pair', 'hidden_single', 'naked_pair', 'naked_single', 'pointing')
```

`rate` returns a `Rating`:

| Field | Value |
| --- | --- |
| `level` | The `Difficulty`: easy, medium, hard or expert |
| `techniques` | The techniques used, by name |
| `steps` | The number of deductions |
| `score` | The total size of the step explanations. It changes between releases and does not measure solving time. |

The technique names are listed on each genre page under Techniques. Puzzles
from `generate` already carry their rating in `Generated.rating`.

`rate` raises `UnratedPuzzleError` when the puzzle has one solution but the
supported techniques cannot finish it. It raises `NoSolutionError` or
`MultipleSolutionsError` like [`solve`](solving.md).

`pp.Difficulty` values compare equal to their names, so
`rating.level == "hard"` works. [How puzzles are rated](../difficulty.md)
describes the levels and the method in full.
