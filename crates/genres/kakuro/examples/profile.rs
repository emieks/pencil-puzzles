//! Time generation and rating for one size and level:
//! `cargo run --release -p pencil-puzzles-kakuro --example profile -- 8x8 hard 10`.
use pencil_puzzles_kakuro::*;
use std::{collections::BTreeMap, time::Instant};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shape = args.get(1).map_or("8x8", |s| s.as_str());
    let level: Option<Difficulty> = args
        .get(2)
        .filter(|s| s.as_str() != "any")
        .map(|s| s.parse().unwrap());
    let seeds: u64 = args.get(3).map_or(5, |s| s.parse().unwrap());
    for seed in 0..seeds {
        let mut o = GenerateOptions::new(shape.parse().unwrap(), seed);
        o.difficulty = level;
        let b = Budget::new(100_000_000);
        let t = Instant::now();
        match generate(&o, &b) {
            Ok(g) => {
                let rb = Budget::unlimited();
                let (r, steps) = rate_with_steps(&g.puzzle, &rb).unwrap();
                let sb = Budget::unlimited();
                solve(&g.puzzle, &sb).unwrap();
                let sh = g.puzzle.shape();
                let whites = g
                    .puzzle
                    .cells()
                    .iter()
                    .filter(|&&c| c == Cell::White)
                    .count();
                let inner = (sh.rows() - 1) * (sh.cols() - 1);
                print!("white {}% ", whites * 100 / inner);
                let mut count = BTreeMap::new();
                for s in &steps {
                    *count.entry(s.technique.clone()).or_insert(0) += 1;
                }
                println!(
                    "seed {seed}: {:?} score {} in {} ms, {} attempts, {} ops; {count:?}",
                    r.level,
                    r.score,
                    t.elapsed().as_millis(),
                    g.attempts,
                    b.spent()
                );
            }
            Err(e) => println!("seed {seed}: {e} after {} ms", t.elapsed().as_millis()),
        }
    }
}
