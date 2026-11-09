# Web app

A static site for playing and solving every genre in the registry. It runs
the Rust engine compiled to WebAssembly in a Web Worker, so it gives the same
results as the library. `.github/workflows/pages.yml` deploys it to GitHub
Pages when a push to `main` changes the web app, the Rust crates or the
benchmark tables.

The site is static and has no frontend dependencies. `web/build.mjs` reads
the registry and builds the pages for every genre, so the frontend has no
genre-specific code. Saves stay in the browser's `localStorage`.

## Pages

- `/`: the list of genres.
- `/<genre>/player/`: generate a puzzle and play it, with checking, undo and
  saves.
- `/<genre>/solver/`: enter clues or paste an ID, text or JSON, and solve.

`?puzzle=<ID>` opens a specific puzzle. Share links contain the puzzle
without the player's marks.

## Development

Requires Node.js, Rust, the `wasm32-unknown-unknown` target and
`wasm-bindgen-cli` 0.2.105, which must match the `wasm-bindgen` crate version:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.105 --locked
```

Build `web/dist/` and serve it at http://127.0.0.1:4173/:

```sh
node web/build.mjs
node web/serve.mjs
```

Rerun the build after changing `web/src/` or the Rust crates. To serve the
site under a subpath, build with `BASE_PATH=/<path>/`.

Tests:

```sh
cargo test -p pencil-puzzles-web
node --test web/tests/*.test.mjs
```
