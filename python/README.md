# pencil-puzzles for Python

Generate, solve, rate and check sudoku, nurikabe, kakurasu and fillomino puzzles.
Generated puzzles have exactly one solution and a difficulty from easy to
expert, set by the hardest solving technique they need.

Requires CPython 3.10 or newer on Windows, macOS or Linux:

```sh
pip install pencil-puzzles
```

```python
import pencil_puzzles as pp

g = pp.generate("sudoku", (9, 9), difficulty="medium", seed=42)
print(g.puzzle.givens)
print(g.rating.level, g.rating.techniques)

assert pp.solve(g.puzzle) == g.solution
assert pp.rate(g.puzzle) == g.rating
assert pp.check(g.puzzle, g.solution) == []
print(pp.hint(g.puzzle))          # the first step a solver would take

same = pp.Puzzle.from_id(g.puzzle.id)
```

The same arguments and package version always give the same puzzle. To save
or share a puzzle, use its ID. Every operation takes
`limits=pp.Limits(work=..., attempts=...)` and `cancel=pp.CancellationToken()`;
running out of a limit raises an error. Cancellation can come from another
thread, and Ctrl-C raises `KeyboardInterrupt`.

All public names are listed in [docs/design/python-api.md](../docs/design/python-api.md).

## Building

```sh
pip install maturin
maturin build --release
pip install target/wheels/pencil_puzzles-*.whl
python -m unittest discover -s python/tests
```
