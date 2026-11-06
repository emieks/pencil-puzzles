//! Generate a few puzzles per size and level and print timings:
//! `cargo run --release -p pencil-puzzles-hashi --example profile -- [sizes] [seeds]`.
use pencil_puzzles_hashi::{Budget, Difficulty, GenerateOptions, generate};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let sizes: Vec<&str> = args
        .first()
        .map_or(vec!["5x5", "7x7", "10x10"], |s| s.split(',').collect());
    let seeds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    for size in sizes {
        for level in [
            None,
            Some(Difficulty::Easy),
            Some(Difficulty::Medium),
            Some(Difficulty::Hard),
            Some(Difficulty::Expert),
        ] {
            let mut times = Vec::new();
            let mut found = 0;
            let mut attempts = 0;
            for seed in 0..seeds {
                let mut o = GenerateOptions::new(size.parse().unwrap(), seed);
                o.difficulty = level;
                let t = Instant::now();
                let r = generate(&o, &Budget::new(2_000_000_000));
                times.push(t.elapsed().as_secs_f64());
                if let Ok(g) = r {
                    found += 1;
                    attempts += g.attempts;
                }
            }
            times.sort_by(|a, b| a.total_cmp(b));
            println!(
                "{size:>6} {:>7} found {found}/{seeds} median {:.3}s max {:.3}s mean attempts {}",
                level.map_or("any".into(), |l| format!("{l:?}")),
                times[times.len() / 2],
                times[times.len() - 1],
                attempts / found.max(1) as u32
            );
        }
    }
}
