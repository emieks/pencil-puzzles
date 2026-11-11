use pencil_puzzles_core::{Position, SplitMix64};
use pencil_puzzles_sudoku::*;
#[test]
fn canonical_records_ids_and_answers_round_trip() {
    let text = "genre: sudoku\nsource: project\nlicense: MIT\ngrid:\n1..4\n.4..\n..4.\n4..1\nparams: 4x4\ndifficulty: easy\nseed: 18446744073709551615\nrating: easy 1\nsolution:\n1234\n3412\n2143\n4321";
    let record = parse_records(text).unwrap().remove(0);
    assert_eq!(write_record(&record), text);
    assert_eq!(Puzzle::from_id(&record.puzzle.id()).unwrap(), record.puzzle);
    let answer: Answer = ".200\n1234\n4321\n....".parse().unwrap();
    assert_eq!(answer.to_string().parse::<Answer>().unwrap(), answer);
    assert!(answer.finished().is_complete());
    let zero: Puzzle = "grid:\n0000\n0000\n0000\n0000".parse().unwrap();
    assert!(zero.to_string().contains("...."));
}
#[test]
fn errors_report_the_source_position() {
    for (source, line) in [
        ("grid:\n12..\n..34\n....\n...x", 5),
        ("genre: sudoku\nwrong: x\ngrid:\n.", 2),
        ("grid:\n.\nseed: -1", 3),
        ("grid:\n.\nsolution:\n.", 3),
        ("grid:\n.\nparams: 5x5", 3),
        ("grid:\n12\n12", 2),
    ] {
        assert_eq!(
            source.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{source}"
        );
    }
    assert_eq!(
        Puzzle::from_id("sudoku:4x4:................x")
            .unwrap_err()
            .position(),
        Position::Char(28)
    );
    assert_eq!(
        Puzzle::from_id("sudoku:4x4:...............x")
            .unwrap_err()
            .position(),
        Position::Char(27)
    );
    assert_eq!(
        Puzzle::from_id("sudoku:5x5:.").unwrap_err().position(),
        Position::Char(8)
    );
    assert!("04x4".parse::<Shape>().is_err());
    assert!(Shape::new(usize::MAX).is_err());
    assert_eq!(
        "\n.\n\nx".parse::<Answer>().unwrap_err().position(),
        Position::Line(2)
    );
}
#[test]
fn ten_thousand_mutated_inputs_do_not_panic() {
    let inputs = [
        "genre: sudoku\ngrid:\n1..4\n.4..\n..4.\n4..1",
        "sudoku:4x4:1..4.4....4.4..1",
        r#"{"grid":["1..4",".4..","..4.","4..1"]}"#,
    ];
    let mut rng = SplitMix64::new(67);
    for i in 0..10_000 {
        let mut bytes = inputs[i % 3].as_bytes().to_vec();
        for _ in 0..=rng.below(5) {
            let at = rng.below(bytes.len() as u64 + 1) as usize;
            match rng.below(3) {
                0 if at < bytes.len() => {
                    bytes.remove(at);
                }
                1 if at < bytes.len() => bytes[at] = rng.below(256) as u8,
                _ => bytes.insert(at, rng.below(256) as u8),
            }
        }
        let text = String::from_utf8_lossy(&bytes);
        let _ = text.parse::<Puzzle>();
        let _ = text.parse::<Answer>();
        let _ = Puzzle::from_id(&text);
        #[cfg(feature = "serde")]
        {
            let _ = serde_json::from_str::<Puzzle>(&text);
            let _ = serde_json::from_str::<Answer>(&text);
        }
    }
}
#[cfg(feature = "serde")]
#[test]
fn json_is_strict_and_bounded() {
    let p: Puzzle = "grid:\n1..4\n.4..\n..4.\n4..1".parse().unwrap();
    let mut json = serde_json::to_value(&p).unwrap();
    assert_eq!(serde_json::from_value::<Puzzle>(json.clone()).unwrap(), p);
    json["extra"] = true.into();
    assert!(serde_json::from_value::<Puzzle>(json).is_err());
    for rows in [vec!["."; 10], vec![".........."; 9]] {
        let json = serde_json::json!({ "grid": rows });
        assert!(serde_json::from_value::<Puzzle>(json).is_err());
    }
}

#[test]
fn grid_and_solution_blocks_are_bounded_while_scanning() {
    for header in ["grid:", "genre: sudoku\ngrid:\n.\nsolution:"] {
        let prefix = header.lines().count();
        let source = format!("{header}\n{}", ".........\n".repeat(100_000));
        assert_eq!(
            source.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(prefix + 10)
        );
    }
    for source in [
        format!("grid:\n{}", ".".repeat(10_000)),
        format!("grid:\n.\nsolution:\n{}", ".".repeat(10_000)),
    ] {
        assert!(source.parse::<Puzzle>().is_err());
    }
    for id in ["sudoku:4x4:x", "sudoku:4x4:5................."] {
        assert_eq!(
            Puzzle::from_id(id).unwrap_err().position(),
            Position::Char(12)
        );
    }
}
#[test]
fn mismatched_view_answers_are_ignored() {
    use pencil_puzzles_core::Genre;
    let p = Puzzle::new(Shape::default(), vec![0; 81]).unwrap();
    let a = Answer::new(Shape::new(1).unwrap());
    let view = Sudoku::view(&p, Some(&a));
    assert_eq!(view.cells.len(), 81);
    assert!(view.cells.iter().all(|c| c.value == "."));
}

#[test]
fn separate_largest_grid_and_solution_survive_bounded_scanning() {
    let mut source = String::from("genre: sudoku\ngrid:\n");
    source.push_str(&".........\n".repeat(9));
    source.push_str("solution:\n");
    source.push_str("123456789\n456789123\n789123456\n234567891\n567891234\n891234567\n345678912\n678912345\n912345678");
    let r = parse_records(&source).unwrap().remove(0);
    assert_eq!(r.puzzle.shape(), Shape::default());
    assert!(r.solution.unwrap().is_complete());
}
