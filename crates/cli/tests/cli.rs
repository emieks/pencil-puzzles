use std::{
    io::Write,
    process::{Command, Output, Stdio},
};
fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pencil-puzzles"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}
fn success(o: &Output) {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
#[test]
fn list_generate_and_aliases() {
    let o = run(&["list", "--format", "json"], "");
    success(&o);
    let value: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
    assert_eq!(value[0]["id"], "fillomino");
    assert_eq!(value[1]["id"], "hashi");
    assert_eq!(value[2]["id"], "kakurasu");
    let args = ["generate", "index-sums", "--shape", "3x5", "--seed", "42"];
    let a = run(&args, "");
    success(&a);
    assert_eq!(stdout(&a), stdout(&run(&args, "")));
    assert!(stdout(&a).contains("seed: 42"));
    assert!(!stdout(&a).contains("solution:"));
    assert!(pencil_puzzles::read(&stdout(&a)).is_ok());
}
#[test]
fn pipelines_solve_rate_check_and_hint() {
    let g = run(
        &[
            "generate",
            "kakurasu",
            "--params",
            "6x6",
            "--count",
            "3",
            "--seed",
            "7",
            "--solution",
        ],
        "",
    );
    success(&g);
    let text = stdout(&g);
    let s = run(&["solve"], &text);
    success(&s);
    assert_eq!(stdout(&s).matches("solution:").count(), 3);
    let r = run(&["rate", "--format", "json"], &text);
    success(&r);
    let value: serde_json::Value = serde_json::from_str(&stdout(&r)).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 3);
    assert!(value[0]["rating"]["steps"].is_u64());
    assert!(value[0]["rating"]["techniques"].is_array());
    let c = run(&["check", "--format", "json"], &text);
    success(&c);
    let value: serde_json::Value = serde_json::from_str(&stdout(&c)).unwrap();
    assert_eq!(value[0]["violations"], serde_json::json!([]));
    let h = run(&["hint", "--format", "json"], &text);
    success(&h);
    let value: serde_json::Value = serde_json::from_str(&stdout(&h)).unwrap();
    assert_eq!(value[0]["hint"], "solved");
}
#[test]
fn json_and_ids_are_inputs() {
    let g = run(
        &[
            "generate",
            "kakurasu",
            "--seed",
            "18446744073709551615",
            "--shape",
            "3x3",
            "--format",
            "json",
        ],
        "",
    );
    success(&g);
    let v: serde_json::Value = serde_json::from_str(&stdout(&g)).unwrap();
    assert_eq!(v["seed"], "18446744073709551615");
    let s = run(&["solve", "--format", "json"], &stdout(&g));
    success(&s);
    let v: serde_json::Value = serde_json::from_str(&stdout(&s)).unwrap();
    assert!(v["solution"].is_array());
    let s = run(
        &[
            "solve",
            "--id",
            "kakurasu:2x2:1.2/1.2",
            "--format",
            "pretty",
        ],
        "",
    );
    success(&s);
    assert!(stdout(&s).contains("# -"));
    let h = run(&["hint", "--id", "index-sums:2x2:1.2/1.2"], "");
    success(&h);
    assert!(stdout(&h).contains("row 1"));
}
#[test]
fn partial_and_finished_answers_and_files() {
    let path = std::env::temp_dir().join(format!("pencil-answer-{}.txt", std::process::id()));
    std::fs::write(&path, "..\n..").unwrap();
    let p = path.to_str().unwrap();
    let c = run(
        &["check", "--id", "kakurasu:2x2:1.2/1.2", "--answer", p],
        "",
    );
    success(&c);
    let c = run(
        &[
            "check",
            "--id",
            "kakurasu:2x2:1.2/1.2",
            "--answer",
            p,
            "--finished",
        ],
        "",
    );
    assert!(!c.status.success());
    assert!(stdout(&c).contains("R1 at row 1"));
    std::fs::write(&path, "genre: kakurasu\nrows: 1 2\ncols: 1 2").unwrap();
    success(&run(&["rate", p], ""));
    std::fs::remove_file(path).unwrap();
}
#[test]
fn multiple_and_missing_solutions_have_valid_output() {
    let m = run(&["solve"], "genre: kakurasu\nrows: 5 5 5 5\ncols: 5 5 5 5");
    success(&m);
    assert_eq!(pencil_puzzles::read(&stdout(&m)).unwrap().len(), 2);
    assert!(String::from_utf8_lossy(&m.stderr).contains("multiple solutions"));
    let n = run(&["solve", "--id", "kakurasu:1x1:1/0"], "");
    assert!(!n.status.success());
    assert!(String::from_utf8_lossy(&n.stderr).contains("no solution"));
    assert!(
        !run(&["rate", "--id", "kakurasu:4x4:5.5.5.5/5.5.5.5"], "")
            .status
            .success()
    );
}
#[test]
fn invalid_inputs_and_small_budgets_fail_cleanly() {
    let o = run(&["solve"], "genre: kakurasu\nrows: 1 2\ncols: 1 banana");
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("line 3"));
    for command in ["solve", "rate", "hint"] {
        let o = run(
            &[command, "--id", "kakurasu:2x2:1.2/1.2", "--budget", "0"],
            "",
        );
        assert!(!o.status.success());
        assert!(String::from_utf8_lossy(&o.stderr).contains("budget"));
    }
    assert!(
        !run(&["generate", "kakurasu", "--budget", "0"], "")
            .status
            .success()
    );
    assert!(
        !run(
            &[
                "generate",
                "kakurasu",
                "--seed",
                "18446744073709551615",
                "--count",
                "2"
            ],
            ""
        )
        .status
        .success()
    );
    assert!(!run(&["generate", "unknown"], "").status.success());
}

#[test]
fn genre_subcommands_follow_the_same_registry_contract() {
    success(&run(&["kakurasu", "generate", "--help"], ""));
    let g = run(&["index-sums", "generate", "--shape", "1x1"], "");
    success(&g);
    success(&run(&["kakurasu", "solve"], &stdout(&g)));
    assert!(!run(&["unknown", "generate"], "").status.success());
}

#[test]
fn sudoku_is_discovered_for_generate_solve_rate_check_and_hint() {
    let generated = run(
        &[
            "generate",
            "sudoku",
            "--shape",
            "4x4",
            "--seed",
            "7",
            "--solution",
        ],
        "",
    );
    success(&generated);
    let text = stdout(&generated);
    for command in ["solve", "rate", "check", "hint"] {
        success(&run(&[command, "--format", "json"], &text));
    }
    success(&run(&["sudoku", "solve"], &text));
    assert_eq!(
        pencil_puzzles::read(&text).unwrap()[0].genre.metadata().id,
        "sudoku"
    );
}

#[test]
fn nurikabe_is_discovered_for_generate_solve_rate_check_and_hint() {
    let generated = run(
        &[
            "generate",
            "nurikabe",
            "--shape",
            "3x3",
            "--seed",
            "7",
            "--solution",
        ],
        "",
    );
    success(&generated);
    let text = stdout(&generated);
    for command in ["solve", "rate", "check", "hint"] {
        success(&run(&[command, "--format", "json"], &text));
    }
    success(&run(&["nurikabe", "solve"], &text));
    assert_eq!(
        pencil_puzzles::read(&text).unwrap()[0].genre.metadata().id,
        "nurikabe"
    );
}

#[test]
fn sudoku_aliases_work_as_generic_subcommands() {
    let g = run(
        &["number-place", "generate", "--shape", "4x4", "--seed", "0"],
        "",
    );
    success(&g);
    let s = run(&["nanpure", "solve", "--format", "json"], &stdout(&g));
    success(&s);
    let json: serde_json::Value = serde_json::from_str(&stdout(&s)).unwrap();
    assert_eq!(json["genre"], "sudoku");
}
