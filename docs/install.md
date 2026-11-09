# Installation

## Python

```sh
pip install pencil-puzzles
```

Requires CPython 3.10 or newer on Windows, macOS or Linux. To check the
install:

```sh
python -c "import pencil_puzzles as pp; print(pp.genres())"
```

Next: the [quickstart](quickstart.md).

## Command line

With Rust 1.88 or newer:

```sh
cargo install pencil-puzzles-cli
```

This installs the `pencil-puzzles` command:

```sh
pencil-puzzles generate nurikabe --shape 7x7 --difficulty hard --format pretty
pencil-puzzles list
pencil-puzzles --help
```

The commands are `list`, `generate`, `solve`, `rate`, `check` and `hint`.
`pencil-puzzles <command> --help` lists each command's options.

## Rust

```sh
cargo add pencil-puzzles
```

Every genre is included by default. To build fewer, turn off default features
and pick genres:

```sh
cargo add pencil-puzzles --no-default-features --features sudoku,nurikabe
```

The `serde` feature adds serialization for puzzles, solutions and ratings.
The API reference is on [docs.rs](https://docs.rs/pencil-puzzles).

## Building from source

The Python package is built with [maturin](https://www.maturin.rs/):

```sh
git clone https://github.com/emieks/pencil-puzzles
cd pencil-puzzles
pip install maturin
maturin build --release
pip install target/wheels/pencil_puzzles-*.whl
```
