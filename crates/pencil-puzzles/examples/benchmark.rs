//! Generation benchmark for every genre (guideline B1-B3):
//! `cargo run --release -p pencil-puzzles --example benchmark -- <genre> [seeds]`.
//!
//! For each size and requested level (Easy to Expert, and "any") that the
//! genre supports, generates
//! seeds 0 to `seeds - 1` (default 30) with the command-line tool's defaults:
//! 10,000 attempts and 100,000,000 operations. Failures count as infinite
//! time in the percentiles. Then times rating every corpus puzzle.
use pencil_puzzles::{Budget, Difficulty, genres};
use std::{fs, path::Path, time::Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().expect("usage: benchmark <genre> [seeds]");
    let seeds: u64 = args.next().map_or(30, |s| s.parse().expect("seed count"));
    let genre = genres()
        .iter()
        .find(|g| g.metadata().id == id)
        .expect("unknown genre");
    let sizes: &[&str] = match id.as_str() {
        "sudoku" => &["9x9"],
        "nurikabe" => &["5x5", "7x7", "8x8", "10x10"],
        "fillomino" => &["5x5", "6x6", "7x7", "8x8", "10x10"],
        "hashi" => &["5x5", "7x7", "10x10", "12x12", "15x15"],
        "hitori" => &["5x5", "6x6", "8x8", "10x10", "12x12"],
        "aquarium" => &["6x6", "8x8", "10x10", "12x12", "15x15"],
        "kakuro" => &["6x6", "8x8", "10x10", "12x12", "16x16"],
        _ => &["6x6", "8x8", "10x10", "12x12"],
    };
    println!(
        "shape,requested,success,median_ms,p95_ms,mean_attempts,mean_operations,easy,medium,hard,expert"
    );
    for &shape in sizes {
        let params = genre.params_for_shape(shape).expect("valid shape");
        let levels = [None]
            .into_iter()
            .chain(Difficulty::ALL[..4].iter().copied().map(Some));
        for level in levels {
            if !genre.supports(&params, level).expect("valid params") {
                continue;
            }
            let mut times = Vec::new();
            let mut attempts = 0u64;
            let mut operations = 0u64;
            let mut mix = [0u32; 4];
            for seed in 0..seeds {
                let budget = Budget::new(100_000_000);
                let start = Instant::now();
                let result = genre.generate(&params, level, seed, 10_000, &budget);
                let ms = start.elapsed().as_secs_f64() * 1000.;
                operations += budget.spent();
                if let Ok(g) = result {
                    times.push(ms);
                    attempts += u64::from(g.attempts);
                    mix[(g.rating.level as usize).min(3)] += 1;
                }
            }
            times.sort_by(f64::total_cmp);
            let n = seeds as usize;
            // Percentiles over all seeds, failures as infinity.
            let at = |p: usize| times.get(p * n / 100).copied().unwrap_or(f64::INFINITY);
            println!(
                "{shape},{},{},{:.1},{:.1},{:.1},{:.0},{},{},{},{}",
                level.map_or("any", |l| l.as_str()),
                times.len(),
                at(50),
                at(95).max(at(50)),
                attempts as f64 / times.len().max(1) as f64,
                operations as f64 / seeds as f64,
                mix[0],
                mix[1],
                mix[2],
                mix[3]
            );
        }
    }
    println!("corpus,level,rating_ms");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../corpus/{id}"));
    let mut paths: Vec<_> = fs::read_dir(dir)
        .expect("corpus directory")
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        let start = Instant::now();
        let rating = genre.rate(&text, &Budget::unlimited()).unwrap();
        println!(
            "{},{},{:.1}",
            path.file_stem().unwrap().to_string_lossy(),
            rating.level,
            start.elapsed().as_secs_f64() * 1000.
        );
    }
}
