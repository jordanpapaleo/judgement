//! Field validation shared by every write path. Each function also runs the
//! unsafe-content check, so a caller can't validate without scanning.

use crate::error::{Error, Result};
use crate::model::{ItemDraft, LearningKind, LearningPayload, Phase, Proposal};
use crate::safety;

pub const TITLE_MAX: usize = 120;
pub const SUMMARY_MAX: usize = 200;
pub const GUIDANCE_MAX: usize = 4000;
pub const CONTEXT_MAX: usize = 1000;
pub const TAG_MAX: usize = 40;
pub const TAGS_MAX: usize = 20;
pub const LABEL_MAX: usize = 64;
/// Bytes per evidence entry and in total on an item (§5).
pub const EVIDENCE_ENTRY_MAX: usize = 1024;
pub const EVIDENCE_TOTAL_MAX: usize = 4096;
pub const GET_MAX_IDS: usize = 10;

fn text(field: &str, value: &str, max: usize, required: bool) -> Result<()> {
    if required && value.trim().is_empty() {
        return Err(Error::invalid(format!("{field} is required")));
    }
    if value.chars().count() > max {
        return Err(Error::invalid(format!(
            "{field} is longer than {max} characters"
        )));
    }
    safety::check(field, value)
}

fn one_line(field: &str, value: &str) -> Result<()> {
    if value.contains('\n') {
        return Err(Error::invalid(format!("{field} must be one line")));
    }
    Ok(())
}

/// Trims tags, drops empties and duplicates, and checks limits.
pub fn tags(values: &[String]) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for raw in values {
        let tag = raw.trim().to_lowercase();
        if tag.is_empty() || out.contains(&tag) {
            continue;
        }
        if tag.chars().count() > TAG_MAX || tag.contains(char::is_whitespace) {
            return Err(Error::invalid(format!(
                "tag \"{tag}\" must be one word of at most {TAG_MAX} characters"
            )));
        }
        safety::check("tags", &tag)?;
        out.push(tag);
    }
    if out.len() > TAGS_MAX {
        return Err(Error::invalid(format!("at most {TAGS_MAX} tags")));
    }
    Ok(out)
}

/// Dedupes phases into canonical order.
pub fn phases(values: &[Phase]) -> Vec<Phase> {
    Phase::ALL
        .iter()
        .copied()
        .filter(|p| values.contains(p))
        .collect()
}

/// Harness/model labels: free text, length-limited, unsafe-checked.
pub fn label(field: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok("unknown".into());
    }
    if value.chars().count() > LABEL_MAX || value.contains(char::is_control) {
        return Err(Error::invalid(format!(
            "{field} must be one line of at most {LABEL_MAX} characters"
        )));
    }
    safety::check(field, value)?;
    Ok(value.to_string())
}

pub fn evidence_entry(field: &str, value: &str) -> Result<()> {
    if value.len() > EVIDENCE_ENTRY_MAX {
        return Err(Error::invalid(format!(
            "{field} is {} bytes; describe it in at most {EVIDENCE_ENTRY_MAX} bytes",
            value.len()
        )));
    }
    safety::check(field, value)
}

pub fn evidence_list(entries: &[String]) -> Result<Vec<String>> {
    let entries: Vec<String> = entries
        .iter()
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .collect();
    for e in &entries {
        evidence_entry("evidence", e)?;
    }
    let total: usize = entries.iter().map(String::len).sum();
    if total > EVIDENCE_TOTAL_MAX {
        return Err(Error::invalid(format!(
            "evidence totals {total} bytes; the limit is {EVIDENCE_TOTAL_MAX}"
        )));
    }
    Ok(entries)
}

/// Appends `entry`, dropping the oldest entries until the total fits the cap.
pub fn append_evidence(existing: &[String], entry: &str) -> Vec<String> {
    let entry = entry.trim();
    let mut out = existing.to_vec();
    if entry.is_empty() {
        return out;
    }
    out.push(entry.to_string());
    while out.iter().map(String::len).sum::<usize>() > EVIDENCE_TOTAL_MAX && out.len() > 1 {
        out.remove(0);
    }
    out
}

/// Validates and normalizes an item draft (edit, combine, confirm-with-edit).
pub fn draft(d: &ItemDraft) -> Result<ItemDraft> {
    text("title", &d.title, TITLE_MAX, true)?;
    one_line("title", &d.title)?;
    text("summary", &d.summary, SUMMARY_MAX, true)?;
    one_line("summary", &d.summary)?;
    text("guidance", &d.guidance, GUIDANCE_MAX, true)?;
    text("applies_when", &d.applies_when, CONTEXT_MAX, false)?;
    text("exceptions", &d.exceptions, CONTEXT_MAX, false)?;
    if let Some(root) = &d.repo_root {
        text("repo_root", root, 4096, true)?;
    }
    Ok(ItemDraft {
        title: d.title.trim().to_string(),
        summary: d.summary.trim().to_string(),
        guidance: d.guidance.trim().to_string(),
        applies_when: d.applies_when.trim().to_string(),
        exceptions: d.exceptions.trim().to_string(),
        tags: tags(&d.tags)?,
        phases: phases(&d.phases),
        repo_root: d.repo_root.clone(),
        evidence: evidence_list(&d.evidence)?,
    })
}

/// Validates a payload's set fields without requiring any of them.
pub fn payload_fields(p: &LearningPayload) -> Result<LearningPayload> {
    let opt = |field: &str, v: &Option<String>, max: usize, line: bool| -> Result<Option<String>> {
        match v {
            None => Ok(None),
            Some(v) => {
                text(field, v, max, line)?;
                if line {
                    one_line(field, v)?;
                }
                Ok(Some(v.trim().to_string()))
            }
        }
    };
    Ok(LearningPayload {
        title: opt("title", &p.title, TITLE_MAX, true)?,
        summary: opt("summary", &p.summary, SUMMARY_MAX, true)?,
        guidance: opt("guidance", &p.guidance, GUIDANCE_MAX, false)?,
        applies_when: opt("applies_when", &p.applies_when, CONTEXT_MAX, false)?,
        exceptions: opt("exceptions", &p.exceptions, CONTEXT_MAX, false)?,
        tags: p.tags.as_deref().map(tags).transpose()?,
        phases: p.phases.as_deref().map(phases),
        repo_root: opt("repo_root", &p.repo_root, 4096, true)?,
    })
}

/// Validates a payload against the shape its kind requires.
pub fn payload_for(kind: LearningKind, p: &LearningPayload) -> Result<LearningPayload> {
    let p = payload_fields(p)?;
    match kind {
        LearningKind::New => {
            for (field, v) in [
                ("title", &p.title),
                ("summary", &p.summary),
                ("guidance", &p.guidance),
            ] {
                if v.as_deref().is_none_or(|v| v.trim().is_empty()) {
                    return Err(Error::invalid(format!(
                        "payload.{field} is required for a new learning"
                    )));
                }
            }
        }
        LearningKind::Correct => {
            if p.is_empty() {
                return Err(Error::invalid(
                    "a correction needs at least one changed field in payload",
                ));
            }
            if p.repo_root.is_some() {
                return Err(Error::invalid(
                    "a correction can't change repo_root; edit the item instead",
                ));
            }
        }
        LearningKind::Reinforce => {
            if !p.is_empty() {
                return Err(Error::invalid(
                    "a reinforcement carries no payload; put the reason in evidence",
                ));
            }
        }
    }
    Ok(p)
}

/// Validates a proposal as submitted by `propose`.
pub fn proposal(p: &Proposal) -> Result<Proposal> {
    match (p.kind, &p.target_id) {
        (LearningKind::New, Some(_)) => {
            return Err(Error::invalid(
                "target_id must be omitted for a new learning",
            ))
        }
        (LearningKind::Reinforce | LearningKind::Correct, None) => {
            return Err(Error::invalid(format!(
                "target_id is required for {}",
                p.kind
            )))
        }
        _ => {}
    }
    if let Some(id) = &p.target_id {
        if ulid::Ulid::from_string(id).is_err() {
            return Err(Error::invalid(format!(
                "target_id \"{id}\" is not an item ID"
            )));
        }
    }
    if p.evidence.trim().is_empty() {
        return Err(Error::invalid(
            "evidence is required: describe what happened, in prose",
        ));
    }
    evidence_entry("evidence", p.evidence.trim())?;
    if let Some(score) = p.signal_score {
        if !(1..=4).contains(&score) {
            return Err(Error::invalid("signal_score must be 1–4"));
        }
    }
    Ok(Proposal {
        kind: p.kind,
        target_id: p.target_id.clone(),
        payload: payload_for(p.kind, &p.payload)?,
        evidence: p.evidence.trim().to_string(),
        signal_score: p.signal_score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_payload() -> LearningPayload {
        LearningPayload {
            title: Some("Reuse first".into()),
            summary: Some("Look for an existing abstraction.".into()),
            guidance: Some("Search before adding a layer.".into()),
            ..Default::default()
        }
    }

    #[test]
    fn new_requires_core_fields() {
        let err = payload_for(LearningKind::New, &LearningPayload::default()).unwrap_err();
        assert!(err.to_string().contains("payload.title is required"));
        assert!(payload_for(LearningKind::New, &new_payload()).is_ok());
    }

    #[test]
    fn reinforce_rejects_payload_and_correct_requires_one() {
        assert!(payload_for(LearningKind::Reinforce, &new_payload()).is_err());
        assert!(payload_for(LearningKind::Correct, &LearningPayload::default()).is_err());
    }

    #[test]
    fn tags_are_normalized() {
        let out = tags(&[
            "Testing".into(),
            " testing ".into(),
            "".into(),
            "react".into(),
        ])
        .unwrap();
        assert_eq!(out, vec!["testing", "react"]);
        assert!(tags(&["two words".into()]).is_err());
    }

    #[test]
    fn evidence_caps() {
        let big = "x".repeat(EVIDENCE_ENTRY_MAX + 1);
        assert!(evidence_entry("evidence", &big).is_err());
        let full: Vec<String> = (0..4).map(|i| format!("{i}{}", "y".repeat(1023))).collect();
        let out = append_evidence(&full, "newest");
        assert_eq!(out.last().unwrap(), "newest");
        assert!(out.iter().map(String::len).sum::<usize>() <= EVIDENCE_TOTAL_MAX);
        assert!(out[0].starts_with('1'), "oldest entry rolled off");
    }

    #[test]
    fn summary_must_be_one_line() {
        let d = ItemDraft {
            title: "t".into(),
            summary: "a\nb".into(),
            guidance: "g".into(),
            ..Default::default()
        };
        assert!(draft(&d)
            .unwrap_err()
            .to_string()
            .contains("summary must be one line"));
    }

    #[test]
    fn labels_default_to_unknown() {
        assert_eq!(label("model", "  ").unwrap(), "unknown");
        assert!(label("model", &"m".repeat(LABEL_MAX + 1)).is_err());
    }
}
