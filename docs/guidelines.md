# Guidelines

What every genre must provide and guarantee, so that the command-line tool,
the web app, the Python package and the benchmarks can treat all genres the
same way. How puzzles are rated is in [difficulty.md](difficulty.md), which is
part of these guidelines.

Numbered items (G5, T3, ...) are requirements; code and tests cite them by
number. SHOULD and MAY are used as in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119); everything else is
required. To add a genre, see [Adding a genre](#adding-a-genre).

## Terms

- **Shape:** what is needed to read a puzzle's clues, such as its size.
  **Parameters:** the shape plus generator settings.
- **Answer:** a player's complete or partial marks. **Solution:** a complete
  answer satisfying every rule.
- **Variant:** a genre with a rule added, removed or changed (D4).
- **Rule instance, technique, bookkeeping:** see
  [difficulty.md](difficulty.md#terms).
- **Limits:** caps on generator attempts. **Work budget:** a cap on
  computation, counted in operations; the default is 100,000,000.
- **Canonical form:** the one form a genre writes, among those it accepts.

## What a genre provides

- **P1. Metadata.** Id, display name and aliases ([Naming](#naming)), a
  one-sentence summary of the rules, generator and rater versions, and the
  number and short name of each rule (R1 "row totals", ...).
- **P2. Shape and parameters.** Types that validate against the size limits,
  have defaults and have stable text encodings, used in
  [puzzle IDs](#puzzle-ids), the `params` key (F6) and the command-line tool.
- **P3. Puzzles.** A type that can only hold valid puzzles, with the
  [text format](#text-format), [ID](#puzzle-ids) and, behind the `serde`
  feature, [JSON](#json).
- **P4. Answers.** A type for complete and partial answers, with text and JSON.
  The genre defines the value an unmarked variable takes when the player says
  they have finished (usually unshaded). Pencil marks, colors and undo
  history stay in the web app.
- **P5. Solve.** Returns no solution; exactly one, with it; or more than one,
  with two different solutions. Accepts a work budget (G8).
- **P6. Rate.** Returns a rating ([difficulty.md](difficulty.md#rating-results)),
  or an error if the puzzle does not have exactly one solution or the
  techniques cannot finish it. Accepts a work budget.
- **P7. Generate.** Takes parameters, an optional difficulty, a seed, limits
  and a work budget. Returns the puzzle, solution, rating, seed, generator
  version and number of candidates tried, or an error saying why it stopped.
- **P8. Check.** Takes a puzzle and an answer and returns every violation:
  rule number, location and any numbers involved. Violations contain data
  only, so tools can build and translate the messages. A finished answer is
  checked as complete after unmarked variables take their default (P4). For a
  partial answer, the checker does not report an instance that some completion
  would satisfy, and reports at least what the rater's "broken" check
  recognizes ([L23](#checking-a-genres-rater)).
- **P9. Steps and hints.** The rater lists its steps and, given a partial
  answer that agrees with the solution, returns the next skill step after
  bookkeeping ([L19](difficulty.md#rating-results)). Hints come from the
  techniques alone and do not use the solution.
- **P10. View.** A function from a puzzle and an answer to a view (I4).

## Guarantees

- **G1. Correct solver.** Every returned solution satisfies every rule. The
  solver reports exactly one solution only after proving no other exists, and
  two reported solutions are valid and different.
- **G2. Generated puzzles have exactly one solution,** proven by the solver.
- **G3. Generated ratings match.** The rating a generator returns equals what
  the rater returns for the same puzzle.
- **G4. Rated levels only.** Every generated puzzle is rated easy to expert
  ([L21](difficulty.md#generating-by-difficulty)).
- **G5. Reproducible generation.** Output depends only on the generator
  version, parameters, difficulty and seed. When generation succeeds, the
  puzzle is the same under any limits and budgets. Random numbers
  come from `pencil_puzzles_core::SplitMix64` seeded with the given seed.
  Output does not depend on `HashMap`/`HashSet` order, unstable sorts, memory
  addresses, time, threads, the platform or floating point, and uses
  fixed-width integers where the width could matter (`usize` is 32 bits in
  WebAssembly).
- **G6. Reproducible ratings.** A rating depends only on the puzzle and the
  rater version ([L15-L16](difficulty.md#rating-procedure)).
- **G7. No panics.** Parsing, validation, solving, rating, checking, hinting
  and generation do not panic on any input; integer overflow counts as a
  panic. Recursion depth is at most proportional to the number of variables,
  and operations at the size limit are tested on `wasm32-unknown-unknown`,
  whose stack is small.
- **G8. Bounded work.** Parsers check sizes against the limits before
  allocating. Memory use is polynomial in the grid size, with the exponent on
  the genre page. Solve, rate, hint and generate accept a work budget and
  return a "budget exceeded" error when it runs out. Generators stop after
  their attempt limit.
- **G9. Untrusted input.** Puzzle text, IDs and JSON may come from links, so
  G8 applies to every parser.
- **G10. Portable and self-contained.** Genre crates do not do I/O, read the
  clock, spawn threads, use global mutable state or use `unsafe`, and they
  build for `wasm32-unknown-unknown`. Their only dependencies are
  `pencil-puzzles-core` and, behind the `serde` feature, `serde`.

## Formats

All genres share these, so tools can read any genre's files.

### Text format

- **F1. Encoding.** UTF-8. Lines end in `\n`; a `\r` before it is accepted.
  Spaces at the start and end of a line are ignored, except inside grid
  blocks (F4).
- **F2. Puzzles and files.** A puzzle starts with `genre: <id>`. Writers
  always write it; readers MAY accept a puzzle without it when the genre is
  known. A file MAY contain several puzzles.
- **F3. Lines.** Outside grid blocks, a line is blank, a `#` comment, or
  `key: value` with a key matching `[a-z][a-z0-9_]*`. Each key appears at most
  once per puzzle unless the genre page says otherwise.
- **F4. Grid blocks.** A line `key:` with nothing after the colon starts a
  grid block: each following line is a row, up to the next blank line, line
  containing `:`, or end of file. One character per cell or, when any cell
  needs more, cells separated by single spaces.
- **F5. Cell characters.** `.` no clue or not marked; `#` shaded; `-` marked
  unshaded; `0`-`9` a number; `?` a clue without a number; `a`-`z`, `A`-`Z` a
  region label.
- **F6. Shared keys.** These mean the same in every genre: `title`, `author`,
  `source`, `license` (C3), `solution` (a grid block), `rating` (level and
  score, such as `hard 42`), `rater_version`, `generator_version`, `params`,
  `difficulty`, `seed`. Readers accept them, check their format and keep them
  for writing back (F8), without solving or rating while parsing.
- **F7. Strict parsing.** Parsing accepts the documented format and rejects
  anything else with an error naming the line. A genre MAY accept documented
  alternatives but always writes its canonical form.
- **F8. Round trips.** Write then read gives the same puzzle. Read canonical
  text then write gives the same text, apart from comments.

### Puzzle IDs

A puzzle ID is a one-line form of a puzzle for links and sharing, such as
`kakurasu:5x5:10.11.7.5.4/4.3.4.14.3`.

- **F9. Grammar.** `<genre id>:<shape>:<description>`, using only `A`-`Z`,
  `a`-`z`, `0`-`9`, `-`, `.`, `_`, `~`, `,` and `/`, so it fits in a URL query
  unescaped. At most 4096 characters, checked first.
- **F10. Content.** The shape holds what is needed to read the description.
  The ID does not contain the seed, difficulty or generator settings.
- **F11. Round trips.** IDs round-trip like text (F8) and SHOULD be compact.
  Errors give the position of the first wrong character.
- **F12. Compatibility.** An incompatible new description format has a marker,
  such as a new shape prefix, that tells it apart from older ones, so old IDs
  keep working (V2). Tools put IDs in a URL's query or fragment, since
  `kakurasu:` would read as a scheme.

### JSON

- **F13. Genre JSON.** Behind the `serde` feature, puzzles, answers and
  ratings serialize with `snake_case` fields. Puzzles and answers reject
  unknown fields; ratings and metadata ignore them. Seeds are strings, since
  JavaScript cannot hold every 64-bit integer.
- **F14. Envelopes.** Multi-genre tools wrap genre data as
  `{"genre": "<id>", "puzzle": ..., "solution": ..., "rating": ..., "seed": "..."}`
  with only the fields that apply, and reject unknown fields.

## Rules and documentation

- **D1. Genre page.** Each genre has `docs/genres/<id>.md`, following
  [the template](genres/TEMPLATE.md) ([L25](#what-a-genre-page-covers)).
  Rule numbers (R1, R2, ...) are used by the checker and do not change once
  the genre is released.
- **D2. Original text.** Rules and examples are written for this project.
  Puzzles from elsewhere follow C3.
- **D3. Links.** The crate's documentation links to its page, the README
  lists the genre, and the page's Performance section matches the recorded
  benchmark results (B2).
- **D4. Variants.** A variant that only changes a number in a rule is a shape
  parameter. One that adds, removes or changes a rule is a separate genre.
- **D5. Coordinates.** Rows and columns count from 0 in code and from 1 for
  people. These guidelines assume rectangular grids.

## Tests

Each genre has T1-T9 and follows T10 and T11.

- **T1. Solver against brute force.** On shapes small enough for brute force,
  including non-square ones, the solver matches a brute-force count.
- **T2. Rater.** Every check in [L23](#checking-a-genres-rater).
- **T3. Generator.** For several sizes, including non-square, and every
  level: exactly one solution (G2), the rating matches the rater (G3), the
  same seed gives the same puzzle (G5).
- **T4. Golden puzzles.** A file of parameters, difficulties, seeds, the IDs
  they generate and the generator version. Changed output with an unchanged
  version fails.
- **T5. Formats.** Round trips (F8, F11), every parse error and its position,
  and at least 10,000 mutated texts, IDs and JSON inputs, none of which may
  panic (G7).
- **T6. Checker.** For every rule, a complete answer breaking only that rule
  is reported with its number. Every solution passes. On small grids, brute
  force over completions confirms that partial answers get no false alarms
  (P8).
- **T7. Corpus.** Every corpus puzzle solves to its recorded solution and gets
  its recorded rating.
- **T8. Platforms.** The crate builds for `wasm32-unknown-unknown`, and solve,
  rate and generate at the size limit run there in CI under Node (G7).
- **T9. Work budgets.** With a small budget, solve, rate and generate give
  their normal result or report that the budget ran out (G8).
- **T10. Independent checks.** Brute-force and reference code does not share
  the propagation or search of the code it checks.
- **T11. Test time.** A genre's default `cargo test` SHOULD take under 60
  seconds.

### Checking a genre's rater

**L23.** Every genre tests that:

1. every technique keeps every solution of random partial states, compared
   with a brute-force enumeration;
2. its "broken" check does not fire while a solution remains, and is exact
   on complete grids;
3. ratings, step traces and generated puzzles are deterministic and match
   the corpus and golden files;
4. work budgets bound every operation.

### What a genre page covers

**L25.** Each genre page lists its techniques in profile order, each with a
level and an example, defines its bookkeeping and describes its "broken"
check.

## Corpus

- **C1. Files.** Each genre has `corpus/<id>/` in the text format, with a
  `solution:` block, `rating:`, `rater_version`, `source:` and `license:` for
  each puzzle (F6).
- **C2. Coverage.** At least one puzzle at each level the genre generates at
  a practical size (B1), plus edge cases such as the smallest size and
  non-square grids.
- **C3. Sources.** Each corpus puzzle comes from the generator (recorded with
  its generator version, parameters, difficulty and seed), from a contributor
  under the project's license, or from a publication whose license allows
  including it.
- **C4. Recorded ratings** change only with a rater version bump (V3).

## Benchmarks

- **B1. Practical sizes.** A size is practical at a level if generation takes
  at most 1 second at the median and 5 seconds at the 95th percentile on the
  benchmark's seeds, counting failures as infinite time.
- **B2. Benchmark.** Every genre is measured with
  `cargo run --release -p pencil-puzzles --example benchmark -- <genre> [seeds]`.
  Results go in `docs/benchmarks/<genre>.csv`, and the largest practical size
  for each level goes on the genre page (D3). All genres are measured in one
  run on one machine, described in `docs/benchmarks/README.md`; times are
  comparable within a run.
- **B3. Operation counts.** Benchmarks record deterministic operation counts,
  which catch regressions independently of the machine.
- **B4. WebAssembly.** Results SHOULD include how much slower the genre runs
  in a browser, measured with `node web/benchmark.mjs <genre> <shape> [seeds]`.

## Code conventions

- **K1. Names.** Each genre's typed API uses `Shape`, `Params`, `Puzzle`,
  `Answer` (also for solutions), `Solutions`, `Rating`, `GenerateOptions`,
  `Generated`, `Violation`, and the functions `solve`, `rate`, `generate`,
  `check`, `hint`.
- **K2. Types.** Public enums and structs that may grow are
  `#[non_exhaustive]`. Errors implement `std::error::Error`.
- **K3. Lints.** `#![forbid(unsafe_code)]`, `#![warn(missing_docs)]`,
  `cargo fmt` and `cargo clippy -- -D warnings`.
- **K4. Umbrella crate.** Each genre crate is added to `pencil-puzzles` behind
  a feature named after the genre id, and to `all-genres`.
- **K5. Minimum Rust version.** The workspace declares `rust-version` and CI
  tests it.

## Versions and compatibility

Releases contain the Python package on PyPI, the Rust crates on crates.io,
the command-line tool and the web app.

- **V1. Semantic versioning.** Released packages follow
  [semantic versioning](https://semver.org). Breaking changes are those that
  stop existing puzzle files, IDs or JSON from being read, or that change the
  public Python API.
- **V2. Old files keep working.** Text, IDs and JSON written by any released
  version stay readable within the same major version.
- **V3. Generator and rater versions.** The generator version increases
  whenever the same parameters, difficulty and seed could give a different
  puzzle; the rater version whenever a rating could change. Both are noted in
  `CHANGELOG.md` as **(new puzzles)** or **(new ratings)**. The code for older
  versions is deleted.
- **V4. Patch releases** do not change any generator or rater version.
- **V5. Sharing puzzles.** Seeds reproduce a puzzle only within one generator
  version, so docs and tools share puzzles by ID.

## Naming

- **N1. Ids.** The most widely used name, in lowercase ASCII with `-` between
  words: `kakurasu`, `star-battle`. Ids do not change once a genre is released.
- **N2. Aliases.** Listed in the metadata and on the genre page, and accepted
  wherever an id is. An alias belongs to one genre only.
- **N3. Crates.** `pencil-puzzles-<id>` in `crates/genres/<id>/`; its feature
  in `pencil-puzzles` and its CLI subcommand are the id.

## Shared infrastructure

See [the framework design](design/framework.md).

- **I1. Genre trait, shared loops and registry.** `pencil-puzzles-core`
  defines the `Genre` trait (P1-P10), the solve loop
  (`pencil_puzzles_core::human`) and the generator loop
  (`pencil_puzzles_core::search`); `pencil-puzzles` has a registry of every
  genre by id or alias. Every genre implements the trait and rates and
  generates through the shared loops.
- **I2. Generic tools.** The command-line tool, web app and Python package
  work through the registry, with no genre-specific code beyond what the
  trait provides.
- **I3. Shared tests.** Tests that are the same for every genre (T1, T3-T7,
  T9) SHOULD be written once and called with each genre's inputs.
- **I4. View.** `pencil-puzzles-core` defines a view: a board as layers of
  cells, links between cells, edges, vertices and outside clues, which the
  web app draws without genre-specific code. Clients send marks back as the
  cell values followed by the link values.
- **I5. Shared helpers.** Helpers more than one genre needs SHOULD live in
  `pencil-puzzles-core`.

## Adding a genre

1. Open an issue proposing the genre, with its rules and origin.
2. Create `crates/genres/<id>/` (N3) and add it to `pencil-puzzles` (K4).
3. Copy [the template](genres/TEMPLATE.md) to `docs/genres/<id>.md` and write
   the rules, variables, rule instances and techniques first (D1).
4. Implement the shape, parameters, puzzle and answer types and their formats
   (P2-P4, F1-F14).
5. Implement the solver and check it against brute force (P5, T1, T10).
6. Implement the checker (P8, T6).
7. Implement the techniques on the shared solve loop (P6, P9, T2).
8. Implement the builder and edit moves on the shared generator loop
   (P7, T3, T4).
9. Add the corpus (C1-C4, T7) and record the benchmark (B1-B4).
10. Register the genre (I1) and add a line to `CHANGELOG.md`.

## Changing these guidelines

A change that makes a requirement stricter says how existing genres will meet
it. Requirement numbers are not reused.
