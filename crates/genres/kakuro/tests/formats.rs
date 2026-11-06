use pencil_puzzles_core::{Position, SplitMix64};
use pencil_puzzles_kakuro::*;

#[test]
fn all_formats_round_trip_with_metadata() {
    for text in [
        "genre: kakuro\ngrid:\n# 4/ 6/\n/3 . .\n/7 . .",
        "genre: kakuro\nsource: project\nlicense: MIT\ngrid:\n# # 17/ 4/\n# 11/3 . .\n/16 . . .\n/3 . . #\nsolution:\n####\n##21\n#493\n#12#",
        "genre: kakuro\ngrid:\n###\n###\n###",
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
    let p: Puzzle = "grid:\n# 4/ 6/\n/3 . .\n/7 . .".parse().unwrap();
    assert_eq!(p.id(), "kakuro:3x3:DbAb/4.6.3.7");
    assert_eq!(
        p.cell(0, 1),
        Cell::Block {
            down: Some(4),
            across: None
        }
    );
    #[cfg(feature = "serde")]
    assert_eq!(
        serde_json::to_string(&p).unwrap(),
        r##"{"grid":["# 4/ 6/","/3 . .","/7 . ."]}"##
    );
    let a: Answer = "###\n#1.\n#0.".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    assert_eq!(a.get(2, 1), Mark::Empty);
    assert!(!a.finished().is_complete());
    #[cfg(feature = "serde")]
    assert_eq!(
        serde_json::from_str::<Answer>(&serde_json::to_string(&a).unwrap()).unwrap(),
        a
    );
    use pencil_puzzles_core::Genre;
    assert!(Kakuro::parse_answer(&p, "###\n#..\n#..").is_ok());
    assert!(Kakuro::parse_answer(&p, "###\n#..\n...").is_err());
}

#[test]
fn malformed_inputs_have_positions_and_respect_limits() {
    for (text, line) in [
        ("grid:\n# 4/ 6/\n/3 . x\n/7 . .", 3),
        ("grid:\n# 4/ 6/\n/3 . .\n/7 .", 4),
        ("grid:\n# 4/ 6/\n/3 . .\n/7 . .\nunknown: 1", 5),
        (
            "grid:\n# 4/ 6/\n/3 . .\n/7 . .\nsolution:\n###\n#12\n#3.",
            5,
        ),
        ("grid:\n# 4/ #\n/3 . .\n/7 . .", 2),
        ("grid:\n# 4/ 6/\n# . .\n/7 . .", 3),
        ("grid:\n# 4/ 6/\n/3 . .\n/7 . #", 4),
        ("grid:\n# 4/ 6/7\n/3 . .\n/7 . .", 2),
        ("grid:\n# 46/ 6/\n/3 . .\n/7 . .", 2),
        ("grid:\n# / 6/\n/3 . .\n/7 . .", 2),
        ("grid:\n# 04/ 6/\n/3 . .\n/7 . .", 2),
        ("grid:\n. # #\n# # #\n# # #", 2),
        ("grid:\n# # #\n# . #\n# # #", 3),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text:?}"
        );
    }
    for (id, at) in [
        ("kakuro:3x3:DbAb/4.6.3.7x", 24),
        ("kakuro:3x3:DbAb/4.6.3", 22),
        ("kakuro:3x3:Db1b/4.6.3.7", 14),
        ("kakuro:3x3:DbAc/4.6.3.7", 15),
        ("kakuro:3x3:DbAb", 16),
        ("kakuro:3y3:DbAb/4.6.3.7", 9),
        ("kakuro:3x33:DbAb/4.6.3.7", 10),
        ("kakuro:3x3:DaBb/4.6.3.7", 12),
    ] {
        assert_eq!(
            Puzzle::from_id(id).unwrap_err().position(),
            Position::Char(at),
            "{id}"
        );
    }
    assert!(Shape::new(usize::MAX, 3).is_err());
    assert!(Puzzle::from_id(&"x".repeat(4097)).is_err());
    for rows in [vec!["###"; 17], vec!["#################"; 3], vec!["# "; 3]] {
        assert!(Puzzle::from_rows(&rows).is_err());
    }
    assert!("##\n#".parse::<Answer>().is_err());
    assert_eq!("##\n##".parse::<Answer>().unwrap().shape().rows(), 2);
    assert!("###\n#x#\n###".parse::<Answer>().is_err());
    assert!("###\n##\n###".parse::<Answer>().is_err());
    #[cfg(feature = "serde")]
    for json in [
        serde_json::json!({"grid": ["###", "###", "###"], "extra": true}).to_string(),
        serde_json::json!({"grid": vec!["###"; 17]}).to_string(),
        serde_json::json!({"grid": vec!["#".repeat(1000); 3]}).to_string(),
    ] {
        assert!(serde_json::from_str::<Puzzle>(&json).is_err());
    }
}

#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: kakuro\ngrid:\n# 4/ 6/\n/3 . .\n/7 . .\nsolution:\n###\n#12\n#34",
        "kakuro:5x8:IcEcAcCeCbC/17.7.23.17.8.11.17.23.8.21.18.12",
        r##"{"grid":["# 4/ 6/","/3 . .","/7 . ."]}"##,
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
            let _ = hint(&p, &Answer::new(p.shape()), &Budget::new(10_000));
        }
        #[cfg(feature = "serde")]
        {
            let _ = serde_json::from_str::<Puzzle>(&s);
            let _ = serde_json::from_str::<Answer>(&s);
        }
    }
}
