use pencil_puzzles_kakurasu::*;
#[test]
fn each_rule_can_be_broken_alone() {
    let answer: Answer = "#-\n-#".parse().unwrap();
    for (puzzle, rule) in [
        (Puzzle::new(vec![0, 2], vec![1, 2]).unwrap(), 1),
        (Puzzle::new(vec![1, 2], vec![0, 2]).unwrap(), 2),
    ] {
        let violations = check(&puzzle, &answer).unwrap();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule, rule);
    }
    assert!(
        check(&Puzzle::from_solution(&answer), &answer)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn partial_answers_report_exactly_the_lines_without_completions() {
    let p = Puzzle::new(vec![1, 2], vec![1, 2]).unwrap();
    for encoded in 0..81 {
        let mut n = encoded;
        let mut answer = Answer::new(p.shape());
        for i in 0..4 {
            answer.set(
                i / 2,
                i % 2,
                [Mark::Unmarked, Mark::Shaded, Mark::Unshaded][n % 3],
            );
            n /= 3;
        }
        let violations = check(&p, &answer).unwrap();
        for line in 0..4 {
            let target = if line < 2 {
                p.row_clues()[line]
            } else {
                p.col_clues()[line - 2]
            };
            let fits = (0..4).any(|bits| {
                (0..2).all(|k| {
                    let mark = if line < 2 {
                        answer.get(line, k)
                    } else {
                        answer.get(k, line - 2)
                    };
                    mark == Mark::Unmarked || (mark == Mark::Shaded) == (bits >> k & 1 == 1)
                }) && (0..2)
                    .filter(|k| bits >> k & 1 == 1)
                    .map(|k| k as u32 + 1)
                    .sum::<u32>()
                    == target
            });
            let place = if line < 2 {
                Place::Row(line as u32)
            } else {
                Place::Col((line - 2) as u32)
            };
            assert_eq!(violations.iter().any(|v| v.place == place), !fits);
        }
    }
}

#[test]
fn view_ignores_an_answer_with_a_different_shape() {
    use pencil_puzzles_core::Genre;
    let p: Puzzle = "rows: 0 0 0\ncols: 0 0 0".parse().unwrap();
    let a = Answer::new("1x1".parse().unwrap());
    let view = Kakurasu::view(&p, Some(&a));
    assert_eq!(view.cells.len(), 9);
    assert!(view.cells.iter().all(|c| c.value == "."));
}
