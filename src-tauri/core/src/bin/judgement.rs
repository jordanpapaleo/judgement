//! `judgement` — the agent-facing CLI. On demand, exits after each call, never
//! migrates the schema. Output is always the JSON envelope; consumers read
//! `result.kind`.

use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use judgement_core::inception::{self, Job, SessionEndInput, ANALYZER_ENV};
use judgement_core::model::{Phase, Proposal, Provenance, Recall, Source};
use judgement_core::{ops, repo, store, Error};
use serde_json::{json, Map, Value};

const API_VERSION: u32 = 1;
const STDIN_MAX: u64 = 64 * 1024;
const ANALYZER_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const CONTINUE: &str = "Continue without Judgement guidance.";
/// Shipped inside the binary so the guide and skill always match this CLI.
const GUIDE: &str = include_str!("../../../../docs/setup-guide.md");
const SKILL: &str = include_str!("../../../../skill/judgement/SKILL.md");

#[derive(Parser)]
#[command(
    name = "judgement",
    version,
    about = "Recall and propose engineering guidance stored by Judgement"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Participation, inception mode, and counts.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// The index of active items for a phase: id, title, summary, tags, phases, scope.
    List {
        #[arg(long, value_parser = parse_phase)]
        phase: Phase,
        /// Repository path (defaults to the current directory's repo, if any).
        #[arg(long)]
        repo: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Full items by ID (up to 10). Bumps their usage counters.
    Get {
        #[arg(required = true, num_args = 1..)]
        ids: Vec<String>,
        #[arg(long)]
        repo: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Submit a learning (JSON on stdin) for the user to review in Judgement.
    Propose {
        #[arg(long, required = true)]
        stdin: bool,
        #[arg(long, default_value = "learn", value_parser = parse_source)]
        source: Source,
        /// Harness label (falls back to JUDGEMENT_HARNESS, then "unknown").
        #[arg(long)]
        harness: Option<String>,
        /// Model that made the observation (falls back to JUDGEMENT_MODEL, then "unknown").
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        session_model: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Print the setup guide (Markdown). Hand it to any AI to set Judgement up.
    Guide,
    /// Print the agent skill (SKILL.md) that matches this version.
    Skill,
    /// Harness hooks for inception mode.
    #[command(hide = true)]
    Hook {
        #[command(subcommand)]
        hook: HookCmd,
    },
}

#[derive(Subcommand)]
enum HookCmd {
    /// Claude Code SessionEnd hook: reads hook JSON on stdin, starts the analyzer detached.
    SessionEnd,
    /// Runs one analyzer job (started by session-end; not for direct use).
    Analyze { job: PathBuf },
}

fn parse_phase(s: &str) -> Result<Phase, String> {
    Phase::parse(s).ok_or_else(|| "expected planning, coding, or review".into())
}

fn parse_source(s: &str) -> Result<Source, String> {
    Source::parse(s).ok_or_else(|| "expected learn, manual, or auto".into())
}

/// Prints `{ "apiVersion": 1, "result": { "kind": ..., ...fields } }` and maps the exit code.
fn emit(kind: &str, fields: Value, code: u8) -> ExitCode {
    let mut result = Map::new();
    result.insert("kind".into(), json!(kind));
    if let Value::Object(fields) = fields {
        result.extend(fields);
    }
    let envelope = json!({ "apiVersion": API_VERSION, "result": result });
    println!(
        "{}",
        serde_json::to_string_pretty(&envelope).expect("serializable")
    );
    ExitCode::from(code)
}

fn emit_error(err: Error) -> ExitCode {
    match err {
        Error::Paused => emit(
            "paused",
            json!({ "message": "Judgement is paused; nothing was submitted. Continue without it." }),
            4,
        ),
        Error::Unsafe { ref field, class } => emit(
            "unsafe",
            json!({ "field": field, "pattern": class, "message": err.to_string() }),
            2,
        ),
        Error::Invalid(_) | Error::NotFound(_) | Error::Conflict(_) => {
            emit("invalid", json!({ "message": err.to_string() }), 2)
        }
        ref e if e.is_store_unavailable() => emit(
            "unavailable",
            json!({ "message": format!("{e}. {CONTINUE}") }),
            3,
        ),
        e => emit("error", json!({ "message": format!("{e}. {CONTINUE}") }), 1),
    }
}

fn paused_read() -> ExitCode {
    emit(
        "paused",
        json!({ "message": format!("Judgement is paused. {CONTINUE}") }),
        0,
    )
}

fn resolve_repo(flag: Option<&Path>) -> Result<Option<String>, Error> {
    match flag {
        Some(path) => repo::repo_root(path).map(Some).ok_or_else(|| {
            Error::invalid(format!("{} is not inside a Git repository", path.display()))
        }),
        None => Ok(std::env::current_dir()
            .ok()
            .and_then(|d| repo::repo_root(&d))),
    }
}

fn label(flag: Option<String>, env: &str) -> String {
    flag.or_else(|| std::env::var(env).ok())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn open() -> Result<rusqlite::Connection, Error> {
    store::open_for_cli(&store::default_data_dir()?)
}

fn run(cmd: Cmd) -> Result<ExitCode, Error> {
    match cmd {
        Cmd::Status { .. } => {
            let dir = store::default_data_dir()?;
            let mut conn = store::open_for_cli(&dir)?;
            let s = ops::status(&mut conn)?;
            let mut fields = serde_json::to_value(s).expect("serializable");
            fields["schema_version"] = json!(store::SCHEMA_VERSION);
            fields["store"] = json!(store::store_path(&dir));
            Ok(emit("ok", fields, 0))
        }
        Cmd::List { phase, repo, .. } => {
            let repo_root = resolve_repo(repo.as_deref())?;
            let mut conn = open()?;
            match ops::list_index(&mut conn, phase, repo_root.as_deref())? {
                Recall::Paused => Ok(paused_read()),
                Recall::Ok(items) => {
                    let bytes = serde_json::to_string(&items).expect("serializable").len();
                    Ok(emit(
                        "ok",
                        json!({ "phase": phase, "repo_root": repo_root, "items": items, "bytes": bytes }),
                        0,
                    ))
                }
            }
        }
        Cmd::Get { ids, repo, .. } => {
            let repo_root = resolve_repo(repo.as_deref())?;
            let mut conn = open()?;
            match ops::get_items(&mut conn, &ids, repo_root.as_deref())? {
                Recall::Paused => Ok(paused_read()),
                Recall::Ok(result) => Ok(emit(
                    "ok",
                    serde_json::to_value(result).expect("serializable"),
                    0,
                )),
            }
        }
        Cmd::Propose {
            source,
            harness,
            model,
            session_model,
            session,
            ..
        } => {
            let mut text = String::new();
            std::io::stdin()
                .take(STDIN_MAX + 1)
                .read_to_string(&mut text)?;
            if text.len() as u64 > STDIN_MAX {
                return Err(Error::invalid("proposal is larger than 64 KB"));
            }
            let mut proposal: Proposal = serde_json::from_str(&text)
                .map_err(|e| Error::invalid(format!("stdin isn't a valid learning: {e}")))?;
            if let Some(path) = proposal.payload.repo_root.take() {
                proposal.payload.repo_root = resolve_repo(Some(Path::new(&path)))?;
            }
            let prov = Provenance {
                source,
                harness: label(harness, "JUDGEMENT_HARNESS"),
                model: label(model, "JUDGEMENT_MODEL"),
                session_model,
                session_ref: session,
            };
            let mut conn = open()?;
            let learning = ops::propose(&mut conn, &proposal, &prov)?;
            Ok(emit(
                "ok",
                json!({
                    "id": learning.id,
                    "status": learning.status,
                    "message": "Submitted to Judgement. The user confirms or denies it under Learnings."
                }),
                0,
            ))
        }
        Cmd::Guide => {
            print!("{GUIDE}");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Skill => {
            print!("{SKILL}");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Hook {
            hook: HookCmd::SessionEnd,
        } => {
            session_end();
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Hook {
            hook: HookCmd::Analyze { job },
        } => {
            analyze(&job);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn log(line: impl AsRef<str>) {
    let Ok(dir) = store::default_data_dir() else {
        return;
    };
    if let Ok(mut file) = inception::open_log(&dir) {
        let _ = writeln!(file, "{} {}", judgement_core::now(), line.as_ref());
    }
}

/// Claude Code SessionEnd hook. Never blocks session exit and never fails it:
/// every problem is logged and the hook exits 0.
fn session_end() {
    if std::env::var_os(ANALYZER_ENV).is_some() {
        return;
    }
    let mut text = String::new();
    if std::io::stdin()
        .take(STDIN_MAX)
        .read_to_string(&mut text)
        .is_err()
    {
        return;
    }
    if let Err(e) = start_analyzer(&text) {
        log(format!("session-end: {e}"));
    }
}

fn start_analyzer(hook_input: &str) -> Result<(), Error> {
    let input: SessionEndInput = serde_json::from_str(hook_input)
        .map_err(|e| Error::invalid(format!("unexpected hook input ({e})")))?;
    let dir = store::default_data_dir()?;
    let mut conn = store::open_for_cli(&dir)?;
    let settings = ops::settings(&conn)?;
    if settings.participation == judgement_core::model::Participation::Paused
        || settings.auto_learn.min_score().is_none()
    {
        return Ok(());
    }
    let transcript_path = input
        .transcript_path
        .ok_or_else(|| Error::invalid("hook input has no transcript_path"))?;
    let transcript = inception::read_transcript(&transcript_path);
    let items: Vec<_> = inception::items_used(&transcript)
        .iter()
        .filter_map(|id| ops::get_item(&conn, id).ok().flatten())
        .collect();
    let denials = ops::recent_denials(&conn, inception::DENIALS_SHOWN)?;
    let status = ops::status(&mut conn)?;
    drop(conn);

    let job = Job {
        harness: "claude-code".into(),
        session_ref: input.session_id,
        session_model: input
            .model
            .or_else(|| inception::session_model(&transcript)),
        cwd: input.cwd,
        prompt: inception::build_prompt(status.auto_learn, &transcript_path, &items, &denials),
    };
    let job_path = inception::write_job(&dir, &job)?;
    let log_file = inception::open_log(&dir)?;
    Command::new(std::env::current_exe()?)
        .args(["hook", "analyze"])
        .arg(&job_path)
        .env(ANALYZER_ENV, "1")
        .stdin(Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file)
        .process_group(0)
        .spawn()?;
    Ok(())
}

/// The detached analyzer run: executes the configured argv (never through a
/// shell) with the prompt on stdin, then proposes whatever it reports.
fn analyze(job_path: &Path) {
    if let Err(e) = run_analyzer(job_path) {
        log(format!("analyze: {e}"));
    }
}

fn run_analyzer(job_path: &Path) -> Result<(), Error> {
    let job = inception::take_job(job_path)?;
    let dir = store::default_data_dir()?;
    let settings = ops::settings(&store::open_for_cli(&dir)?)?;
    let (program, args) = settings
        .analyzer
        .argv
        .split_first()
        .ok_or_else(|| Error::invalid("no analyzer command is configured"))?;

    let mut command = Command::new(program);
    command
        .args(args)
        .env(ANALYZER_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(inception::open_log(&dir)?));
    if let Some(cwd) = job.cwd.as_ref().filter(|c| c.is_dir()) {
        command.current_dir(cwd);
    }
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().expect("piped");
    let prompt = job.prompt.clone();
    std::thread::spawn(move || {
        let _ = stdin.write_all(prompt.as_bytes());
    });
    let mut stdout = child.stdout.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.read_to_string(&mut out);
        out
    });
    let deadline = Instant::now() + ANALYZER_TIMEOUT;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::invalid("analyzer timed out"));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let output = reader.join().unwrap_or_default();
    let proposals = inception::parse_output(&output)?;
    log(format!(
        "analyze: session {:?} produced {} candidate(s)",
        job.session_ref,
        proposals.len()
    ));

    let mut conn = store::open_for_cli(&dir)?;
    let prov = Provenance {
        source: Source::Auto,
        harness: job.harness,
        model: settings.analyzer.model,
        session_model: job.session_model,
        session_ref: job.session_ref,
    };
    for proposal in &proposals {
        match ops::propose(&mut conn, proposal, &prov) {
            Ok(l) => log(format!("analyze: proposed {} ({})", l.id, l.kind)),
            Err(e) => log(format!("analyze: skipped a candidate: {e}")),
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind;
            if matches!(
                e.kind(),
                ErrorKind::DisplayHelp
                    | ErrorKind::DisplayVersion
                    | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            ) {
                let _ = e.print();
                return ExitCode::SUCCESS;
            }
            return emit(
                "invalid",
                json!({ "message": e.render().to_string().trim() }),
                2,
            );
        }
    };
    match run(cli.command) {
        Ok(code) => code,
        Err(e) => emit_error(e),
    }
}
