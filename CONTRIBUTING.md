# Contributing

Issues and pull requests of any kind are welcome: bug reports, fixes, new
features, new genres, performance work and docs. A draft pull request is a
good place for unfinished work or open questions.

## Ideas

- **Bugs and ratings.** A puzzle's ID (the web app's share link contains it)
  is enough to reproduce most problems, including a level that seems wrong.
- **Performance.** Large grids are slow to generate in some genres. Each
  genre page lists the largest size per level that generates in about a
  second.
- **New genres.** The [guidelines](docs/guidelines.md#adding-a-genre) have a
  checklist, and [docs/genres/TEMPLATE.md](docs/genres/TEMPLATE.md) is a
  starting point for the genre page. An issue lets others know a genre is
  being worked on.
- **Docs.** The docs site is built from the Markdown in `docs/`.

## Running it

With [Rust](https://rustup.rs) installed:

```sh
cargo test --workspace --all-features
```

`cargo fmt --all` formats the code. CI runs the tests on Linux, macOS and
Windows, plus clippy and the WebAssembly and Python builds, on every pull
request.

The Python package builds with [maturin](https://www.maturin.rs):

```sh
pip install maturin
maturin build --release
pip install target/wheels/pencil_puzzles-*.whl
python -m unittest discover -s python/tests
```

The docs site builds with Sphinx into `web/dist/docs/`, using the installed
package for the API reference:

```sh
pip install . -r docs/requirements.txt
sphinx-build -b dirhtml docs web/dist/docs
```

The web app needs Node.js; [web/README.md](web/README.md) shows how to run it
locally.

## Where things are

```text
crates/
  core/            shared types, the human solve loop and the generator loop
  genres/<genre>/  one crate per genre
  pencil-puzzles/  umbrella crate with every genre
  cli/             the pencil-puzzles command
  python/          the native module of the Python package
  web/             the WebAssembly engine of the web app
python/            the Python package and its tests
web/               the web app
corpus/            known puzzles per genre, used as regression tests
docs/              the documentation site, one page per genre
```

[How puzzles are rated](docs/difficulty.md) explains the difficulty scale, and
[the framework design](docs/design/framework.md) the shared engine.

## Handy commands

Benchmark a genre's generator (the last number is seeds per size and level):

```sh
cargo run --release -p pencil-puzzles --example benchmark -- kakurasu 20
```

Regenerate a genre's corpus after a change to its puzzles or ratings:

```sh
cargo run --release -p pencil-puzzles-<genre> --example <genre>_corpus -- --write
```

The project has a short [code of conduct](CODE_OF_CONDUCT.md).
