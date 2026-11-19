//! Inception mode (auto-learn, §10). The trigger is harness-specific (Claude
//! Code `SessionEnd` first); the analyzer is whatever headless command the user
//! configured, run with a provider-neutral prompt. The analyzer prints JSON;
//! every learning it reports goes through `ops::propose`, so it passes the same
//! validation, unsafe check, pause, and temperature gate as any other write.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{AutoLearn, Item, Learning, Proposal};

/// Set for the headless analyzer run; a hook that sees it exits immediately.
pub const ANALYZER_ENV: &str = "JUDGEMENT_ANALYZER";
pub const DENIALS_SHOWN: u32 = 20;
const PROMPT_TEMPLATE: &str = include_str!("../prompts/analyzer.md");
/// Transcripts larger than this are only scanned in their last part for item IDs.
const TRANSCRIPT_SCAN_MAX: u64 = 32 * 1024 * 1024;
pub const LOG_FILE: &str = "inception.log";
const LOG_MAX: u64 = 1024 * 1024;

/// Claude Code `SessionEnd` hook input (other fields are ignored).
#[derive(Debug, Clone, Deserialize)]
pub struct SessionEndInput {
    pub session_id: Option<String>,
    pub transcript_path: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub model: Option<String>,
}

/// Everything the detached analyzer run needs, written to a private job file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub harness: String,
    pub session_ref: Option<String>,
    pub session_model: Option<String>,
    pub cwd: Option<PathBuf>,
    pub prompt: String,
}

static ITEMS_USED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"judgement\s+get((?:\s+[0-9A-HJKMNP-TV-Z]{26})+)").expect("valid pattern")
});
static ULID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[0-9A-HJKMNP-TV-Z]{26}").expect("valid pattern"));
static MODEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""model"\s*:\s*"([A-Za-z0-9._:/\[\]-]{3,64})""#).expect("valid pattern")
});

/// Item IDs the session pulled via `judgement get`, in first-seen order.
pub fn items_used(transcript: &str) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for call in ITEMS_USED.captures_iter(transcript) {
        for id in ULID.find_iter(&call[1]) {
            if !ids.iter().any(|i| i == id.as_str()) {
                ids.push(id.as_str().to_string());
            }
        }
    }
    ids
}

/// The last model label recorded in a Claude Code transcript, if any.
pub fn session_model(transcript: &str) -> Option<String> {
    MODEL
        .captures_iter(transcript)
        .last()
        .map(|c| c[1].to_string())
}

pub fn read_transcript(path: &Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = fs::File::open(path) else {
        return String::new();
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    if len > TRANSCRIPT_SCAN_MAX {
        let _ = file.seek(SeekFrom::Start(len - TRANSCRIPT_SCAN_MAX));
    }
    let mut bytes = Vec::new();
    let _ = file.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn min_score_text(temperature: AutoLearn) -> String {
    match temperature.min_score() {
        Some(4) => "4".into(),
        Some(n) => format!("{n} or higher"),
        None => "none".into(),
    }
}

/// Fills the analyzer prompt. Item and denial text is the user's own approved
/// or reviewed text, already unsafe-checked when it was stored.
pub fn build_prompt(
    temperature: AutoLearn,
    transcript_path: &Path,
    items: &[Item],
    denials: &[Learning],
) -> String {
    let items_text = if items.is_empty() {
        "(none)".to_string()
    } else {
        items
            .iter()
            .map(|i| {
                format!(
                    "- {} — {}: {}\n  Guidance: {}",
                    i.id, i.title, i.summary, i.guidance
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let denials_text = if denials.is_empty() {
        "(none)".to_string()
    } else {
        denials
            .iter()
            .map(|d| {
                let what = d
                    .payload
                    .title
                    .clone()
                    .or_else(|| d.payload.summary.clone())
                    .unwrap_or_else(|| {
                        format!("{} of {}", d.kind, d.target_id.as_deref().unwrap_or("?"))
                    });
                format!("- {} ({}): {}", what, d.kind, d.evidence)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    PROMPT_TEMPLATE
        .replace("{{temperature}}", temperature.as_str())
        .replace("{{min_score}}", &min_score_text(temperature))
        .replace(
            "{{transcript_path}}",
            &transcript_path.display().to_string(),
        )
        .replace("{{items_used}}", &items_text)
        .replace("{{recent_denials}}", &denials_text)
}

/// Extracts the analyzer's proposals from its stdout: the last JSON array in
/// the output (fenced or bare). No array means nothing was learned.
pub fn parse_output(stdout: &str) -> Result<Vec<Proposal>> {
    let trimmed = stdout.trim();
    let candidate = match (trimmed.rfind("```json"), trimmed.rfind('[')) {
        (Some(fence), _) => {
            let body = &trimmed[fence + 7..];
            body.split("```").next().unwrap_or(body).trim()
        }
        (None, Some(_)) => {
            let start = trimmed.find('[').expect("checked");
            let end = trimmed.rfind(']').map(|e| e + 1).unwrap_or(trimmed.len());
            &trimmed[start..end]
        }
        (None, None) => return Ok(Vec::new()),
    };
    serde_json::from_str(candidate).map_err(|e| {
        Error::invalid(format!(
            "analyzer output isn't a JSON array of learnings ({e})"
        ))
    })
}

pub fn write_job(dir: &Path, job: &Job) -> Result<PathBuf> {
    let jobs = dir.join("jobs");
    fs::create_dir_all(&jobs)?;
    let path = jobs.join(format!("{}.json", crate::new_id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(serde_json::to_string(job).expect("serializable").as_bytes())?;
    Ok(path)
}

/// Reads and removes a job file, so a job runs at most once.
pub fn take_job(path: &Path) -> Result<Job> {
    let text = fs::read_to_string(path)?;
    let _ = fs::remove_file(path);
    serde_json::from_str(&text).map_err(|e| Error::invalid(format!("bad job file ({e})")))
}

/// Opens the inception log for appending (0600), starting over past 1 MB.
pub fn open_log(dir: &Path) -> Result<fs::File> {
    let path = dir.join(LOG_FILE);
    if fs::metadata(&path).is_ok_and(|m| m.len() > LOG_MAX) {
        let _ = fs::remove_file(&path);
    }
    Ok(OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LearningKind;

    #[test]
    fn finds_items_used_and_session_model() {
        let transcript = r#"
{"type":"assistant","message":{"model":"claude-opus-5-5","content":"judgement get 01JABCDEFGHJKMNPQRSTVWXYZ0 01JABCDEFGHJKMNPQRSTVWXYZ1 --json"}}
{"type":"assistant","message":{"model":"claude-sonnet-5-5","content":"judgement get 01JABCDEFGHJKMNPQRSTVWXYZ0"}}
"#;
        assert_eq!(
            items_used(transcript),
            vec!["01JABCDEFGHJKMNPQRSTVWXYZ0", "01JABCDEFGHJKMNPQRSTVWXYZ1"]
        );
        assert_eq!(
            session_model(transcript).as_deref(),
            Some("claude-sonnet-5-5")
        );
    }

    #[test]
    fn parses_bare_and_fenced_output() {
        let bare = r#"[{"kind":"new","payload":{"title":"t","summary":"s","guidance":"g"},"evidence":"e","signal_score":4}]"#;
        assert_eq!(parse_output(bare).unwrap()[0].kind, LearningKind::New);
        let fenced = format!("Here you go:\n```json\n{bare}\n```\n");
        assert_eq!(parse_output(&fenced).unwrap().len(), 1);
        assert!(parse_output("Nothing learned.").unwrap().is_empty());
        assert!(parse_output("[]").unwrap().is_empty());
        assert!(parse_output("[{\"bogus\":1}]").is_err());
    }

    #[test]
    fn prompt_carries_temperature_and_context() {
        let prompt = build_prompt(AutoLearn::Warm, Path::new("/tmp/t.jsonl"), &[], &[]);
        assert!(prompt.contains("/tmp/t.jsonl"));
        assert!(prompt.contains("warm"));
        assert!(prompt.contains("3 or higher"));
        assert!(!prompt.contains("{{"));
    }
}
