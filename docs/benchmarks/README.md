# Benchmarks

Generation and rating times for every genre, measured on one machine. Times
are comparable within a run. Operation counts are deterministic and the same
on every machine.

## Runs

| Genres | Code | Date |
| --- | --- | --- |
| fillomino, hashi, hitori, kakurasu | pencil-puzzles 0.2.0 | 2026-11-06 |
| kakuro, nurikabe, sudoku | pencil-puzzles 0.3.0 | 2026-11-08 |

Both runs used the same setup:

- Machine: Intel Core i7-8700K, Windows 11 Pro, otherwise idle
- Rust: rustc 1.99.1, release profile
- WebAssembly: Node.js 24.17.0

## Native

```sh
cargo run --release -p pencil-puzzles --example benchmark -- <genre> 20
```

Each genre's sizes are listed in
[`benchmark.rs`](../../crates/pencil-puzzles/examples/benchmark.rs). For
each size and level, the benchmark generates seeds 0-19 with the
command-line tool's limits (10,000 attempts and 100,000,000 operations),
then rates every corpus puzzle. `<genre>.csv` has one row per size and level:

| Column | Meaning |
| --- | --- |
| `shape`, `requested` | Size and requested level (`any` for none) |
| `success` | Seeds that produced a puzzle |
| `median_ms`, `p95_ms` | Generation time over all seeds, failures as infinite |
| `mean_attempts` | Attempts per puzzle produced |
| `mean_operations` | Operations per seed, including failures |
| `easy` … `expert` | Levels of the puzzles produced |

The rows after `corpus,level,rating_ms` give the time to rate each corpus
puzzle.

## WebAssembly

```sh
node web/build.mjs
node web/benchmark.mjs <genre> <default size> 20
```

This times the web app's engine with its limits (10,000 attempts and
100,000,000 operations). Each genre page gives the WebAssembly median divided
by the native median, as a geometric mean over the levels where both found
all 20 puzzles and the native median is at least 10 ms.
