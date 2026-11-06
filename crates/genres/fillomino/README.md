# pencil-puzzles-fillomino

Fillomino for [pencil-puzzles](../../../README.md): solving, checking, rating, hints
and generation. The rules, formats and techniques are in
[docs/genres/fillomino.md](../../../docs/genres/fillomino.md).

The `serde` feature adds JSON. The `pencil-puzzles` crate re-exports this one
as `pencil_puzzles::fillomino` and registers it for the command-line tool and web app.
