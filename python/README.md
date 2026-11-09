# pencil-puzzles for Python

Generate, solve, rate and check pencil puzzles such as sudoku and nurikabe.
Generated puzzles have exactly one solution and a difficulty from easy to
expert, set by the hardest solving technique they need.

Requires CPython 3.10 or newer on Windows, macOS or Linux:

```sh
pip install pencil-puzzles
```

```python
import pencil_puzzles as pp

g = pp.generate("sudoku", difficulty="hard")
print(g.puzzle)
print(g.rating.level, g.rating.techniques)

assert pp.solve(g.puzzle) == g.solution
print(pp.hint(g.puzzle))  # the first step a solver would take
```

Documentation: [quickstart](https://pencil-puzzles.org/docs/quickstart/),
[user guide](https://pencil-puzzles.org/docs/guide/generating/),
[API reference](https://pencil-puzzles.org/docs/api/) and
[genres](https://pencil-puzzles.org/docs/). Source and issues are on
[GitHub](https://github.com/emieks/pencil-puzzles).
