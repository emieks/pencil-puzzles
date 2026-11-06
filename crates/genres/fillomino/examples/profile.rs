use pencil_puzzles_fillomino::*;
use std::{collections::BTreeMap, time::Instant};
fn main() {
    let shape = std::env::args().nth(1).unwrap_or("10x10".into());
    let level: Difficulty = std::env::args()
        .nth(2)
        .unwrap_or("medium".into())
        .parse()
        .unwrap();
    let mut o = GenerateOptions::new(shape.parse().unwrap(), 0);
    o.difficulty = Some(level);
    let g = generate(&o, &Budget::unlimited()).unwrap();
    let t = Instant::now();
    let b = Budget::unlimited();
    let (r, steps) = rate_with_steps(&g.puzzle, &b).unwrap();
    println!(
        "rate {:?} {} ms, {} ops, {} steps",
        r.level,
        t.elapsed().as_millis(),
        b.spent(),
        steps.len()
    );
    let mut count = BTreeMap::new();
    for s in &steps {
        *count.entry(s.technique.clone()).or_insert(0) += 1;
    }
    println!("{count:?}");
    let t = Instant::now();
    for _ in 0..10 {
        solve(&g.puzzle, &Budget::unlimited()).unwrap();
    }
    println!("solve x10 {} ms", t.elapsed().as_millis());
    let t = Instant::now();
    for _ in 0..10 {
        rate(&g.puzzle, &Budget::unlimited()).unwrap();
    }
    println!("rate x10 {} ms", t.elapsed().as_millis());
}
#[allow(dead_code)]
fn unused() {}
