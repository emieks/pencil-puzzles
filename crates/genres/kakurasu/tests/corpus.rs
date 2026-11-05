use pencil_puzzles_kakurasu::*;
use std::{fs, path::Path};
#[test]
fn corpus_solutions_ratings_and_generation_match() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../corpus/kakurasu");
    let mut levels = [false; 5];
    for entry in fs::read_dir(root).unwrap() {
        let text = fs::read_to_string(entry.unwrap().path()).unwrap();
        for record in parse_records(&text).unwrap() {
            let r = rate(&record.puzzle, &Budget::unlimited()).unwrap();
            assert_eq!(record.shared.rating, Some((r.level, r.score)));
            assert_eq!(record.shared.rater_version, Some(RATER_VERSION));
            assert_eq!(
                solve(&record.puzzle, &Budget::unlimited()).unwrap(),
                Solutions::Unique(record.solution.clone().unwrap())
            );
            assert!(
                check(&record.puzzle, &record.solution.unwrap())
                    .unwrap()
                    .is_empty()
            );
            levels[r.level as usize] = true;
            let mut options = GenerateOptions::new(
                record.shared.params.unwrap().parse().unwrap(),
                record.shared.seed.unwrap(),
            );
            options.difficulty = record.shared.difficulty;
            assert_eq!(
                generate(&options, &Budget::unlimited()).unwrap().puzzle,
                record.puzzle
            );
        }
    }
    // Easy to Expert; Extreme is not generated.
    assert_eq!(levels, [true, true, true, true, false]);
}
#[test]
fn golden_generation_is_stable() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden.txt");
    let text = fs::read_to_string(&path).unwrap();
    let mut output = String::from("# params difficulty seed id generator_version\n");
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let fields: Vec<_> = line.split_whitespace().collect();
        let mut options =
            GenerateOptions::new(fields[0].parse().unwrap(), fields[2].parse().unwrap());
        options.difficulty = Some(fields[1].parse().unwrap());
        let g = generate(&options, &Budget::unlimited()).unwrap();
        output.push_str(&format!(
            "{} {} {} {} {}\n",
            fields[0],
            fields[1],
            g.seed,
            g.puzzle.id(),
            g.generator_version
        ));
    }
    if std::env::var_os("BLESS").is_some() {
        fs::write(path, &output).unwrap();
    } else {
        assert_eq!(text, output);
    }
}
