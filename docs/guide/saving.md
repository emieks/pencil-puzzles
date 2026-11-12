---
description: Save and load pencil-puzzles puzzles as IDs, text or JSON.
---

# Saving and loading puzzles

A puzzle holds a genre's clues. Every puzzle has:

| Attribute | Value |
| --- | --- |
| `genre` | The genre name, such as `"sudoku"` |
| `shape` | `(rows, columns)` |
| `id` | A one-line ID that holds the whole puzzle |
| `clues` | The clues, in a form that depends on the genre |

Puzzles are immutable and compare equal when their clues are equal.

## Clues by genre

Each genre has its own puzzle class:

| Class | `clues` |
| --- | --- |
| `SudokuPuzzle` | Rows of digits, 0 for an empty cell |
| `NurikabePuzzle`, `FillominoPuzzle` | Rows of numbers, 0 for an empty cell |
| `HashiPuzzle` | Rows of island numbers, 0 for an empty cell |
| `HitoriPuzzle` | Rows of numbers |
| `KakuroPuzzle` | Rows of cells: `None` for a white cell, `(down, across)` for a block, 0 for a missing sum |
| `KakurasuPuzzle` | A `KakurasuClues` with `rows` and `cols` |
| `AquariumPuzzle` | An `AquariumClues` with `rows`, `cols` and `tanks`, which holds each cell's tank number |
| `BinairoPuzzle` | Rows of given digits, 0 or 1, and `None` for an empty cell |

```pycon
>>> g = pp.generate("kakuro", (6, 6), difficulty="easy", seed=4)
>>> g.puzzle.clues[1]
((0, 0), (0, 15), None, None, (12, 0), (16, 0))
```

## Printing

`print(puzzle)` shows the grid laid out for reading:

```pycon
>>> print(g.puzzle)
   #    #  25/   7/    #    #
   #  /15    .    .  12/  16/
   # 3/13    .    .    .    .
  /7    .    . 3/13    .    .
 /15    .    .    .    .    #
   #    #   /3    .    .    #
```

## IDs

The ID is the shortest way to store or share a puzzle. It starts with the
genre and size:

```pycon
>>> pp.generate("sudoku", (4, 4), difficulty="easy", seed=1).puzzle.id
'sudoku:4x4:31............34'
>>> puzzle = pp.Puzzle.from_id('sudoku:4x4:31............34')
```

`Puzzle.from_id` returns the genre's puzzle class. The web app accepts the
same IDs: `https://pencil-puzzles.org/sudoku/player/?puzzle=<ID>` opens the
puzzle there.

## Text format

`to_text()` gives a text form that is easy to read and edit, and
`Puzzle.from_text()` reads it back:

```pycon
>>> print(puzzle.to_text())
genre: sudoku
grid:
31..
....
....
..34
```

You can write puzzles in this format by hand, for example to solve or rate a
puzzle from a book. Each genre page describes its format under Formats.

`from_id` and `from_text` raise `ParseError` when the input cannot be read.
