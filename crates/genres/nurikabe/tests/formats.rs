use pencil_puzzles_core::{Position, SplitMix64};
use pencil_puzzles_nurikabe::*;
#[test]
fn all_formats_round_trip_with_multidigit_clues_and_metadata() {
    for text in [
        "genre: nurikabe\ngrid:\n1.1\n...\n1.1",
        "genre: nurikabe\nsource: project\nlicense: MIT\ngrid:\n12 . . .\n. . . .\n. . . .\nsolution:\n----\n----\n####",
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
    let a: Answer = "#-.\n...".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    assert!(a.finished().is_complete());
    #[cfg(feature = "serde")]
    assert_eq!(
        serde_json::from_str::<Answer>(&serde_json::to_string(&a).unwrap()).unwrap(),
        a
    );
}
#[test]
fn malformed_inputs_have_positions_and_respect_limits() {
    for (text, line) in [
        ("grid:\n1.1\n.x.", 3),
        ("grid:\n0", 2),
        ("grid:\n2", 2),
        ("grid:\n.\nunknown: 1", 3),
        ("grid:\n.\nsolution:\n.", 3),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line)
        );
    }
    assert_eq!(
        Puzzle::from_id("nurikabe:3x3:1,.,x,.,.,.,1,.,1")
            .unwrap_err()
            .position(),
        Position::Char(18)
    );
    assert!(Shape::new(usize::MAX, 1).is_err());
    assert!(Puzzle::from_id(&"x".repeat(4097)).is_err());
    for rows in [vec!["........."; 8], vec!["."; 9], vec![". "; 8]] {
        assert!(Puzzle::from_rows(&rows).is_err());
    }
    #[cfg(feature = "serde")]
    {
        for json in [
            r#"{"grid":["1"],"extra":true}"#.to_string(),
            serde_json::json!({"grid":vec![".";9]}).to_string(),
            serde_json::json!({"grid":vec!["x".repeat(1000);8]}).to_string(),
        ] {
            assert!(serde_json::from_str::<Puzzle>(&json).is_err());
        }
    }
}
#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: nurikabe\ngrid:\n1.1\n...\n1.1",
        "nurikabe:3x3:1,.,1,.,.,.,1,.,1",
        r#"{"grid":["1.1","...","1.1"]}"#,
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
        let _ = s.parse::<Puzzle>();
        let _ = s.parse::<Answer>();
        let _ = Puzzle::from_id(&s);
        #[cfg(feature = "serde")]
        {
            let _ = serde_json::from_str::<Puzzle>(&s);
            let _ = serde_json::from_str::<Answer>(&s);
        }
    }
}
