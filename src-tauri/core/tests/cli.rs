//! The real `judgement` binary against a temp store: envelopes, exit codes,
//! schema mismatch, app-closed operation, missing store (§7, A8).

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use judgement_core::model::{LearningStatus, Participation};
use judgement_core::{ops, store};
use serde_json::Value;

struct Run {
    code: i32,
    json: Value,
    elapsed: Duration,
}

fn judgement(home: &Path, cwd: &Path, args: &[&str], stdin: Option<&str>) -> Run {
    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_judgement"))
        .args(args)
        .current_dir(cwd)
        .env("JUDGEMENT_HOME", home)
        .env_remove("JUDGEMENT_HARNESS")
        .env_remove("JUDGEMENT_MODEL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.unwrap_or("").as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap_or_else(|_| panic!("not JSON: {text}"));
    assert_eq!(json["apiVersion"], 1);
    Run {
        code: out.status.code().unwrap(),
        json,
        elapsed: started.elapsed(),
    }
}

/// A store created the way the app creates it, then closed (app not running).
fn app_store(active: bool) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let mut conn = store::open_for_app(tmp.path()).unwrap().conn;
    if active {
        ops::set_participation(&mut conn, Participation::Active).unwrap();
    }
    tmp
}

const NEW: &str = r#"{"kind":"new","payload":{"title":"Reuse first","summary":"Look for an existing abstraction first.","guidance":"Search before adding a layer.","phases":["planning"]},"evidence":"The user rejected a duplicate migration layer."}"#;

#[test]
fn missing_store_is_unavailable() {
    let home = tempfile::tempdir().unwrap();
    let r = judgement(home.path(), home.path(), &["status", "--json"], None);
    assert_eq!(r.code, 3);
    assert_eq!(r.json["result"]["kind"], "unavailable");
    assert!(r.json["result"]["message"]
        .as_str()
        .unwrap()
        .contains("Continue without"));
}

#[test]
fn schema_mismatch_is_unavailable() {
    let home = app_store(true);
    let conn = rusqlite::Connection::open(store::store_path(home.path())).unwrap();
    conn.pragma_update(None, "user_version", 99).unwrap();
    let r = judgement(
        home.path(),
        home.path(),
        &["list", "--phase", "coding", "--json"],
        None,
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (3, Some("unavailable"))
    );
    assert!(r.json["result"]["message"]
        .as_str()
        .unwrap()
        .contains("open Judgement to upgrade"));
}

#[test]
fn first_launch_is_paused_for_agents() {
    let home = app_store(false);
    let r = judgement(
        home.path(),
        home.path(),
        &["list", "--phase", "coding", "--json"],
        None,
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (0, Some("paused"))
    );
    assert!(r.json["result"].get("items").is_none());

    let r = judgement(
        home.path(),
        home.path(),
        &["propose", "--stdin", "--json"],
        Some(NEW),
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (4, Some("paused"))
    );

    let r = judgement(
        home.path(),
        home.path(),
        &["get", "01JABCDEFGHJKMNPQRSTVWXYZ0", "--json"],
        None,
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (0, Some("paused"))
    );
}

#[test]
fn propose_confirm_list_get_round_trip_with_app_closed() {
    let home = app_store(true);
    let r = judgement(
        home.path(),
        home.path(),
        &[
            "propose",
            "--stdin",
            "--harness",
            "codex",
            "--model",
            "gpt-6.1-sol",
            "--json",
        ],
        Some(NEW),
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (0, Some("ok")),
        "{}",
        r.json
    );
    assert!(r.elapsed < Duration::from_secs(3), "A8: {:?}", r.elapsed);
    let learning_id = r.json["result"]["id"].as_str().unwrap().to_string();

    let mut conn = store::open_for_cli(home.path()).unwrap();
    let learning = ops::get_learning(&conn, &learning_id).unwrap().unwrap();
    assert_eq!(learning.status, LearningStatus::Pending);
    assert_eq!(
        (learning.harness.as_str(), learning.model.as_str()),
        ("codex", "gpt-6.1-sol")
    );
    let item = ops::confirm_learning(&mut conn, &learning_id, None).unwrap();
    drop(conn);

    let r = judgement(
        home.path(),
        home.path(),
        &["list", "--phase", "planning", "--json"],
        None,
    );
    assert_eq!(r.code, 0);
    let items = r.json["result"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], item.id.as_str());
    assert_eq!(items[0]["scope"], "personal");
    assert!(
        items[0].get("guidance").is_none(),
        "list returns the index only"
    );
    assert!(r.json["result"]["bytes"].as_u64().unwrap() > 0);

    let r = judgement(
        home.path(),
        home.path(),
        &["list", "--phase", "review", "--json"],
        None,
    );
    assert!(r.json["result"]["items"].as_array().unwrap().is_empty());

    let r = judgement(home.path(), home.path(), &["get", &item.id, "--json"], None);
    assert_eq!(r.code, 0);
    assert_eq!(
        r.json["result"]["items"][0]["guidance"],
        "Search before adding a layer."
    );
    assert_eq!(r.json["result"]["items"][0]["times_returned"], 1);
}

#[test]
fn provenance_falls_back_to_env_then_unknown() {
    let home = app_store(true);
    let mut child = Command::new(env!("CARGO_BIN_EXE_judgement"))
        .args(["propose", "--stdin", "--json"])
        .current_dir(home.path())
        .env("JUDGEMENT_HOME", home.path())
        .env("JUDGEMENT_HARNESS", "pi")
        .env_remove("JUDGEMENT_MODEL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(NEW.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    let id = json["result"]["id"].as_str().unwrap();
    let conn = store::open_for_cli(home.path()).unwrap();
    let l = ops::get_learning(&conn, id).unwrap().unwrap();
    assert_eq!((l.harness.as_str(), l.model.as_str()), ("pi", "unknown"));
}

#[test]
fn invalid_and_unsafe_input_exit_2() {
    let home = app_store(true);
    let r = judgement(
        home.path(),
        home.path(),
        &["propose", "--stdin", "--json"],
        Some("not json"),
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (2, Some("invalid"))
    );

    let leaky = NEW.replace(
        "The user rejected",
        "AKIAIOSFODNN7EXAMPLE showed the user rejected",
    );
    let r = judgement(
        home.path(),
        home.path(),
        &["propose", "--stdin", "--json"],
        Some(&leaky),
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (2, Some("unsafe"))
    );
    assert_eq!(r.json["result"]["field"], "evidence");

    let r = judgement(
        home.path(),
        home.path(),
        &["list", "--phase", "deploy", "--json"],
        None,
    );
    assert_eq!(
        (r.code, r.json["result"]["kind"].as_str()),
        (2, Some("invalid"))
    );

    let conn = store::open_for_cli(home.path()).unwrap();
    assert!(
        ops::list_learnings(&conn, None).unwrap().is_empty(),
        "nothing partial stored"
    );
}

#[test]
fn propose_resolves_repo_root_to_git_common_dir() {
    let home = app_store(true);
    let repo = tempfile::tempdir().unwrap();
    let ok = Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo.path())
        .status()
        .unwrap();
    assert!(ok.success());
    let with_repo = NEW.replace(
        r#""phases":["planning"]"#,
        r##""phases":["planning"],"repo_root":".""##,
    );
    let r = judgement(
        home.path(),
        repo.path(),
        &["propose", "--stdin", "--json"],
        Some(&with_repo),
    );
    assert_eq!(r.code, 0, "{}", r.json);
    let conn = store::open_for_cli(home.path()).unwrap();
    let l = ops::get_learning(&conn, r.json["result"]["id"].as_str().unwrap())
        .unwrap()
        .unwrap();
    let expected = repo.path().join(".git").canonicalize().unwrap();
    assert_eq!(
        l.payload.repo_root.as_deref(),
        Some(expected.to_str().unwrap())
    );
}

#[test]
fn status_reports_counts() {
    let home = app_store(true);
    let r = judgement(home.path(), home.path(), &["status", "--json"], None);
    assert_eq!(r.code, 0);
    assert_eq!(r.json["result"]["participation"], "active");
    assert_eq!(r.json["result"]["pending"], 0);
    assert_eq!(r.json["result"]["schema_version"], store::SCHEMA_VERSION);
}

#[test]
fn session_end_hook_exits_quietly_when_off_or_recursing() {
    let home = app_store(true);
    let input = r#"{"session_id":"abc","transcript_path":"/nonexistent","cwd":"/tmp"}"#;
    for analyzer_env in [false, true] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_judgement"));
        cmd.args(["hook", "session-end"])
            .env("JUDGEMENT_HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        if analyzer_env {
            cmd.env("JUDGEMENT_ANALYZER", "1");
        }
        let mut child = cmd.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        assert!(
            !home.path().join("jobs").exists(),
            "no analyzer job started"
        );
    }
}

#[test]
fn session_end_hook_runs_the_configured_analyzer_detached() {
    use judgement_core::model::{Analyzer, AutoLearn, Source};
    let home = app_store(true);
    {
        let mut conn = store::open_for_cli(home.path()).unwrap();
        ops::set_auto_learn(&mut conn, AutoLearn::Warm).unwrap();
        // A stand-in analyzer: swallows the prompt and reports one score-4 and one score-2 learning.
        let output = r#"[{"kind":"new","signal_score":4,"payload":{"title":"Simpler","summary":"Prefer the simpler fix.","guidance":"Choose the simpler fix first."},"evidence":"The user said no, simpler."},{"kind":"new","signal_score":2,"payload":{"title":"Weak","summary":"Weak.","guidance":"Weak."},"evidence":"A passing reaction."}]"#;
        let script = format!("cat > /dev/null; printf '%s' '{output}'");
        let analyzer = Analyzer {
            argv: vec!["/bin/sh".into(), "-c".into(), script],
            model: "gpt-6.1-sol".into(),
        };
        ops::set_analyzer(&mut conn, &analyzer).unwrap();
    }
    let transcript = home.path().join("t.jsonl");
    std::fs::write(&transcript, r#"{"message":{"model":"claude-opus-5-5"}}"#).unwrap();
    let input = serde_json::json!({ "session_id": "sess-1", "transcript_path": transcript, "cwd": home.path() });

    let mut child = Command::new(env!("CARGO_BIN_EXE_judgement"))
        .args(["hook", "session-end"])
        .env("JUDGEMENT_HOME", home.path())
        .env_remove("JUDGEMENT_ANALYZER")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());

    let deadline = Instant::now() + Duration::from_secs(10);
    let learnings = loop {
        let conn = store::open_for_cli(home.path()).unwrap();
        let found = ops::list_learnings(&conn, None).unwrap();
        if !found.is_empty() || Instant::now() > deadline {
            break found;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(learnings.len(), 1, "only the score that warm saves");
    let l = &learnings[0];
    assert_eq!(l.source, Source::Auto);
    assert_eq!(
        (l.harness.as_str(), l.model.as_str()),
        ("claude-code", "gpt-6.1-sol")
    );
    assert_eq!(l.session_model.as_deref(), Some("claude-opus-5-5"));
    assert_eq!(l.session_ref.as_deref(), Some("sess-1"));
    assert_eq!(
        (l.signal_score, l.temperature),
        (Some(4), Some(AutoLearn::Warm))
    );
}

#[test]
fn guide_and_skill_print_without_a_store() {
    let home = tempfile::tempdir().unwrap();
    for (cmd, expected) in [
        ("guide", "# Judgement Setup Guide"),
        ("skill", "name: judgement"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_judgement"))
            .arg(cmd)
            .env("JUDGEMENT_HOME", home.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "{cmd}");
        assert!(
            String::from_utf8(out.stdout).unwrap().contains(expected),
            "{cmd}"
        );
    }
}
