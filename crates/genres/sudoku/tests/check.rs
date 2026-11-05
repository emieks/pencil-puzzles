mod support;
use pencil_puzzles_sudoku::*;
#[test]
fn each_rule_can_be_broken_alone() {
    let blank = Puzzle::new("4x4".parse().unwrap(), vec![0; 16]).unwrap();
    let row: Answer = "3234\n1412\n2143\n4321".parse().unwrap();
    assert!(check(&blank, &row).unwrap().iter().all(|v| v.rule == 1));
    assert!(!check(&blank, &row).unwrap().is_empty());
    let col: Answer = "2134\n3412\n2143\n4321".parse().unwrap();
    let v = check(&blank, &col).unwrap();
    assert!(!v.is_empty());
    assert!(v.iter().all(|v| v.rule == 2));
    let boxes: Answer = "1234\n2341\n3412\n4123".parse().unwrap();
    let v = check(&blank, &boxes).unwrap();
    assert!(!v.is_empty());
    assert!(v.iter().all(|v| v.rule == 3));
    let solution: Answer = "1234\n3412\n2143\n4321".parse().unwrap();
    let p = Puzzle::from_solution(&solution).unwrap();
    assert!(check(&p, &solution).unwrap().is_empty());
    let mut different = solution.clone();
    for r in 0..4 {
        for c in 0..4 {
            different
                .set(
                    r,
                    c,
                    if solution.get(r, c) == 1 {
                        2
                    } else if solution.get(r, c) == 2 {
                        1
                    } else {
                        solution.get(r, c)
                    },
                )
                .unwrap();
        }
    }
    let v = check(&p, &different).unwrap();
    assert!(!v.is_empty());
    assert!(v.iter().all(|v| v.rule == 4));
}
#[test]
fn partial_unit_reports_agree_with_independent_permutations() {
    fn line_possible(values: &[u8]) -> bool {
        fn fill(values: &[u8], chosen: &mut Vec<u8>) -> bool {
            if chosen.len() == values.len() {
                return true;
            }
            for value in 1..=4 {
                if (values[chosen.len()] == 0 || values[chosen.len()] == value)
                    && !chosen.contains(&value)
                {
                    chosen.push(value);
                    if fill(values, chosen) {
                        return true;
                    }
                    chosen.pop();
                }
            }
            false
        }
        fill(values, &mut Vec::new())
    }
    let p = Puzzle::new("4x4".parse().unwrap(), vec![0; 16]).unwrap();
    let mut rng = pencil_puzzles_core::SplitMix64::new(677);
    for _ in 0..500 {
        let mut a = Answer::new(p.shape());
        for r in 0..4 {
            for c in 0..4 {
                a.set(r, c, rng.below(5) as u8).unwrap();
            }
        }
        let actual = check(&p, &a).unwrap();
        for i in 0..4 {
            let row: Vec<_> = (0..4).map(|c| a.get(i, c)).collect();
            let col: Vec<_> = (0..4).map(|r| a.get(r, i)).collect();
            let region: Vec<_> = (0..4)
                .map(|j| a.get(i / 2 * 2 + j / 2, i % 2 * 2 + j % 2))
                .collect();
            for (place, values) in [
                (Place::Row(i as u32), row),
                (Place::Col(i as u32), col),
                (Place::Region(i as u32), region),
            ] {
                assert_eq!(
                    actual.iter().any(|v| v.place == place),
                    !line_possible(&values)
                );
            }
        }
    }
    let generated = generate(
        &GenerateOptions::new("4x4".parse().unwrap(), 2),
        &Budget::unlimited(),
    )
    .unwrap();
    for mask in 0u32..256 {
        let mut a = Answer::new(p.shape());
        for i in 0..16 {
            if mask >> (i % 8) & 1 != 0 {
                a.set(i / 4, i % 4, generated.solution.get(i / 4, i % 4))
                    .unwrap();
            }
        }
        assert!(check(&generated.puzzle, &a).unwrap().is_empty());
    }
    assert!(
        !check(&p, &Answer::new(p.shape()).finished())
            .unwrap()
            .is_empty()
    );
}
