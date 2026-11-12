use pencil_puzzles_chocona::*;
use pencil_puzzles_core::{Position, SplitMix64};

/// A 15x15 grid of one-cell regions, which needs numbers for labels, with
/// clues of one and two characters in IDs.
fn single_cells() -> String {
    let regions: Vec<String> = (0..15)
        .map(|r| {
            (1..=15)
                .map(|c| (r * 15 + c).to_string())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    let clues: Vec<&str> = (0..225).map(|i| ["0", "1", "."][i % 3]).collect();
    format!(
        "genre: chocona\nregions:\n{}\nclues: {}",
        regions.join("\n"),
        clues.join(" ")
    )
}

/// One region of 225 cells with a large clue, which needs the `-` form in
/// an ID.
fn one_region() -> String {
    let regions = vec!["a".repeat(15); 15];
    format!(
        "genre: chocona\nregions:\n{}\nclues: 200",
        regions.join("\n")
    )
}

#[test]
fn all_formats_round_trip_with_letters_numbers_and_metadata() {
    for text in [
        "genre: chocona\nregions:\naab\nacc\nclues: 2 . 0".to_string(),
        "genre: chocona\nsource: project\nlicense: MIT\nregions:\naab\nacc\nclues: 3 . 1\nsolution:\n##-\n##-".into(),
        "genre: chocona\nregions:\na\nclues: 1".into(),
        single_cells(),
        one_region(),
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
    // Regions may have any letters or numbers; they are written in order.
    let p: Puzzle = "regions:\nzzQ\nzRR\nclues: 2 . 0".parse().unwrap();
    assert_eq!(p.region_rows(), ["aab", "acc"]);
    let numbered: Puzzle = "regions:\n12 12 7\n12 3 3\nclues: 2 . 0".parse().unwrap();
    assert_eq!(numbered, p);
    assert_eq!(p.id(), "chocona:2x3:Zg/2.0");
    assert_eq!(p.region_count(), 3);
    assert_eq!(p.clue_cells(), [0, 2, 4]);
    let one: Puzzle = "regions:\na\nclues: .".parse().unwrap();
    assert_eq!(one.id(), "chocona:1x1:/.");
    let big: Puzzle = one_region().parse().unwrap();
    assert_eq!(big.id(), format!("chocona:15x15:{}/-5k", "A".repeat(70)));
    let wide: Puzzle = single_cells().parse().unwrap();
    assert_eq!(wide.region_count(), 225);
    let a: Answer = "#-.\n.-#".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    assert_eq!(a.get(1, 0), Mark::Unmarked);
    assert!(a.finished().is_complete());
    #[cfg(feature = "serde")]
    {
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"regions":["aab","acc"],"clues":[2,null,0]}"#
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
        ("regions:\naab\na?c\nclues: 2 . 0", 3),
        ("regions:\naab\nac\nclues: 2 . 0", 3),
        ("regions:\naba\nbbb\nclues: 2 . 0", 2),
        ("regions:\naab\nacc\nclues: 2 .", 4),
        ("regions:\naab\nacc\nclues: 2 . 0 1", 4),
        ("regions:\naab\nacc\nclues: 4 . 0", 4),
        ("regions:\naab\nacc\nclues: 2 x 0", 4),
        ("regions:\naab\nacc\nclues: 02 . 0", 4),
        ("regions:\naab\nacc", 1),
        ("clues: 2 . 0", 1),
        ("regions:\naab\nacc\nclues: 2 . 0\nunknown: 1", 5),
        ("regions:\naab\nacc\nclues: 2 . 0\nsolution:\n#.#\n---", 5),
        ("regions:\n1 0\n1 1\nclues: 1 1", 2),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text:?}"
        );
    }
    for (id, at) in [
        ("chocona:2x3:ZgA/2.0", 15),
        ("chocona:2x3:Z/2.0", 14),
        ("chocona:2x3:Zh/2.0", 14),
        ("chocona:2x3:Z!/2.0", 14),
        ("chocona:2x3:Zg/2.", 18),
        ("chocona:2x3:Zg/2.01", 19),
        ("chocona:2x3:Zg/4.0", 16),
        ("chocona:2x3:Zg/2!0", 17),
        ("chocona:2x3:Zg/-01", 16),
        ("chocona:2x3:Zg/-0", 18),
        ("chocona:2x3:Zg", 15),
        ("chocona:2y3:Zg/2.0", 10),
        ("chocona:2x16:A/1", 11),
        ("hitori:2x2:1221", 1),
    ] {
        assert_eq!(
            Puzzle::from_id(id).unwrap_err().position(),
            Position::Char(at),
            "{id}"
        );
    }
    assert!(Shape::new(usize::MAX, 1).is_err());
    assert!(Puzzle::from_id(&"x".repeat(4097)).is_err());
    let shape: Shape = "2x2".parse().unwrap();
    assert!(Puzzle::new(shape, &[0, 1, 1, 0], vec![None, None]).is_err());
    assert!(Puzzle::new(shape, &[0, 0, 0], vec![None]).is_err());
    assert!(Puzzle::new(shape, &[0, 0, 0, 0], vec![Some(5)]).is_err());
    assert!(Puzzle::new(shape, &[0, 0, 0, 0], vec![None, None]).is_err());
    assert!(Puzzle::new(shape, &[9, 9, 4, 4], vec![Some(2), None]).is_ok());
    for rows in [vec!["a"; 16], vec!["aaaaaaaaaaaaaaaa"; 15], vec!["a "; 15]] {
        assert!(Puzzle::from_rows(&rows, vec![None]).is_err());
    }
    assert!("#x".parse::<Answer>().is_err());
    assert!("#-\n-".parse::<Answer>().is_err());
    #[cfg(feature = "serde")]
    for json in [
        r#"{"regions":["a"],"clues":[1],"extra":true}"#.to_string(),
        serde_json::json!({"regions": vec!["a"; 16], "clues": [1]}).to_string(),
        serde_json::json!({"regions": ["a".repeat(1000)], "clues": [1]}).to_string(),
        serde_json::json!({"regions": ["a"], "clues": vec![1; 1000]}).to_string(),
        r#"{"regions":["a"]}"#.to_string(),
        r#"{"regions":["a"],"clues":[2]}"#.to_string(),
    ] {
        assert!(serde_json::from_str::<Puzzle>(&json).is_err(), "{json}");
    }
}

#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: chocona\nregions:\naab\nacc\nclues: 3 . 1\nsolution:\n##-\n##-",
        "chocona:2x3:Zg/2.0",
        r#"{"regions":["aab","acc"],"clues":[2,null,0]}"#,
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
