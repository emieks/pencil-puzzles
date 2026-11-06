//! Time generation for a shape at each level over a few seeds:
//! `cargo run --release -p pencil-puzzles-hitori --example profile -- 8x8 5`.
use pencil_puzzles_hitori::*;
use std::time::Instant;
fn main() {
    let shape = std::env::args().nth(1).unwrap_or("8x8".into());
    let seeds: u64 = std::env::args().nth(2).map_or(5, |s| s.parse().unwrap());
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
        let mut ops = 0;
        for seed in 0..seeds {
            let mut o = GenerateOptions::new(shape.parse().unwrap(), seed);
            o.difficulty = level;
            let b = Budget::new(100_000_000);
            let t = Instant::now();
            let r = generate(&o, &b);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            ops += b.spent();
            if let Ok(g) = r {
                found += 1;
                attempts += g.attempts;
            }
        }
        times.sort_by(f64::total_cmp);
        println!(
            "{shape} {:>6}: {found}/{seeds} found, median {:.0} ms, max {:.0} ms, {} attempts, {} Mops",
            level.map_or("any", |l| l.as_str()),
            times[times.len() / 2],
            times[times.len() - 1],
            attempts / found.max(1),
            ops / seeds / 1_000_000
        );
    }
}
