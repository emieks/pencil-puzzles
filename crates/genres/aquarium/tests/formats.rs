use pencil_puzzles_aquarium::*;
use pencil_puzzles_core::{Position, SplitMix64};

/// A 15x15 grid of one-cell tanks, which needs numbers for labels.
fn single_cells() -> String {
    let regions: Vec<String> = (0..15)
        .map(|r| {
            (1..=15)
                .map(|c| (r * 15 + c).to_string())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    let zeros = vec!["0"; 15].join(" ");
    format!(
        "genre: aquarium\nrows: {zeros}\ncols: {zeros}\nregions:\n{}",
        regions.join("\n")
    )
}

#[test]
fn all_formats_round_trip_with_letters_numbers_and_metadata() {
    for text in [
        "genre: aquarium\nrows: 1 2\ncols: 2 1\nregions:\nab\naa".to_string(),
        "genre: aquarium\nsource: project\nlicense: MIT\nrows: 1 2\ncols: 2 1\nregions:\nab\naa\nsolution:\n#-\n##".into(),
        "genre: aquarium\nrows: 0\ncols: 0\nregions:\na".into(),
        single_cells(),
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
    // Tanks may have any letters or numbers; they are written in order.
    let p: Puzzle = "rows: 1 2\ncols: 2 1\nregions:\nzQ\nzz".parse().unwrap();
    assert_eq!(p.regions(), ["ab", "aa"]);
    let numbered: Puzzle = "rows: 1 2\ncols: 2 1\nregions:\n12 7\n12 12"
        .parse()
        .unwrap();
    assert_eq!(numbered, p);
    assert_eq!(p.id(), "aquarium:2x2:12/21/k");
    assert_eq!(p.tank_count(), 2);
    let one: Puzzle = "rows: 1\ncols: 1\nregions:\na".parse().unwrap();
    assert_eq!(one.id(), "aquarium:1x1:1/1/");
    let wide: Puzzle = single_cells().parse().unwrap();
    assert_eq!(wide.tank_count(), 225);
    assert_eq!(Puzzle::from_id(&wide.id()).unwrap(), wide);
    let a: Answer = "#-\n.-".parse().unwrap();
    assert_eq!(a.to_string().parse::<Answer>().unwrap(), a);
    assert_eq!(a.get(1, 0), Mark::Unmarked);
    assert!(a.finished().is_complete());
    #[cfg(feature = "serde")]
    {
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"row_clues":[1,2],"col_clues":[2,1],"regions":["ab","aa"]}"#
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
        ("rows: 1 2\ncols: 2 1\nregions:\nab\na?", 5),
        ("rows: 1 2\ncols: 2 1\nregions:\nab\na", 5),
        ("rows: 1 2\ncols: 2 1\nregions:\nab\nba", 4),
        ("rows: 1 3\ncols: 2 1\nregions:\nab\naa", 1),
        ("rows: 1 x\ncols: 2 1\nregions:\nab\naa", 1),
        ("rows: 1 2\ncols: 2 01\nregions:\nab\naa", 2),
        ("rows: 1 2 0\ncols: 2 1\nregions:\nab\naa", 4),
        ("rows: 1 2\ncols: 2 1", 1),
        ("cols: 2 1\nregions:\nab\naa", 1),
        ("rows: 1 2\ncols: 2 1\nregions:\nab\naa\nunknown: 1", 6),
        (
            "rows: 1 2\ncols: 2 1\nregions:\nab\naa\nsolution:\n#.\n##",
            6,
        ),
        ("rows: 1 2\ncols: 2 1\nregions:\n1 0\n1 1", 4),
    ] {
        assert_eq!(
            text.parse::<Puzzle>().unwrap_err().position(),
            Position::Line(line),
            "{text:?}"
        );
    }
    for (id, at) in [
        ("aquarium:2x2:12/21/kA", 21),
        ("aquarium:2x2:12/21/", 20),
        ("aquarium:2x2:12/21/l", 20),
        ("aquarium:2x2:12/21/!", 20),
        ("aquarium:2x2:12/21/g", 20),
        ("aquarium:2x2:12/2/k", 18),
        ("aquarium:2x2:13/21/k", 15),
        ("aquarium:2x2:1A/21/k", 15),
        ("aquarium:2x2:123/21/k", 16),
        ("aquarium:2x2:12/21", 19),
        ("aquarium:2y2:12/21/k", 11),
        ("aquarium:2x16:1/1/A", 12),
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
    assert!(Puzzle::new(shape, &[0, 1, 1, 0], vec![0, 0], vec![0, 0]).is_err());
    assert!(Puzzle::new(shape, &[0, 0, 0], vec![0, 0], vec![0, 0]).is_err());
    assert!(Puzzle::new(shape, &[0, 0, 0, 0], vec![0, 3], vec![0, 0]).is_err());
    assert!(Puzzle::new(shape, &[9, 9, 4, 4], vec![0, 2], vec![1, 1]).is_ok());
    for rows in [vec!["a"; 16], vec!["aaaaaaaaaaaaaaaa"; 15], vec!["a "; 15]] {
        assert!(Puzzle::from_rows(&rows, vec![0; rows.len()], vec![0]).is_err());
    }
    assert!("#x".parse::<Answer>().is_err());
    assert!("#-\n-".parse::<Answer>().is_err());
    #[cfg(feature = "serde")]
    for json in [
        r#"{"row_clues":[0],"col_clues":[0],"regions":["a"],"extra":true}"#.to_string(),
        serde_json::json!({"row_clues": vec![0; 16], "col_clues": [0], "regions": vec!["a"; 16]})
            .to_string(),
        serde_json::json!({"row_clues": [0], "col_clues": [0], "regions": ["a".repeat(1000)]})
            .to_string(),
        r#"{"row_clues":[0],"col_clues":[0]}"#.to_string(),
    ] {
        assert!(serde_json::from_str::<Puzzle>(&json).is_err());
    }
}

#[test]
fn ten_thousand_mutations_do_not_panic() {
    let inputs = [
        "genre: aquarium\nrows: 1 2\ncols: 2 1\nregions:\nab\naa\nsolution:\n#-\n##",
        "aquarium:3x3:123/222/__",
        r#"{"row_clues":[1,2],"col_clues":[2,1],"regions":["ab","aa"]}"#,
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
