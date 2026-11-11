use pencil_puzzles_core::{Position, SplitMix64};
use pencil_puzzles_kakurasu::{Answer, Puzzle, parse_records, write_record};

#[test]
fn formats_round_trip_with_shared_keys() {
    let text = "genre: kakurasu\ntitle: A\nsource: project\nlicense: MIT\nrows: 1 2\ncols: 1 2\nparams: 2x2\ndifficulty: easy\nseed: 18446744073709551615\nrating: easy 4\nsolution:\n#-\n-#";
    let record = parse_records(text).unwrap().remove(0);
    assert_eq!(write_record(&record), text);
    assert_eq!(Puzzle::from_id(&record.puzzle.id()).unwrap(), record.puzzle);
    let answer: Answer = "#.\n-#".parse().unwrap();
    assert_eq!(answer.to_string().parse::<Answer>().unwrap(), answer);
    assert!(answer.finished().is_complete());
    assert_eq!(
        parse_records(&format!("{text}\n\n{text}")).unwrap().len(),
        2
    );
}

#[test]
fn errors_have_positions() {
    for (text, line) in [
        ("rows: 1\nsizes: 1\ncols: 1", 2),
        ("rows: 1 x\ncols: 1", 1),
        ("rows: 1\nrows: 2", 2),
        ("1 2 3", 1),
        ("rows: 1", 1),
        ("ROWS: 1\ncols: 1", 1),
        ("rows: 1, 2\ncols: 1", 1),
        ("rows: 1\ncols: 1\nseed: -1", 3),
        ("rows: 1\ncols: 1\nrating: hard", 3),
        ("rows: 1\ncols: 1\nsolution:\n.", 3),
        ("rows: 1\ncols: 1\nsolution:\n #", 4),
        ("genre: kakurasu\nrows: 1\ncols: 2", 3),
        ("genre: kakurasu\nrows: 2\ncols: 1", 2),
        ("rows: 1\ncols: 1\nparams: 1x0", 3),
        ("rows: 1\ncols: 1\nsolution:\nx", 4),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text}"
        );
    }
    for id in [
        "",
        "kakurasu:0x1:0/0",
        "kakurasu:1x1:x/0",
        "kakurasu:1x1:0",
        "kakurasu:1x1:2/0",
        "other:1x1:0/0",
    ] {
        assert!(
            matches!(
                Puzzle::from_id(id).unwrap_err().position(),
                Position::Char(_)
            ),
            "{id}"
        );
    }
    for text in ["", "##\n#", "#x", "33x1"] {
        assert!(text.parse::<Answer>().is_err());
    }
    assert!(Puzzle::new(vec![0; 33], vec![0]).is_err());
}

// T5: deterministic mutations include Unicode and invalid UTF-8 (lossily decoded).
#[test]
fn ten_thousand_mutations_never_panic() {
    let inputs = [
        "genre: kakurasu\nrows: 1 2\ncols: 1 2\nsolution:\n#-\n-#",
        "kakurasu:2x2:1.2/1.2",
        r#"{"row_clues":[1,2],"col_clues":[1,2]}"#,
    ];
    let mut rng = SplitMix64::new(991);
    for n in 0..10_000 {
        let mut bytes = inputs[n % inputs.len()].as_bytes().to_vec();
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
        let _ = Puzzle::from_id(&text);
        let _ = text.parse::<Answer>();
        #[cfg(feature = "serde")]
        {
            let _ = serde_json::from_str::<Puzzle>(&text);
            let _ = serde_json::from_str::<Answer>(&text);
        }
    }
}

#[test]
fn id_errors_identify_the_wrong_character() {
    for (id, at) in [
        ("kakurasu:2xq:1.2/1.2", 12),
        ("kakurasu:2X2:1.2/1.2", 11),
        ("kakurasu:2x0:1.2/1.2", 12),
        ("Kakurasu:1x1:0/0", 1),
    ] {
        assert_eq!(
            Puzzle::from_id(id).unwrap_err().position(),
            Position::Char(at),
            "{id}"
        );
    }
}

#[test]
fn standalone_answer_errors_keep_physical_line_numbers() {
    let error = "\n#\n\nx".parse::<Answer>().unwrap_err();
    assert_eq!(error.position(), Position::Line(4));
}
