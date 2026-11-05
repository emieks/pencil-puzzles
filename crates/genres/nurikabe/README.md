# pencil-puzzles-nurikabe

Nurikabe for [pencil-puzzles](../../../README.md): solving, checking, rating, hints
and generation. The rules, formats and techniques are in
[docs/genres/nurikabe.md](../../../docs/genres/nurikabe.md).

The `serde` feature adds JSON. The `pencil-puzzles` crate re-exports this one
as `pencil_puzzles::nurikabe` and registers it for the command-line tool and web app.

License: [MIT](../../../LICENSE)
