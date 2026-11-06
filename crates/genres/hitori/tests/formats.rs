use pencil_puzzles_core::{Position, SplitMix64};
use pencil_puzzles_hitori::*;

#[test]
fn all_formats_round_trip_with_two_digit_numbers_and_metadata() {
    let wide = (1..=11)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    for text in [
        "genre: hitori\ngrid:\n11\n12".to_string(),
        "genre: hitori\nsource: project\nlicense: MIT\ngrid:\n12\n21\nsolution:\n--\n--".into(),
        format!("genre: hitori\ngrid:\n{wide}\n{wide}"),
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
    let p: Puzzle = "grid:\n11\n12\n33".parse().unwrap();
    assert_eq!(p.id(), "hitori:3x2:111233");
    let wide: Puzzle = format!("grid:\n{wide}\n{wide}").parse().unwrap();
    assert_eq!(wide.id(), "hitori:2x11:123456789ab123456789ab");
    let a: Answer = "#-\n.-\n--".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    assert_eq!(a.get(1, 0), Mark::Unmarked);
    assert!(a.finished().is_complete());
    assert_eq!(p.render(Some(&a)), "# 1\n1 2\n3 3");
    #[cfg(feature = "serde")]
    {
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"grid":["11","12","33"]}"#
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
        ("grid:\n12\n2x", 3),
        ("grid:\n0", 2),
        ("grid:\n.", 2),
        ("grid:\n3", 2),
        ("grid:\n1\nunknown: 1", 3),
        ("grid:\n1\nsolution:\n.", 3),
        ("grid:\n1 2\n3", 3),
        ("grid:\n12\n1", 3),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text:?}"
        );
    }
    for (id, at) in [
        ("hitori:2x2:1221x", 16),
        ("hitori:2x2:122", 15),
        ("hitori:2x2:1231", 14),
        ("hitori:2x2:0221", 12),
        ("hitori:2y2:1221", 9),
        ("hitori:2x13:a", 10),
        ("sudoku:2x2:1221", 1),
    ] {
        assert_eq!(
            Puzzle::from_id(id).unwrap_err().position(),
            Position::Char(at),
            "{id}"
        );
    }
    assert!(Shape::new(usize::MAX, 1).is_err());
    assert!(Puzzle::from_id(&"x".repeat(4097)).is_err());
    for rows in [vec!["1"; 13], vec!["1111111111111"; 12], vec!["1 "; 12]] {
        assert!(Puzzle::from_rows(&rows).is_err());
    }
    assert!(Puzzle::new("2x2".parse().unwrap(), vec![1, 2, 3, 1]).is_err());
    assert!("#x".parse::<Answer>().is_err());
    assert!("#-\n-".parse::<Answer>().is_err());
    #[cfg(feature = "serde")]
    for json in [
        r#"{"grid":["1"],"extra":true}"#.to_string(),
        serde_json::json!({"grid": vec!["1"; 13]}).to_string(),
        serde_json::json!({"grid": vec!["1".repeat(1000); 12]}).to_string(),
    ] {
        assert!(serde_json::from_str::<Puzzle>(&json).is_err());
    }
}

#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: hitori\ngrid:\n11\n12\nsolution:\n#-\n--",
        "hitori:3x3:123231312",
        r#"{"grid":["11","12"]}"#,
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
