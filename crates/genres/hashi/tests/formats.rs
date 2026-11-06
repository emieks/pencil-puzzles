use pencil_puzzles_core::{Genre, Position, SplitMix64};
use pencil_puzzles_hashi::*;

#[test]
fn all_formats_round_trip_with_metadata_and_crossings() {
    for text in [
        "genre: hashi\ngrid:\n2.2\n...\n1.1",
        "genre: hashi\nsource: project\nlicense: MIT\ngrid:\n2.2\n...\n1.1\nsolution:\n2-2\n|.|\n1.1",
        "genre: hashi\ngrid:\n.",
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
    let p: Puzzle = "grid:\n2.2\n...\n1.1".parse().unwrap();
    assert_eq!(p.id(), "hashi:3x3:2a2c1a1");
    let a: Answer = "2=2\nH.|\n1.1".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    assert_eq!(a.get(1, 0), Cell::Bridges { across: 0, down: 2 });
    let crossing: Answer = ". 1 .\n1 =H 1\n. 1 .".parse().unwrap();
    assert_eq!(crossing.to_string(), ". 1 .\n1 =H 1\n. 1 .");
    assert_eq!(crossing.get(1, 1), Cell::Bridges { across: 2, down: 2 });
    // Island cells may be left out of an answer; they take their number.
    assert_eq!(
        Hashi::parse_answer(&p, ".-.\n|.|\n...")
            .unwrap()
            .to_string(),
        "2-2\n|.|\n1.1"
    );
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
        ("grid:\n9", 2),
        ("grid:\n.\nunknown: 1", 3),
        ("grid:\n1.1\nsolution:\n1-2", 4),
        ("grid:\n1.\n.", 3),
        ("grid:\n11", 2),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text:?}"
        );
    }
    for (id, at) in [
        ("hashi:3x3:2a2c1a1x", 18),
        ("hashi:3x3:2a2c1a", 17),
        ("hashi:3x3:2a2f1", 15),
        ("hashi:3x3:9", 11),
        ("hashi:3y3:2a2c1a1", 8),
        ("hashi:3x33:a", 9),
        ("hashi:3x3:22g", 11),
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
        vec!["."; 16],
        vec![".".repeat(16).leak() as &str; 15],
        vec![". "; 3],
    ] {
        assert!(Puzzle::from_rows(&rows).is_err());
    }
    for bad in ["1 01", "0", "x", "- -|-", ". .\n."] {
        assert!(bad.parse::<Answer>().is_err(), "{bad:?}");
    }
    #[cfg(feature = "serde")]
    for json in [
        r#"{"grid":["1"],"extra":true}"#.to_string(),
        serde_json::json!({"grid": vec!["."; 16]}).to_string(),
        serde_json::json!({"grid": vec!["x".repeat(1000); 10]}).to_string(),
    ] {
        assert!(serde_json::from_str::<Puzzle>(&json).is_err());
    }
}

#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: hashi\ngrid:\n2.2\n...\n1.1\nsolution:\n2-2\n|.|\n1.1",
        "hashi:4x4:2a2c1b3e3a1",
        r#"{"grid":["2.2","...","1.1"]}"#,
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
            let _ = Hashi::parse_answer(&p, "2-2\n|.|\n1.1");
        }
        if let Ok(a) = s.parse::<Answer>() {
            let p: Puzzle = "grid:\n2.2\n...\n1.1".parse().unwrap();
            let _ = check(&p, &a, &Budget::new(10_000));
            let _ = hint(&p, &a, &Budget::new(10_000));
        }
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
