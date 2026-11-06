use pencil_puzzles_core::{Position, SplitMix64};
use pencil_puzzles_fillomino::*;

#[test]
fn all_formats_round_trip_with_two_digit_numbers_and_metadata() {
    for text in [
        "genre: fillomino\ngrid:\n2.\n..\n.1",
        "genre: fillomino\nsource: project\nlicense: MIT\ngrid:\n12 . . .\n. . . .\n. . . .\nsolution:\n12 12 12 12\n12 12 12 12\n12 12 12 12",
    ] {
        let r = parse_records(text).unwrap().remove(0);
        assert_eq!(write_record(&r), text);
        assert_eq!(Puzzle::from_id(&r.puzzle.id()).unwrap(), r.puzzle);
        #[cfg(feature = "serde")]
        assert_eq!(
            serde_json::from_str::<Puzzle>(&serde_json::to_string(&r.puzzle).unwrap()).unwrap(),
            r.puzzle
        );
    }
    let p: Puzzle = "grid:\n2.\n..\n.1".parse().unwrap();
    assert_eq!(p.id(), "fillomino:3x2:2d1");
    let a: Answer = "2.\n0.\n31".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    assert_eq!(a.get(1, 0), Mark::Empty);
    assert!(!a.finished().is_complete());
    let spaced: Answer = "10 10 10 10 10\n. 0 . . .".parse().unwrap();
    assert_eq!(spaced.to_string(), "10 10 10 10 10\n. 0 . . .");
    #[cfg(feature = "serde")]
    assert_eq!(
        serde_json::from_str::<Answer>(&serde_json::to_string(&a).unwrap()).unwrap(),
        a
    );
}

#[test]
fn malformed_inputs_have_positions_and_respect_limits() {
    for (text, line) in [
        ("grid:\n2.\n.x", 3),
        ("grid:\n0", 2),
        ("grid:\n2", 2),
        ("grid:\n.\nunknown: 1", 3),
        ("grid:\n.\nsolution:\n.", 3),
        ("grid:\n1 2\n3", 3),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text:?}"
        );
    }
    for (id, at) in [
        ("fillomino:3x2:2d1x", 18),
        ("fillomino:3x2:2d", 17),
        ("fillomino:3x2:2f1", 16),
        ("fillomino:3x2:-1", 15),
        ("fillomino:3x2:-12", 15),
        ("fillomino:3y2:2d1", 12),
        ("fillomino:3x22:a", 13),
    ] {
        assert_eq!(
            Puzzle::from_id(id).unwrap_err().position(),
            Position::Char(at),
            "{id}"
        );
    }
    assert!(Shape::new(usize::MAX, 1).is_err());
    assert!(Puzzle::from_id(&"x".repeat(4097)).is_err());
    for rows in [
        vec![".........."; 11],
        vec!["..........."; 10],
        vec![". "; 10],
    ] {
        assert!(Puzzle::from_rows(&rows).is_err());
    }
    assert!("1000 1".parse::<Answer>().is_err());
    assert!("1 01".parse::<Answer>().is_err());
    assert!("5".parse::<Answer>().is_err());
    #[cfg(feature = "serde")]
    for json in [
        r#"{"grid":["1"],"extra":true}"#.to_string(),
        serde_json::json!({"grid": vec!["."; 11]}).to_string(),
        serde_json::json!({"grid": vec!["x".repeat(1000); 10]}).to_string(),
    ] {
        assert!(serde_json::from_str::<Puzzle>(&json).is_err());
    }
}

#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: fillomino\ngrid:\n2.\n..\n.1\nsolution:\n22\n33\n31",
        "fillomino:3x3:2b1c-12a3",
        r#"{"grid":["2.","..",".1"]}"#,
    ];
    let mut rng = SplitMix64::new(67);
    for i in 0..10000 {
        let mut b = inputs[i % 3].as_bytes().to_vec();
        for _ in 0..=rng.below(4) {
            let at = rng.below((b.len() + 1) as u64) as usize;
            if at < b.len() {
                b[at] = rng.below(256) as u8;
            } else {
                b.push(rng.below(256) as u8);
            }
        }
        let s = String::from_utf8_lossy(&b);
        if let Ok(p) = s.parse::<Puzzle>() {
            let _ = rate(&p, &Budget::new(10_000));
        }
        let _ = s.parse::<Answer>();
        if let Ok(p) = Puzzle::from_id(&s) {
            let _ = solve(&p, &Budget::new(10_000));
        }
        #[cfg(feature = "serde")]
        {
            let _ = serde_json::from_str::<Puzzle>(&s);
            let _ = serde_json::from_str::<Answer>(&s);
        }
    }
}
