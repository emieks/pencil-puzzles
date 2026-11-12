"""Generate, solve, rate and check pencil puzzles, backed by a Rust engine.

>>> import pencil_puzzles as pp
>>> g = pp.generate("sudoku", (4, 4), difficulty="easy", seed=1)
>>> pp.solve(g.puzzle) == g.solution
True
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
import json
import random
from typing import Any, NamedTuple
import warnings

from . import _native

__version__ = "0.4.0"
__all__ = [
    "AquariumClues", "AquariumPuzzle", "AttemptsExhaustedError", "BinairoPuzzle", "BudgetExceededError", "CancellationToken", "CancelledError",
    "ChoconaClues", "ChoconaPuzzle",
    "Difficulty", "FillominoPuzzle", "Generated", "HashiPuzzle", "HitoriPuzzle", "KakurasuClues", "KakurasuPuzzle", "KakuroPuzzle", "Limits", "Mark",
    "MultipleSolutionsError", "NoSolutionError", "NurikabePuzzle", "ParseError",
    "PencilPuzzlesError", "Puzzle", "Rating",
    "Solution", "Step", "SudokuPuzzle", "UnratedPuzzleError", "UnsupportedRequestError",
    "Violation", "check", "generate", "genres", "hint", "rate", "solve",
]
_U64 = (1 << 64) - 1
_U32 = (1 << 32) - 1


class Difficulty(str, Enum):
    """How hard a puzzle is: the level of the hardest human technique it needs."""
    EASY = "easy"
    MEDIUM = "medium"
    HARD = "hard"
    EXPERT = "expert"


class Mark(str, Enum):
    """A cell in a nurikabe, kakurasu, hitori, aquarium or chocona answer. In
    aquarium, shaded cells are water."""
    SHADED = "#"
    UNSHADED = "-"
    UNMARKED = "."


class PencilPuzzlesError(Exception):
    """Base class for every error; extra context is available as attributes."""
    def __init__(self, message: str, **context: Any):
        super().__init__(message)
        for name, value in context.items():
            setattr(self, name, value)


class UnsupportedRequestError(PencilPuzzlesError):
    """Unknown genre, unsupported shape or difficulty, or an invalid argument."""


class AttemptsExhaustedError(PencilPuzzlesError):
    """No puzzle was found within `Limits.attempts`."""


class BudgetExceededError(PencilPuzzlesError):
    """The work ran past `Limits.work`."""


class CancelledError(PencilPuzzlesError):
    """The operation's `CancellationToken` was cancelled."""


class UnratedPuzzleError(PencilPuzzlesError):
    """The puzzle is unique, but the supported techniques cannot finish it."""


class NoSolutionError(PencilPuzzlesError):
    """The puzzle has no solution."""


class MultipleSolutionsError(PencilPuzzlesError):
    """The puzzle has more than one solution; `witnesses` holds two of them."""


class ParseError(PencilPuzzlesError):
    """Puzzle text, an ID or an answer could not be read."""


_ERRORS = {"unsupported": UnsupportedRequestError, "attempts": AttemptsExhaustedError,
           "budget": BudgetExceededError, "cancelled": CancelledError,
           "unrated": UnratedPuzzleError, "no_solution": NoSolutionError,
           "multiple_solutions": MultipleSolutionsError, "parse": ParseError}


def _call(function: Any, *args: Any) -> Any:
    try:
        return function(*args)
    except _native.EngineError as exc:
        kind, message, attempts = exc.args
        raise _ERRORS.get(kind, PencilPuzzlesError)(message, attempts=attempts) from exc


def _integer(value: Any, name: str, maximum: int) -> int:
    if type(value) is not int:
        raise TypeError(f"{name} must be an integer")
    if not 0 <= value <= maximum:
        raise ValueError(f"{name} must be between 0 and {maximum}")
    return value


@dataclass(frozen=True)
class Limits:
    """Bounds on an operation. Running out raises an error; a result is the same under any limits."""
    work: int = 100_000_000
    attempts: int = 10_000

    def __post_init__(self) -> None:
        _integer(self.work, "work", _U64)
        _integer(self.attempts, "attempts", _U32)


CancellationToken = _native.CancellationToken


@dataclass(frozen=True)
class Rating:
    """A puzzle's difficulty, from solving it with human techniques."""
    level: Difficulty
    techniques: tuple[str, ...]
    steps: int
    score: int
    """Total size of the step explanations. It changes between releases and does not measure solving time."""

    @classmethod
    def _read(cls, data: dict[str, Any]) -> Rating:
        return cls(Difficulty(data["level"]), tuple(data["techniques"]), data["steps"],
                   data["score"])


@dataclass(frozen=True)
class Step:
    """One deduction: the technique and the candidates it removes, as
    (row, col, value) for a cell, or (row, col, row2, col2, value) for the
    bridges between two hashi islands."""
    level: Difficulty
    technique: str
    removed: tuple[tuple[int, int, str] | tuple[int, int, int, int, str], ...]

    @classmethod
    def _read(cls, data: dict[str, Any]) -> Step:
        def literal(r: dict[str, Any]) -> tuple[Any, ...]:
            place = r["place"]
            if "link" in place:
                link = place["link"]
                return (link["row"], link["col"], link["row2"], link["col2"], r["value"])
            return (place["cell"]["row"], place["cell"]["col"], r["value"])
        return cls(Difficulty(data["level"]), data["technique"], tuple(map(literal, data["removed"])))


@dataclass(frozen=True)
class Violation:
    """A broken rule: its number on the genre page, where, and the values involved."""
    rule: int
    place: dict[str, Any]
    values: dict[str, int]

    @classmethod
    def _read(cls, data: dict[str, Any]) -> Violation:
        place = data["place"] if isinstance(data["place"], dict) else {data["place"]: None}
        return cls(data["rule"], place, {name: value for name, value in data["values"]})


@dataclass(frozen=True)
class Solution:
    """A complete or partial answer: numbers for sudoku, fillomino and kakuro
    (0 for unmarked, None for kakuro blocks), 0 and 1 for binairo (None for
    unmarked), each cell's text for hashi (`"."`, an island number, `"-"`,
    `"="`, `"|"` or `"H"`), `Mark` values otherwise."""
    genre: str
    cells: tuple[tuple[int | str | Mark | None, ...], ...]

    @property
    def shape(self) -> tuple[int, int]:
        return (len(self.cells), len(self.cells[0]) if self.cells else 0)

    @classmethod
    def _read(cls, genre: str, rows: list[str]) -> Solution:
        if genre == "sudoku":
            cells = tuple(tuple(int(c) if c.isdigit() else 0 for c in row) for row in rows)
        elif genre == "fillomino":
            cells = tuple(tuple(0 if t == "." else int(t) for t in _tokens(rows, row)) for row in rows)
        elif genre == "hashi":
            cells = tuple(tuple(_tokens(rows, row)) for row in rows)
        elif genre == "kakuro":
            cells = tuple(tuple(None if c == "#" else 0 if c == "." else int(c) for c in row)
                          for row in rows)
        elif genre == "binairo":
            cells = tuple(tuple(None if c == "." else int(c) for c in row) for row in rows)
        else:
            cells = tuple(tuple(Mark(c) for c in row) for row in rows)
        return cls(genre, cells)

    def _wire(self) -> list[str]:
        if self.genre == "hashi":
            if any(type(v) is not str for row in self.cells for v in row):
                raise ParseError('Hashi cells are text, such as ".", "2" or "="')
            spaced = any(len(t) > 1 for row in self.cells for t in row)
            return [(" " if spaced else "").join(row) for row in self.cells]
        if self.genre == "kakuro":
            if any(not (v is None or (type(v) is int and 0 <= v <= 9)) for row in self.cells for v in row):
                raise ParseError("Kakuro cells must be digits, 0 for unmarked or None for blocks")
            return ["".join("#" if v is None else str(v) if v else "." for v in row) for row in self.cells]
        if self.genre == "binairo":
            if any(not (v is None or (type(v) is int and v in (0, 1))) for row in self.cells for v in row):
                raise ParseError("Binairo cells must be 0, 1 or None for unmarked")
            return ["".join("." if v is None else str(v) for v in row) for row in self.cells]
        if self.genre == "fillomino":
            if any(type(v) is not int or v < 0 for row in self.cells for v in row):
                raise ParseError("Fillomino cells must be whole numbers, 0 for unmarked")
            tokens = [[str(v) if v else "." for v in row] for row in self.cells]
            spaced = any(len(t) > 1 for row in tokens for t in row)
            return [(" " if spaced else "").join(row) for row in tokens]

        def text(value: int | Mark) -> str:
            if isinstance(value, Mark):
                return value.value
            if self.genre == "sudoku" and type(value) is int and 0 <= value <= 9:
                return str(value) if value else "."
            return Mark(value).value
        return ["".join(text(v) for v in row) for row in self.cells]


@dataclass(frozen=True)
class Puzzle:
    """A puzzle's clues. Use `Puzzle.from_id` or `Puzzle.from_text` to load one."""
    genre: str
    shape: tuple[int, int]
    id: str
    _handle: Any = field(repr=False, compare=False)

    @staticmethod
    def from_id(puzzle_id: str) -> Puzzle:
        """Read a puzzle ID such as ``sudoku:4x4:...``."""
        return _puzzle(_call(_native.from_id, puzzle_id))

    @staticmethod
    def from_text(source: str) -> Puzzle:
        """Read a puzzle in the shared text format."""
        return _puzzle(_call(_native.from_text, source))

    def to_text(self) -> str:
        """The puzzle in the shared text format."""
        return self._handle.to_text()

    def __str__(self) -> str:
        """The grid laid out for reading, as `print(puzzle)` shows it."""
        return self._handle.render()


def _alias(old: str, get: Any) -> property:
    """A deprecated name for part of `clues`."""
    def read(self: Any) -> Any:
        warnings.warn(f"{old} is deprecated; use clues", DeprecationWarning, stacklevel=2)
        return get(self)
    return property(read, doc="Deprecated; use `clues`.")


@dataclass(frozen=True)
class SudokuPuzzle(Puzzle):
    clues: tuple[tuple[int, ...], ...]
    """Digits, 0 for empty cells."""
    givens = _alias("givens", lambda p: p.clues)


@dataclass(frozen=True)
class NurikabePuzzle(Puzzle):
    clues: tuple[tuple[int, ...], ...]


@dataclass(frozen=True)
class FillominoPuzzle(Puzzle):
    clues: tuple[tuple[int, ...], ...]


@dataclass(frozen=True)
class HashiPuzzle(Puzzle):
    clues: tuple[tuple[int, ...], ...]
    """Island numbers, 0 for empty cells."""


@dataclass(frozen=True)
class HitoriPuzzle(Puzzle):
    clues: tuple[tuple[int, ...], ...]
    numbers = _alias("numbers", lambda p: p.clues)


@dataclass(frozen=True)
class KakuroPuzzle(Puzzle):
    clues: tuple[tuple[tuple[int, int] | None, ...], ...]
    """Per cell: None for a white cell, (down, across) sums for a block, 0 for
    a missing sum."""
    cells = _alias("cells", lambda p: p.clues)


class KakurasuClues(NamedTuple):
    """Kakurasu clues: one sum per row and one per column."""
    rows: tuple[int, ...]
    cols: tuple[int, ...]


@dataclass(frozen=True)
class KakurasuPuzzle(Puzzle):
    clues: KakurasuClues
    row_clues = _alias("row_clues", lambda p: p.clues.rows)
    col_clues = _alias("col_clues", lambda p: p.clues.cols)


class AquariumClues(NamedTuple):
    """Aquarium clues: a water count per row and per column, and each cell's
    tank. Tanks are numbered from 0 in the order their first cells appear,
    reading row by row."""
    rows: tuple[int, ...]
    cols: tuple[int, ...]
    tanks: tuple[tuple[int, ...], ...]


@dataclass(frozen=True)
class AquariumPuzzle(Puzzle):
    clues: AquariumClues


@dataclass(frozen=True)
class BinairoPuzzle(Puzzle):
    clues: tuple[tuple[int | None, ...], ...]
    """The given digits, 0 or 1, and None for empty cells."""


class ChoconaClues(NamedTuple):
    """Chocona clues: each cell's region, and each region's number (``None``
    for a region without one). Regions are numbered from 0 in the order their
    first cells appear, reading row by row."""
    regions: tuple[tuple[int, ...], ...]
    numbers: tuple[int | None, ...]


@dataclass(frozen=True)
class ChoconaPuzzle(Puzzle):
    clues: ChoconaClues


def _tokens(rows: list[str], row: str) -> list[str]:
    """A grid row's cells: space-separated when any row of the grid has a space."""
    return row.split(" ") if any(" " in r for r in rows) else list(row)


def _puzzle(handle: Any) -> Puzzle:
    data = handle.data()
    genre, puzzle_id, clues = data["genre"], data["id"], data["puzzle"]
    shape = tuple(map(int, puzzle_id.split(":")[1].split("x")))
    common = (genre, shape, puzzle_id, handle)
    if genre == "sudoku":
        rows = clues["grid"] if isinstance(clues, dict) else clues
        return SudokuPuzzle(*common, tuple(tuple(int(c) if c.isdigit() else 0 for c in row) for row in rows))
    if genre in ("nurikabe", "fillomino", "hashi"):
        rows = clues["grid"]
        values = tuple(tuple(int(c) if c != "." else 0 for c in _tokens(rows, row)) for row in rows)
        kind = {"nurikabe": NurikabePuzzle, "fillomino": FillominoPuzzle, "hashi": HashiPuzzle}[genre]
        return kind(*common, values)
    if genre == "hitori":
        rows = clues["grid"]
        return HitoriPuzzle(*common, tuple(tuple(int(t) for t in _tokens(rows, row)) for row in rows))
    if genre == "kakuro":
        rows = clues["grid"]

        def cell(token: str) -> tuple[int, int] | None:
            if token == ".":
                return None
            if token == "#":
                return (0, 0)
            down, _, across = token.partition("/")
            return (int(down or 0), int(across or 0))
        return KakuroPuzzle(*common, tuple(tuple(cell(t) for t in _tokens(rows, row)) for row in rows))
    if genre == "binairo":
        rows = clues["grid"]
        return BinairoPuzzle(*common, tuple(tuple(None if c == "." else int(c) for c in row) for row in rows))
    if genre == "aquarium":
        rows = clues["regions"]
        letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        tanks = tuple(tuple(letters.index(t) if t in letters else int(t) - 1 for t in _tokens(rows, row))
                      for row in rows)
        return AquariumPuzzle(*common, AquariumClues(tuple(clues["row_clues"]), tuple(clues["col_clues"]), tanks))
    if genre == "chocona":
        rows = clues["regions"]
        letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
        regions = tuple(tuple(letters.index(t) if t in letters else int(t) - 1 for t in _tokens(rows, row))
                        for row in rows)
        return ChoconaPuzzle(*common, ChoconaClues(regions, tuple(clues["clues"])))
    return KakurasuPuzzle(*common, KakurasuClues(tuple(clues["row_clues"]), tuple(clues["col_clues"])))


@dataclass(frozen=True)
class Generated:
    """A generated puzzle with its solution, rating and how to reproduce it."""
    puzzle: Puzzle
    solution: Solution
    rating: Rating
    seed: int
    attempts: int


def genres() -> tuple[str, ...]:
    """The supported genre ids."""
    return tuple(row["id"] for row in _native.metadata())


def generate(genre: str, shape: tuple[int, int] | None = None, *,
             difficulty: Difficulty | str | None = None, seed: int | None = None,
             limits: Limits = Limits(), cancel: CancellationToken | None = None) -> Generated:
    """A puzzle with exactly one solution, at exactly `difficulty` (any level if None).

    Without `shape`, the puzzle has the genre's standard size. Without `seed`, the seed is
    random; it is kept in `Generated.seed`. The same seed and package version always give the
    same puzzle.
    """
    if not isinstance(genre, str):
        raise TypeError("genre must be a string")
    size = None
    if shape is not None:
        if not isinstance(shape, tuple) or len(shape) != 2:
            raise TypeError("shape must be a (rows, columns) tuple")
        rows, cols = (_integer(n, "shape", _U32) for n in shape)
        size = f"{rows}x{cols}"
    if seed is None:
        seed = random.getrandbits(64)
    _integer(seed, "seed", _U64)
    level = Difficulty(difficulty) if difficulty is not None else None
    handle, data = _call(_native.generate, genre, size, level.value if level else None,
                         seed, limits.attempts, limits.work, cancel or CancellationToken())
    puzzle = _puzzle(handle)
    return Generated(puzzle, Solution._read(puzzle.genre, data["solution"]), Rating._read(data["rating"]),
                     seed, data["attempts"])


def _operate(puzzle: Puzzle, operation: str, limits: Limits, cancel: CancellationToken | None,
             answer: Solution | None = None, finished: bool = False) -> Any:
    if answer is not None and (answer.genre != puzzle.genre or answer.shape != puzzle.shape):
        raise ParseError("the answer's genre or shape differs from the puzzle's")
    wire = json.dumps(answer._wire()) if answer is not None else None
    return _call(puzzle._handle.operate, operation, limits.work, cancel or CancellationToken(), wire,
                 finished)


def solve(puzzle: Puzzle, *, limits: Limits = Limits(), cancel: CancellationToken | None = None) -> Solution:
    """The puzzle's only solution."""
    result = _operate(puzzle, "solve", limits, cancel)
    if result["status"] == "multiple":
        raise MultipleSolutionsError("the puzzle has more than one solution", witnesses=tuple(
            Solution._read(puzzle.genre, a) for a in result["witnesses"]))
    return Solution._read(puzzle.genre, result["solution"])


def rate(puzzle: Puzzle, *, limits: Limits = Limits(), cancel: CancellationToken | None = None) -> Rating:
    """The human techniques the puzzle needs."""
    return Rating._read(_operate(puzzle, "rate", limits, cancel))


def check(puzzle: Puzzle, answer: Solution, *, finished: bool = False, limits: Limits = Limits(),
          cancel: CancellationToken | None = None) -> list[Violation]:
    """Every rule the answer breaks; empty when it breaks none.

    With `finished`, unmarked cells and undrawn bridges first take the
    genre's default for a finished answer, such as no bridge in hashi.
    """
    if not isinstance(finished, bool):
        raise TypeError("finished must be True or False")
    return [Violation._read(v) for v in _operate(puzzle, "check", limits, cancel, answer, finished)]


def hint(puzzle: Puzzle, answer: Solution | None = None, *, limits: Limits = Limits(),
         cancel: CancellationToken | None = None) -> Step | None:
    """The next step a solver would take from `answer`, or None if it is already solved."""
    result = _operate(puzzle, "hint", limits, cancel, answer)
    return None if result == "solved" else Step._read(result["step"])
