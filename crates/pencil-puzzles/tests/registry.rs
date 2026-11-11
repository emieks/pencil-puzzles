use pencil_puzzles::*;
#[cfg(feature = "serde")]
#[test]
fn generation_provenance_round_trips_for_every_genre() {
    for genre in genres() {
        let level = Difficulty::Easy;
        let generated = genre
            .generate("1x1", Some(level), u64::MAX, 100, &Budget::new(100_000))
            .unwrap();
        let value = genre
            .to_json(&generated.text, Some(&generated.rating))
            .unwrap();
        assert!(value.get("generator_version").is_none());
        assert!(value["rating"].get("rater_version").is_none());
        assert_eq!(value["params"], "1x1");
        assert_eq!(value["difficulty"], level.as_str());
        assert_eq!(value["seed"], u64::MAX.to_string());
        let input = genre.read_json(value.clone()).unwrap();
        assert_eq!(input.text, generated.text, "{}", genre.metadata().id);
        assert_eq!(
            genre.to_json(&input.text, input.rating.as_ref()).unwrap(),
            value
        );

        // Older envelopes carry version numbers, which are read and dropped.
        let mut old = value.clone();
        old["generator_version"] = 42.into();
        old["rating"]["rater_version"] = 3.into();
        let input = genre.read_json(old).unwrap();
        assert_eq!(
            genre.to_json(&input.text, input.rating.as_ref()).unwrap(),
            value
        );

        for (key, invalid) in [
            ("generator_version", serde_json::json!(-1)),
            ("generator_version", serde_json::json!("2")),
            ("params", serde_json::json!("invalid")),
            ("difficulty", serde_json::json!("unknown")),
        ] {
            let mut bad = value.clone();
            bad[key] = invalid;
            assert!(
                genre.read_json(bad).is_err(),
                "{}: {key}",
                genre.metadata().id
            );
        }
        // Requests without parameters or an exact level remain supported.
        for key in ["params", "difficulty"] {
            let mut absent = value.clone();
            absent.as_object_mut().unwrap().remove(key);
            let input = genre.read_json(absent.clone()).unwrap();
            assert_eq!(
                genre.to_json(&input.text, input.rating.as_ref()).unwrap(),
                absent
            );
        }
    }
}

#[test]
fn registry_names_are_unique_and_sorted() {
    let mut names = std::collections::BTreeSet::new();
    let mut previous = "";
    for genre in genres() {
        let m = genre.metadata();
        assert!(m.id > previous);
        previous = m.id;
        for name in [m.id].into_iter().chain(m.aliases.iter().copied()) {
            assert!(names.insert(name), "duplicate name: {name}");
            assert_eq!(find(name).unwrap().metadata().id, m.id);
        }
        assert!(!m.summary.is_empty());
        assert!(!m.rules.is_empty());
    }
    assert!(find("not-a-genre").is_none());
}
#[cfg(feature = "kakurasu")]
#[test]
fn text_ids_aliases_and_generic_operations() {
    let g = find("index-sums").unwrap();
    let generated = g
        .generate("3x5", None, 0, 100, &Budget::new(10000))
        .unwrap();
    let id = g.id(&generated.text).unwrap();
    let from_id = read(&id).unwrap();
    assert_eq!(g.id(&from_id[0].text).unwrap(), id);
    assert_eq!(
        g.rate(&generated.text, &Budget::new(10000)).unwrap(),
        generated.rating
    );
    assert!(
        g.check(&generated.text, None, false, &Budget::new(10000))
            .unwrap()
            .is_empty()
    );
    let (rating, steps) = g
        .rate_with_steps(&generated.text, &Budget::new(10000))
        .unwrap();
    assert_eq!(rating, generated.rating);
    assert!(!steps.is_empty());
    let answer = g.solution(&generated.text).unwrap().unwrap();
    assert!(matches!(
        g.hint(&generated.text, Some(&answer), &Budget::new(10000))
            .unwrap(),
        Hint::Solved
    ));
    let no_answer = g.with_solution(&generated.text, None).unwrap();
    assert!(matches!(
        g.hint(&no_answer, None, &Budget::new(10000)).unwrap(),
        Hint::Step(_)
    ));
    let alias = no_answer.replace("genre: kakurasu", "genre: index-sums");
    assert_eq!(read(&alias).unwrap()[0].text, no_answer);
    assert_eq!(
        read(&id.replace("kakurasu:", "index-sums:")).unwrap()[0].text,
        from_id[0].text
    );
    assert_eq!(read(&format!("{alias}\n\n{no_answer}")).unwrap().len(), 2);
    assert!(read("rows: 1\ncols: 1").is_err());
    assert!(g.read_id("other:1x1:0/0").is_err());
    assert!(
        g.with_solution(&from_id[0].text, Some(".....\n.....\n....."))
            .is_err()
    );
}
#[cfg(all(feature = "serde", feature = "kakurasu"))]
#[test]
fn strict_json_envelopes_preserve_large_seeds_and_ratings() {
    let g = find("kakurasu").unwrap();
    let generated = g
        .generate("2x3", None, u64::MAX, 100, &Budget::new(10000))
        .unwrap();
    let value = g.to_json(&generated.text, Some(&generated.rating)).unwrap();
    assert_eq!(value["seed"], u64::MAX.to_string());
    let input = read(&value.to_string()).unwrap().remove(0);
    assert_eq!(input.rating, Some(generated.rating.clone()));
    assert_eq!(
        g.to_json(&input.text, input.rating.as_ref()).unwrap(),
        value
    );
    let mut changed = value.clone();
    changed["future"] = true.into();
    assert!(read(&changed.to_string()).is_err());
    let mut changed = value.clone();
    changed["puzzle"]["future"] = true.into();
    assert!(read(&changed.to_string()).is_err());
    let mut changed = value.clone();
    changed["seed"] = 123.into();
    assert!(read(&changed.to_string()).is_err());
    let mut changed = value.clone();
    changed["solution"] = serde_json::json!(["."]);
    assert!(read(&changed.to_string()).is_err());
    let mut changed = value.clone();
    changed["rating"]["future"] = true.into();
    assert!(read(&changed.to_string()).is_ok());
    let array = serde_json::json!([value.clone(), value]);
    assert_eq!(read(&array.to_string()).unwrap().len(), 2);
    let mut changed = value;
    changed["genre"] = "index-sums".into();
    assert!(read(&changed.to_string()).is_ok());
}

#[cfg(all(feature = "sudoku", feature = "serde"))]
#[test]
fn sudoku_works_through_the_same_registry_and_envelope() {
    let genre = find("sudoku").unwrap();
    let generated = genre
        .generate("4x4", None, u64::MAX, 1000, &Budget::new(100_000))
        .unwrap();
    let value = genre
        .to_json(&generated.text, Some(&generated.rating))
        .unwrap();
    let input = read(&value.to_string()).unwrap().remove(0);
    assert_eq!(input.genre.metadata().id, "sudoku");
    assert_eq!(
        genre.to_json(&input.text, input.rating.as_ref()).unwrap(),
        value
    );
    let editor = genre.editor("9x9", Some(&input.text)).unwrap();
    assert_eq!(editor.shape, "4x4");
    let values: Vec<_> = editor.fields.iter().map(|f| f.value.clone()).collect();
    assert_eq!(
        genre
            .id(&genre.puzzle_from_editor(&editor.shape, &values).unwrap())
            .unwrap(),
        genre.id(&input.text).unwrap()
    );
}

#[cfg(feature = "nurikabe")]
#[test]
fn nurikabe_visual_and_generic_operations_share_the_contract() {
    let g = find("nurikabe").unwrap();
    let source = "nurikabe:3x3:1,.,1,.,.,.,1,.,1";
    let text = g.read_id(source).unwrap();
    assert_eq!(g.id(&text).unwrap(), source);
    let solved = g.solve(&text, &Budget::unlimited()).unwrap();
    let a = solved.any().unwrap();
    assert!(
        g.check(&text, Some(a), true, &Budget::unlimited())
            .unwrap()
            .is_empty()
    );
    assert_eq!(g.editor("5x5", Some(&text)).unwrap().shape, "3x3");
    let fields = g
        .editor("3x3", Some(&text))
        .unwrap()
        .fields
        .into_iter()
        .map(|f| f.value)
        .collect::<Vec<_>>();
    assert_eq!(
        g.id(&g.puzzle_from_editor("3x3", &fields).unwrap())
            .unwrap(),
        source
    );
    #[cfg(feature = "serde")]
    {
        let json = g.to_json(&text, None).unwrap();
        assert_eq!(g.id(&g.read_json(json).unwrap().text).unwrap(), source);
    }
}

#[cfg(feature = "sudoku")]
#[test]
fn sudoku_aliases_and_mixed_text_use_genre_limits() {
    for alias in ["sudoku", "number-place", "nanpure"] {
        let source = format!("{alias}:1x1:.");
        let input = read(&source).unwrap();
        assert_eq!(input[0].genre.metadata().id, "sudoku");
        let source = format!("genre: {alias}\ngrid:\n{}", ".........\n".repeat(100_000));
        let error = read(&source).err().unwrap().to_string();
        assert!(error.contains("line 12"), "{error}");
    }
    #[cfg(feature = "kakurasu")]
    {
        let source = format!(
            "genre: kakurasu\nrows: 0\ncols: 0\ngenre: number-place\ngrid:\n{}",
            ".........\n".repeat(100_000)
        );
        assert!(read(&source).err().unwrap().to_string().contains("line 15"));
    }
}
