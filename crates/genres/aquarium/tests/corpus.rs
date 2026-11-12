use pencil_puzzles_aquarium::*;
use std::{fs, path::Path};
#[test]
fn corpus_solutions_ratings_and_reproduction() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../corpus/aquarium");
    let mut levels = [false; 5];
    for entry in fs::read_dir(root).unwrap() {
        let text = fs::read_to_string(entry.unwrap().path()).unwrap();
        for r in parse_records(&text).unwrap() {
            let rating = rate(&r.puzzle, &Budget::unlimited()).unwrap();
            assert_eq!(r.shared.rating, Some((rating.level, rating.score)));
            assert_eq!(
                solve(&r.puzzle, &Budget::unlimited()).unwrap(),
                Solutions::Unique(r.solution.unwrap())
            );
            let mut options = GenerateOptions::new(
                r.shared.params.unwrap().parse().unwrap(),
                r.shared.seed.unwrap(),
            );
            options.difficulty = r.shared.difficulty;
            assert_eq!(
                generate(&options, &Budget::unlimited()).unwrap().puzzle,
                r.puzzle
            );
            levels[rating.level as usize] = true;
        }
    }
    assert!(levels[0] && levels[1] && levels[2] && levels[3]);
}
#[test]
fn golden_generation_is_stable() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden.txt");
    let text = fs::read_to_string(&path).unwrap();
    let mut actual = String::from("# params difficulty seed id\n");
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let f: Vec<_> = line.split_whitespace().collect();
        let mut o = GenerateOptions::new(f[0].parse().unwrap(), f[2].parse().unwrap());
        o.difficulty = Some(f[1].parse().unwrap());
        let g = generate(&o, &Budget::unlimited()).unwrap();
        actual.push_str(&format!("{} {} {} {}\n", f[0], f[1], g.seed, g.puzzle.id()));
    }
    if std::env::var_os("BLESS").is_some() {
        fs::write(path, &actual).unwrap();
    } else {
        assert_eq!(text, actual);
    }
}
