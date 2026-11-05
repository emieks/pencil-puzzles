# Contributing

Contributions of every size are welcome. Everyone taking part follows the
[code of conduct](CODE_OF_CONDUCT.md).

## Ways to help

- **Report a bug.** Include the command or code you ran, the puzzle (its text
  or ID), what you expected, and what happened.
- **Report a rating that seems wrong.** Include the puzzle, the rating it got,
  the rating you'd expect, and why: for example, the step that seems harder or
  easier than its level. See [how puzzles are rated](docs/difficulty.md).
- **Make something faster.** Generating large grids is slow for some genres;
  each genre page lists its practical sizes.
- **Add a genre.** See [Adding a genre](#adding-a-genre) below.

Every genre follows the [guidelines](docs/guidelines.md) and rates puzzles on
the shared [difficulty scale](docs/difficulty.md). Please read both before
adding or changing a genre. The shared engine is described in
[the framework design](docs/design/framework.md).

## Getting started

You need [Rust](https://rustup.rs). Then:

```sh
cargo test --workspace --all-features
```

Before opening a pull request, also run:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

CI runs these on Linux, macOS and Windows, plus WebAssembly, Python wheel and
Rust 1.88 checks.

## Layout

```text
crates/
  core/            shared types, the human solve loop and the generator loop
  genres/<genre>/  one library crate per genre, named pencil-puzzles-<genre>
  pencil-puzzles/  umbrella crate with every genre behind a feature
  cli/             the `pencil-puzzles` command, using the genre registry
  python/          the native module of the Python package
  web/             the WebAssembly engine of the web app
python/            the Python package and its tests
web/               the static web app
corpus/            known puzzles per genre, used as regression tests
docs/
  guidelines.md    what every genre must guarantee
  difficulty.md    how puzzles are rated
  genres/          one page per genre, and the template for new ones
  design/          the solving framework and the Python API
  benchmarks/      recorded generator benchmarks
```

To see how a genre's generator performs after a change, run the shared
benchmark (20 seeds per size and level here):

```sh
cargo run --release -p pencil-puzzles --example benchmark -- kakurasu 20
```

If a change alters generated puzzles or ratings, regenerate the corpus and
golden files with the genre's corpus example, for example
`cargo run --release -p pencil-puzzles-sudoku --example sudoku_corpus -- --write`
(kakurasu: `--example corpus`; nurikabe: `--example nurikabe_corpus`).

The Python package builds with [maturin](https://www.maturin.rs); see
[python/README.md](python/README.md). The web app needs Node.js; see
[web/README.md](web/README.md).

## Adding a genre

Open an issue first, so that nobody else starts on the same genre, then follow
the [checklist in the guidelines](docs/guidelines.md#adding-a-genre).
