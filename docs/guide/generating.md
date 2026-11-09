# Generating puzzles

```python
pp.generate(genre, shape=None, *, difficulty=None, seed=None, limits=Limits(), cancel=None)
```

`generate` returns a new puzzle with exactly one solution, together with that
solution and its rating.

## Genre

`genre` is one of the names from `pp.genres()`:

```pycon
>>> pp.genres()
('fillomino', 'hashi', 'hitori', 'kakurasu', 'kakuro', 'nurikabe', 'sudoku')
```

An unknown name raises `UnsupportedRequestError`.

## Size

`shape` is `(rows, columns)`. Without it, the puzzle has the genre's
standard size, such as 9×9 for sudoku and 10×10 for nurikabe. Each genre
supports a range of sizes, listed on its page under Parameters:

```pycon
>>> pp.generate("sudoku", (5, 5))
Traceback (most recent call last):
  ...
pencil_puzzles.UnsupportedRequestError: character 1: Sudoku sizes are 1x1, 4x4 and 9x9
```

Larger puzzles take longer to generate. The [benchmarks](../benchmarks/README.md)
list typical times for each genre, size and level.

## Difficulty

`difficulty` is `"easy"`, `"medium"`, `"hard"` or `"expert"`, or a
`pp.Difficulty`. The generated puzzle has exactly that level: the hardest
technique it needs is at that level. Without `difficulty`, the puzzle can
have any level.

```pycon
>>> g = pp.generate("sudoku", difficulty="hard", seed=7)
>>> g.rating.level
<Difficulty.HARD: 'hard'>
>>> g.rating.techniques
('claiming', 'hidden_pair', 'hidden_single', 'naked_pair', 'naked_single', 'pointing')
```

[How puzzles are rated](../difficulty.md) describes the levels, and each
genre page lists its techniques by level.

## Seeds

`seed` is an integer from 0 to 2⁶⁴−1. The same genre, shape, difficulty,
seed and package version always give the same puzzle:

```pycon
>>> a = pp.generate("nurikabe", (7, 7), difficulty="medium", seed=12)
>>> b = pp.generate("nurikabe", (7, 7), difficulty="medium", seed=12)
>>> a.puzzle == b.puzzle
True
```

Without `seed`, the seed is random. It is kept in `Generated.seed`, so any
puzzle can be generated again. A genre's generator version changes only in a
minor or major release, and `Generated.generator_version` records it.

To make a set of puzzles, use a range of seeds:

```python
puzzles = [pp.generate("hitori", (6, 6), difficulty="easy", seed=s).puzzle
           for s in range(100)]
```

## The result

`generate` returns a `Generated`:

| Field | Value |
| --- | --- |
| `puzzle` | The clues, as a [puzzle](saving.md) |
| `solution` | The only solution, as a `Solution` |
| `rating` | The `Rating` |
| `seed` | The seed used, including a random one |
| `generator_version` | The genre's generator version |
| `attempts` | How many candidate puzzles were tried |

```pycon
>>> g = pp.generate("nurikabe", (7, 7), difficulty="medium", seed=12)
>>> print(g.puzzle)
1 . . . 1 . .
. . . 2 . 1 .
. . . . . . .
. . 3 . . . .
. . . . 3 . .
. . . . . . .
. . 6 . 3 . .
>>> g.attempts
93
```

The generator tries candidate puzzles until one has the requested level.
`limits` bounds how many it tries and how much work it does; see [Long-running
calls](long-running.md).
