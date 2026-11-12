# pencil-puzzles-cli

The `pencil-puzzles` command: generate, solve, rate, check and give hints
for pencil puzzles (aquarium, binairo, fillomino, hashi, hitori, kakurasu,
kakuro, nurikabe and sudoku). Every generated puzzle has exactly one solution and a
difficulty from easy to expert, set by the hardest human technique it needs.

```sh
cargo install pencil-puzzles-cli
```

Requires Rust 1.88 or newer.

## Commands

| Command | What it does |
| --- | --- |
| `list` | Lists the genres and their aliases |
| `generate <genre>` | Generates puzzles, with `--shape`, `--difficulty`, `--seed` and `--count` |
| `solve` | Solves puzzles and proves each solution is the only one |
| `rate` | Rates puzzles from easy to expert |
| `check` | Checks an answer against the rules |
| `hint` | Gives the next step from a partial answer |
| `completions <shell>` | Prints a completion script for bash, zsh, fish, PowerShell or elvish |

`solve`, `rate`, `check` and `hint` read a file, standard input, or a puzzle
ID with `--id`. Output is text by default; `--format pretty` prints a grid and
`--format json` prints JSON.

```sh
pencil-puzzles generate nurikabe --shape 7x7 --difficulty hard --format pretty
pencil-puzzles generate sudoku --difficulty medium --count 10 --seed 1 > puzzles.txt
pencil-puzzles solve puzzles.txt
pencil-puzzles rate --id 'sudoku:4x4:31............34'
```

`pencil-puzzles <command> --help` lists every option.

## Links

- [Genre rules, techniques and formats](https://pencil-puzzles.org/docs/)
- [Rust library](https://crates.io/crates/pencil-puzzles) and
  [Python package](https://pypi.org/project/pencil-puzzles/)
- [Source](https://github.com/emieks/pencil-puzzles)
