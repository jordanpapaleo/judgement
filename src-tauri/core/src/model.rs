//! Store DTOs. Every type here is exported to TypeScript by ts-rs (`cargo test`
//! writes `src/bindings/*.ts`), so the UI and the store share one definition.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
        #[serde(rename_all = "lowercase")]
        #[ts(export)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }

            pub fn parse(text: &str) -> Option<Self> {
                match text { $($text => Some($name::$variant),)+ _ => None }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

text_enum!(
    /// Global switch: whether Judgement participates in agent work.
    Participation { Active => "active", Paused => "paused" }
);
text_enum!(
    /// Inception-mode temperature.
    AutoLearn { Off => "off", Cool => "cool", Warm => "warm", Hot => "hot" }
);
text_enum!(Phase { Planning => "planning", Coding => "coding", Review => "review" });
text_enum!(ItemStatus { Active => "active", Omitted => "omitted" });
text_enum!(
    /// Where a learning came from.
    Source { Manual => "manual", Learn => "learn", Auto => "auto" }
);
text_enum!(LearningKind { New => "new", Reinforce => "reinforce", Correct => "correct" });
text_enum!(LearningStatus { Pending => "pending", Confirmed => "confirmed", Denied => "denied" });

impl AutoLearn {
    /// Lowest signal score inception mode saves at this temperature (§10.3).
    pub fn min_score(self) -> Option<u8> {
        match self {
            AutoLearn::Off => None,
            AutoLearn::Cool => Some(4),
            AutoLearn::Warm => Some(3),
            AutoLearn::Hot => Some(1),
        }
    }
}

/// The headless command that runs the analyzer prompt, stored and executed as
/// an argument array (never through a shell), plus the model label it records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Analyzer {
    pub argv: Vec<String>,
    pub model: String,
}

impl Default for Analyzer {
    fn default() -> Self {
        Analyzer {
            argv: vec![
                "claude".into(),
                "-p".into(),
                "--allowedTools".into(),
                "Read".into(),
            ],
            model: "unknown".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Settings {
    pub participation: Participation,
    pub auto_learn: AutoLearn,
    pub analyzer: Analyzer,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub guidance: String,
    pub applies_when: String,
    pub exceptions: String,
    pub tags: Vec<String>,
    /// Empty means all phases.
    pub phases: Vec<Phase>,
    /// Canonical Git common dir; `None` means personal (all repos).
    pub repo_root: Option<String>,
    pub status: ItemStatus,
    pub omitted_at: Option<String>,
    pub source: Source,
    pub source_harness: String,
    pub source_model: String,
    pub evidence: Vec<String>,
    pub times_returned: u32,
    pub last_returned_at: Option<String>,
    pub reinforced_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

/// The user-editable fields of an item: used by edit and combine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ItemDraft {
    pub title: String,
    pub summary: String,
    pub guidance: String,
    pub applies_when: String,
    pub exceptions: String,
    pub tags: Vec<String>,
    pub phases: Vec<Phase>,
    pub repo_root: Option<String>,
    pub evidence: Vec<String>,
}

/// What a learning proposes. For `new`, title/summary/guidance are required;
/// for `correct`, only the changed fields are set; `reinforce` carries none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default, deny_unknown_fields)]
#[ts(export)]
pub struct LearningPayload {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub guidance: Option<String>,
    pub applies_when: Option<String>,
    pub exceptions: Option<String>,
    pub tags: Option<Vec<String>>,
    pub phases: Option<Vec<Phase>>,
    /// Only meaningful for `new`. The CLI resolves a path to its canonical Git
    /// common dir before storing.
    pub repo_root: Option<String>,
}

impl LearningPayload {
    pub fn is_empty(&self) -> bool {
        *self == LearningPayload::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Learning {
    pub id: String,
    pub source: Source,
    pub kind: LearningKind,
    pub target_id: Option<String>,
    pub payload: LearningPayload,
    pub evidence: String,
    pub harness: String,
    pub model: String,
    pub session_model: Option<String>,
    pub session_ref: Option<String>,
    pub signal_score: Option<u8>,
    pub temperature: Option<AutoLearn>,
    pub status: LearningStatus,
    pub created_at: String,
    pub decided_at: Option<String>,
}

/// Who produced a learning. Labels are free text and never affect recall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub source: Source,
    pub harness: String,
    pub model: String,
    pub session_model: Option<String>,
    pub session_ref: Option<String>,
}

/// A candidate learning as submitted by `propose` (JSON on stdin).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct Proposal {
    pub kind: LearningKind,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub payload: LearningPayload,
    #[serde(default)]
    pub evidence: String,
    /// Auto-learn only (§10.2): 1–4.
    #[serde(default)]
    pub signal_score: Option<u8>,
}

/// One row of the `list` index: what agents see before choosing what to `get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IndexEntry {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub phases: Vec<Phase>,
    /// `personal` or `repo`.
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NotReturnedReason {
    Unknown,
    Omitted,
    OutOfScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NotReturned {
    pub id: String,
    pub reason: NotReturnedReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetResult {
    pub items: Vec<Item>,
    pub not_returned: Vec<NotReturned>,
}

/// Outcome of an agent-facing read: either results, or "paused" with nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recall<T> {
    Ok(T),
    Paused,
}
