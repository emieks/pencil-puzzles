<!--
Template for a genre page. Copy it to docs/genres/<id>.md, replace every part
in angle brackets, and delete these comments. The codes (L25, D2, ...) refer
to docs/difficulty.md (L) and docs/guidelines.md (the rest).

Write Rules, Variables, Rule instances and Techniques before any code (L25).
-->

# <Display name>

<One sentence describing the goal of the puzzle.>

## Rules

<!-- Numbered R1, R2, ..., written for this project (D2), precise enough to
settle edge cases. The checker reports violations by these numbers, so
they do not change once the genre is released. -->

- **R1.** <Rule.>
- **R2.** <Rule.>

## Example

<!-- A small generated puzzle, with its ID, solution and a sentence on how
the rules apply. -->

Generator version <n>, shape `<shape>`, <level>, seed `<seed>` produces
`<id>:<shape>:<description>`:

```text
<Puzzle and solution, drawn.>
```

## Variables

<What the variables are, their values, and their order.>

## Rule instances

<!-- When an instance counts as broken. It only reports a break that no
completion avoids, and on complete grids it reports exactly the broken rules
(L23). -->

| Instance | Rule | Broken when |
| --- | --- | --- |
| <Each row> | R1 | <its shaded cells exceed the clue> |

## Techniques

<!-- The technique profile in solving order: name, level (or bookkeeping)
and what it does, with a small example where it helps. -->

| Technique | Level | What it does |
| --- | --- | --- |
| <`line_complete`> | bookkeeping | <A line that reached its total has its other cells unshaded.> |
| <`too_heavy`> | Easy | <A cell worth more than the line still needs is unshaded.> |

## Parameters

| Parameter | Part of the shape? | Limits | Default |
| --- | --- | --- | --- |
| <rows> | yes | <1-20> | <6> |

<How the generator builds and edits a puzzle.>

## Formats

<Puzzle text, ID and JSON, each with an example; answer format and the value
unmarked cells take when the player finishes (P4); violation fields.>

## Performance

<!-- From `cargo run --release -p pencil-puzzles --example benchmark -- <id> 20`,
recorded in docs/benchmarks/<id>.csv (B2, D3). -->

<The benchmark table, the largest practical size per level (B1), the
slowest corpus rating, and how memory grows with the grid size (G8).>

## References

<Other names and where the genre comes from.>
