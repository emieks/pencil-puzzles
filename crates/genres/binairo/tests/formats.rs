use pencil_puzzles_binairo::*;
use pencil_puzzles_core::{Position, SplitMix64};

#[test]
fn all_formats_round_trip_with_metadata() {
    let full = (0..16)
        .map(|r| {
            if r % 2 == 0 {
                "0".repeat(16)
            } else {
                "1".repeat(16)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    for text in [
        "genre: binairo\ngrid:\n1.\n..".to_string(),
        "genre: binairo\nsource: project\nlicense: MIT\ngrid:\n1...\n..0.\n....\n.1..\nsolution:\n1010\n0101\n1001\n0110".into(),
        "genre: binairo\ngrid:\n....\n....\n....\n....".into(),
        format!("genre: binairo\ngrid:\n{full}"),
    ] {
        let r = parse_records(&text).unwrap().remove(0);
        assert_eq!(write_record(&r), text);
        assert_eq!(Puzzle::from_id(&r.puzzle.id()).unwrap(), r.puzzle);
        #[cfg(feature = "serde")]
        assert_eq!(
            serde_json::from_str::<Puzzle>(&serde_json::to_string(&r.puzzle).unwrap()).unwrap(),
            r.puzzle
        );
    }
    let p: Puzzle = "grid:\n1...\n..0.\n....\n.1..".parse().unwrap();
    assert_eq!(p.id(), "binairo:4x4:1e0f1b");
    assert_eq!(p.given(3, 1), Mark::One);
    let empty: Puzzle = format!("grid:{}", "\n........".repeat(8)).parse().unwrap();
    assert_eq!(empty.id(), "binairo:8x8:zzl");
    let a: Answer = "10\n.1".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    #[cfg(feature = "serde")]
    {
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"grid":["1...","..0.","....",".1.."]}"#
        );
        assert_eq!(
            serde_json::from_str::<Answer>(&serde_json::to_string(&a).unwrap()).unwrap(),
            a
        );
    }
}

#[test]
fn malformed_inputs_have_positions_and_respect_limits() {
    for (text, line) in [
        ("grid:\n1.\n.2", 3),
        ("grid:\n1.\n.", 3),
        ("grid:\n1..\n...", 2),
        ("grid:\n1.\n..\n..", 2),
        ("grid:\n#.\n..", 2),
        ("rows: 1 2\ngrid:\n1.\n..", 1),
        ("genre: binairo", 1),
        ("grid:\n1.\n..\nsolution:\n1.\n01", 4),
        ("grid:\n1.\n..\nsolution:\n1010\n0101", 5),
        ("grid:\n1.\n..\nparams: 3x3", 4),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text:?}"
        );
    }
    for (id, at) in [
        ("binairo:2x2:1c0", 15),
        ("binairo:2x2:1b", 15),
        ("binairo:2x2:1ab", 15),
        ("binairo:2x2:1d", 14),
        ("binairo:2x2:12aa", 14),
        ("binairo:2x2:1A0b", 14),
        ("binairo:2x3:d", 11),
        ("binairo:4x8:ze", 9),
        ("binairo:18x2:ze", 9),
        ("hitori:2x2:1221", 1),
    ] {
        assert_eq!(
            Puzzle::from_id(id).unwrap_err().position(),
            Position::Char(at),
            "{id}"
        );
    }
    assert!(Shape::new(usize::MAX, 2).is_err());
    assert!(Shape::new(4, 8).is_err());
    assert!(Shape::new(6, 14).is_ok());
    assert!(Puzzle::from_id(&"x".repeat(4097)).is_err());
    let shape: Shape = "2x2".parse().unwrap();
    assert!(Puzzle::new(shape, vec![Mark::Unmarked; 3]).is_err());
    for rows in [
        vec!["01".to_string(); 18],
        vec!["01".repeat(9); 2],
        vec!["0".into(); 2],
    ] {
        assert!(Puzzle::from_rows(&rows).is_err());
    }
    assert!("1x".parse::<Answer>().is_err());
    assert!("10\n1".parse::<Answer>().is_err());
    #[cfg(feature = "serde")]
    for json in [
        r#"{"grid":["1.",".."],"extra":true}"#.to_string(),
        serde_json::json!({"grid": vec![".."; 18]}).to_string(),
        serde_json::json!({"grid": [".".repeat(1000), ".."]}).to_string(),
        r#"{}"#.to_string(),
    ] {
        assert!(serde_json::from_str::<Puzzle>(&json).is_err());
    }
}

#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: binairo\ngrid:\n1...\n..0.\n....\n.1..\nsolution:\n1010\n0101\n1001\n0110",
        "binairo:6x6:1e0f1b0s",
        r#"{"grid":["1...","..0.","....",".1.."]}"#,
    ];
    assert!(Puzzle::from_id(inputs[1]).is_ok());
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
