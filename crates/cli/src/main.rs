//! The `pencil-puzzles` command-line tool, built on the genre registry.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
use anyhow::{Result, anyhow, bail};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use pencil_puzzles::{Budget, Difficulty, Input, Rating, Solutions, find, genres, read};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
};
#[derive(Parser)]
#[command(
    name = "pencil-puzzles",
    version,
    about = "Generate, solve, rate, check and hint pencil puzzles."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Run a command under a registered genre id or alias (N3).
    #[command(external_subcommand)]
    Genre(Vec<String>),
    /// List every compiled-in genre and its aliases.
    List {
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
    /// Generate unique puzzles with reproducible ratings.
    Generate {
        genre: String,
        #[arg(long, conflicts_with = "params")]
        shape: Option<String>,
        #[arg(long)]
        params: Option<String>,
        #[arg(long)]
        difficulty: Option<Difficulty>,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long,default_value_t=1,value_parser=clap::value_parser!(u32).range(1..))]
        count: u32,
        #[arg(long)]
        solution: bool,
        #[arg(long, default_value_t = 10_000)]
        max_attempts: u32,
        #[arg(long, default_value_t = 100_000_000)]
        budget: u64,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
    },
    /// Solve files, stdin or a puzzle ID, proving uniqueness.
    Solve(InputArgs),
    /// Rate uniquely solvable puzzles.
    Rate(InputArgs),
    /// Check an answer file, or a record's solution block.
    Check {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        answer: Option<PathBuf>,
        /// Treat unmarked cells as the genre's finished default.
        #[arg(long)]
        finished: bool,
    },
    /// Give the first canonical step for a partial answer.
    Hint {
        #[command(flatten)]
        input: InputArgs,
        #[arg(long)]
        answer: Option<PathBuf>,
    },
    /// Print a shell completion script to standard output.
    Completions {
        /// The shell to generate a script for.
        shell: Shell,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Pretty,
    Json,
}
#[derive(Args)]
struct InputArgs {
    /// Input file, or - for stdin. Omit to read stdin.
    #[arg(conflicts_with = "id")]
    file: Option<PathBuf>,
    /// Read a puzzle ID directly.
    #[arg(long)]
    id: Option<String>,
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
    #[arg(long, default_value_t = 100_000_000)]
    budget: u64,
}
fn convert<T>(result: Result<T, pencil_puzzles::RegistryError>) -> Result<T> {
    result.map_err(|e| anyhow!(e.to_string()))
}
fn inputs(args: &InputArgs, selected: Option<&str>) -> Result<Vec<Input>> {
    let text = if let Some(id) = &args.id {
        id.clone()
    } else if let Some(path) = &args.file {
        if path.as_os_str() == "-" {
            stdin()?
        } else {
            fs::read_to_string(path)?
        }
    } else {
        stdin()?
    };
    let records = convert(read(&text))?;
    if let Some(selected) = selected
        && records.iter().any(|r| r.genre.metadata().id != selected)
    {
        bail!("the input contains a different genre from {selected}");
    }
    Ok(records)
}
fn stdin() -> Result<String> {
    let mut text = String::new();
    io::stdin().read_to_string(&mut text)?;
    Ok(text)
}
fn record_output(
    input: &Input,
    format: Format,
    solution: Option<&str>,
    rating: Option<&Rating>,
) -> Result<String> {
    Ok(match format {
        Format::Text => input.text.clone(),
        Format::Pretty => format!(
            "{}\n{}{}",
            convert(input.genre.id(&input.text))?,
            convert(input.genre.render(&input.text, solution))?,
            rating
                .map(|r| format!("\n{} (score {})", r.level, r.score))
                .unwrap_or_default()
        ),
        Format::Json => serde_json::to_string(&convert(input.genre.to_json(&input.text, rating))?)?,
    })
}
fn print_all(outputs: Vec<String>, format: Format) -> Result<()> {
    if format == Format::Json && outputs.len() != 1 {
        let values: Vec<serde_json::Value> = outputs
            .iter()
            .map(|s| serde_json::from_str(s))
            .collect::<std::result::Result<_, _>>()?;
        println!("{}", serde_json::to_string(&values)?);
    } else {
        println!("{}", outputs.join("\n\n"));
    }
    Ok(())
}
fn run(cli: Cli, selected: Option<&str>) -> Result<bool> {
    match cli.command {
        Command::Genre(args) => {
            let (name, rest) = args
                .split_first()
                .ok_or_else(|| anyhow!("expected a genre id"))?;
            let genre = find(name).ok_or_else(|| {
                anyhow!("unknown genre {name:?}; run list to see available genres")
            })?;
            let (operation, rest) = rest.split_first().ok_or_else(|| {
                anyhow!("expected generate, solve, rate, check or hint after {name}")
            })?;
            if !["generate", "solve", "rate", "check", "hint"].contains(&operation.as_str()) {
                bail!("unknown command {operation:?}");
            }
            let mut args = vec!["pencil-puzzles".to_string(), operation.clone()];
            if operation == "generate" {
                args.push(name.clone());
            }
            args.extend_from_slice(rest);
            let cli = match Cli::try_parse_from(args) {
                Ok(cli) => cli,
                Err(error)
                    if matches!(
                        error.kind(),
                        clap::error::ErrorKind::DisplayHelp
                            | clap::error::ErrorKind::DisplayVersion
                    ) =>
                {
                    error.print()?;
                    return Ok(true);
                }
                Err(error) => return Err(error.into()),
            };
            return run(cli, Some(genre.metadata().id));
        }
        Command::List { format } => {
            if format == Format::Json {
                println!(
                    "{}",
                    serde_json::to_string(
                        &genres().iter().map(|g| g.metadata()).collect::<Vec<_>>()
                    )?
                );
            } else {
                for g in genres() {
                    let m = g.metadata();
                    println!(
                        "{}\t{}\t{}{}",
                        m.id,
                        m.name,
                        m.summary,
                        if m.aliases.is_empty() {
                            String::new()
                        } else {
                            format!(" (aliases: {})", m.aliases.join(", "))
                        }
                    );
                }
            }
        }
        Command::Generate {
            genre,
            shape,
            params,
            difficulty,
            seed,
            count,
            solution,
            max_attempts,
            budget,
            format,
        } => {
            let genre = find(&genre).ok_or_else(|| {
                anyhow!("unknown genre {genre:?}; run list to see available genres")
            })?;
            let params = match (params, shape) {
                (Some(p), _) => p,
                (_, Some(s)) => convert(genre.params_for_shape(&s))?,
                _ => genre.default_params(),
            };
            // Check the entire seed range before producing partial output.
            if seed.checked_add(u64::from(count - 1)).is_none() {
                bail!("the requested seed range exceeds u64::MAX");
            }
            let mut outputs = Vec::new();
            for offset in 0..count {
                let g = convert(genre.generate(
                    &params,
                    difficulty,
                    seed + u64::from(offset),
                    max_attempts,
                    &Budget::new(budget),
                ))?;
                let text = if solution {
                    g.text
                } else {
                    convert(genre.with_solution(&g.text, None))?
                };
                let input = Input {
                    genre,
                    text,
                    rating: Some(g.rating),
                };
                let answer = if solution {
                    convert(genre.solution(&input.text))?
                } else {
                    None
                };
                outputs.push(record_output(
                    &input,
                    format,
                    answer.as_deref(),
                    input.rating.as_ref(),
                )?);
            }
            print_all(outputs, format)?;
        }
        Command::Solve(args) => {
            let mut outputs = Vec::new();
            let mut success = true;
            for mut input in inputs(&args, selected)? {
                match convert(input.genre.solve(&input.text, &Budget::new(args.budget)))? {
                    Solutions::None => {
                        success = false;
                        eprintln!("{}: no solution", convert(input.genre.id(&input.text))?);
                        input.text = convert(input.genre.with_solution(&input.text, None))?;
                        outputs.push(record_output(
                            &input,
                            args.format,
                            None,
                            input.rating.as_ref(),
                        )?);
                    }
                    Solutions::Unique(a) => {
                        input.text = convert(input.genre.with_solution(&input.text, Some(&a)))?;
                        outputs.push(record_output(
                            &input,
                            args.format,
                            Some(&a),
                            input.rating.as_ref(),
                        )?);
                    }
                    Solutions::Multiple(a, b) => {
                        eprintln!(
                            "{}: multiple solutions (two witnesses follow)",
                            convert(input.genre.id(&input.text))?
                        );
                        for answer in [a, b] {
                            input.text =
                                convert(input.genre.with_solution(&input.text, Some(&answer)))?;
                            outputs.push(record_output(
                                &input,
                                args.format,
                                Some(&answer),
                                input.rating.as_ref(),
                            )?);
                        }
                    }
                }
            }
            print_all(outputs, args.format)?;
            return Ok(success);
        }
        Command::Rate(args) => {
            let mut outputs = Vec::new();
            for mut input in inputs(&args, selected)? {
                let rating = convert(input.genre.rate(&input.text, &Budget::new(args.budget)))?;
                input.text = convert(input.genre.with_rating(&input.text, &rating))?;
                outputs.push(record_output(&input, args.format, None, Some(&rating))?);
            }
            print_all(outputs, args.format)?;
        }
        Command::Check {
            input: args,
            answer,
            finished,
        } => {
            let answer = answer.map(fs::read_to_string).transpose()?;
            let mut outputs = Vec::new();
            let mut success = true;
            for input in inputs(&args, selected)? {
                let violations = convert(input.genre.check(
                    &input.text,
                    answer.as_deref(),
                    finished,
                    &Budget::new(args.budget),
                ))?;
                success &= violations.is_empty();
                if args.format == Format::Json {
                    outputs.push(serde_json::to_string(&serde_json::json!({"genre":input.genre.metadata().id,"violations":violations}))?);
                } else {
                    let mut out = format!(
                        "{}: {}",
                        convert(input.genre.id(&input.text))?,
                        if violations.is_empty() {
                            "no violations"
                        } else {
                            "violations"
                        }
                    );
                    for v in violations {
                        out.push_str(&format!("\nR{} at {}: {:?}", v.rule, v.place, v.values));
                    }
                    outputs.push(out);
                }
            }
            print_all(outputs, args.format)?;
            return Ok(success);
        }
        Command::Hint {
            input: args,
            answer,
        } => {
            let answer = answer.map(fs::read_to_string).transpose()?;
            let mut outputs = Vec::new();
            for input in inputs(&args, selected)? {
                let hint = convert(input.genre.hint(
                    &input.text,
                    answer.as_deref(),
                    &Budget::new(args.budget),
                ))?;
                if args.format == Format::Json {
                    outputs.push(serde_json::to_string(
                        &serde_json::json!({"genre":input.genre.metadata().id,"hint":hint}),
                    )?);
                } else {
                    outputs.push(format!(
                        "{}\n{}",
                        convert(input.genre.id(&input.text))?,
                        hint_text(&hint)
                    ));
                }
            }
            print_all(outputs, args.format)?;
        }
        Command::Completions { shell } => {
            generate(
                shell,
                &mut Cli::command(),
                "pencil-puzzles",
                &mut io::stdout(),
            );
        }
    }
    Ok(true)
}
fn hint_text(hint: &pencil_puzzles::Hint) -> String {
    use pencil_puzzles::Hint;
    match hint {
        Hint::Solved => "The answer is solved.".into(),
        Hint::Step(step) => {
            let mut text = format!("{} ({})", step.technique, step.level);
            for removed in &step.removed {
                text.push_str(&format!(
                    "
Remove {} from {}.",
                    removed.value, removed.place
                ));
            }
            text
        }
        _ => "No display is available for this hint.".into(),
    }
}
fn main() -> ExitCode {
    match run(Cli::parse(), None) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
