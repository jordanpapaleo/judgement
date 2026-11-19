//! Every read and write the app and CLI perform. Writes run in
//! `BEGIN IMMEDIATE`; agent-facing operations read `settings` inside the same
//! transaction as their work, so Pause is linearizable across processes.

use std::collections::{BTreeMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction, TransactionBehavior};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::model::*;
use crate::{new_id, now, validate};

fn write_tx(conn: &mut Connection) -> Result<Transaction<'_>> {
    Ok(conn.transaction_with_behavior(TransactionBehavior::Immediate)?)
}

fn conversion_error(index: usize, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, message.into())
}

fn text_col<T>(row: &Row, index: usize, parse: fn(&str) -> Option<T>) -> rusqlite::Result<T> {
    let text: String = row.get(index)?;
    parse(&text).ok_or_else(|| conversion_error(index, format!("unexpected value {text:?}")))
}

fn json_col<T: DeserializeOwned>(row: &Row, index: usize) -> rusqlite::Result<T> {
    let text: String = row.get(index)?;
    serde_json::from_str(&text).map_err(|e| conversion_error(index, e.to_string()))
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("store types serialize")
}

// ---------------------------------------------------------------- settings

pub fn settings(conn: &Connection) -> Result<Settings> {
    Ok(conn.query_row(
        "SELECT participation, auto_learn, analyzer, updated_at FROM settings WHERE id = 1",
        [],
        |r| {
            Ok(Settings {
                participation: text_col(r, 0, Participation::parse)?,
                auto_learn: text_col(r, 1, AutoLearn::parse)?,
                analyzer: json_col(r, 2)?,
                updated_at: r.get(3)?,
            })
        },
    )?)
}

pub fn set_participation(conn: &mut Connection, value: Participation) -> Result<Settings> {
    let tx = write_tx(conn)?;
    tx.execute(
        "UPDATE settings SET participation = ?1, updated_at = ?2 WHERE id = 1",
        params![value.as_str(), now()],
    )?;
    let s = settings(&tx)?;
    tx.commit()?;
    Ok(s)
}

pub fn set_auto_learn(conn: &mut Connection, value: AutoLearn) -> Result<Settings> {
    let tx = write_tx(conn)?;
    tx.execute(
        "UPDATE settings SET auto_learn = ?1, updated_at = ?2 WHERE id = 1",
        params![value.as_str(), now()],
    )?;
    let s = settings(&tx)?;
    tx.commit()?;
    Ok(s)
}

pub fn set_analyzer(conn: &mut Connection, analyzer: &Analyzer) -> Result<Settings> {
    let argv: Vec<String> = analyzer.argv.iter().map(|a| a.trim().to_string()).collect();
    if argv.first().is_none_or(|a| a.is_empty()) {
        return Err(Error::invalid("analyzer command is required"));
    }
    for arg in &argv {
        crate::safety::check("analyzer command", arg)?;
    }
    let analyzer = Analyzer {
        argv,
        model: validate::label("analyzer model", &analyzer.model)?,
    };
    let tx = write_tx(conn)?;
    tx.execute(
        "UPDATE settings SET analyzer = ?1, updated_at = ?2 WHERE id = 1",
        params![to_json(&analyzer), now()],
    )?;
    let s = settings(&tx)?;
    tx.commit()?;
    Ok(s)
}

// ------------------------------------------------------------------- items

const ITEM_COLUMNS: &str = "id, title, summary, guidance, applies_when, exceptions, tags, phases, \
     repo_root, status, omitted_at, source, source_harness, source_model, evidence, \
     times_returned, last_returned_at, reinforced_count, created_at, updated_at";

fn item_from_row(r: &Row) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        title: r.get(1)?,
        summary: r.get(2)?,
        guidance: r.get(3)?,
        applies_when: r.get(4)?,
        exceptions: r.get(5)?,
        tags: json_col(r, 6)?,
        phases: json_col(r, 7)?,
        repo_root: r.get(8)?,
        status: text_col(r, 9, ItemStatus::parse)?,
        omitted_at: r.get(10)?,
        source: text_col(r, 11, Source::parse)?,
        source_harness: r.get(12)?,
        source_model: r.get(13)?,
        evidence: json_col(r, 14)?,
        times_returned: r.get(15)?,
        last_returned_at: r.get(16)?,
        reinforced_count: r.get(17)?,
        created_at: r.get(18)?,
        updated_at: r.get(19)?,
    })
}

pub fn get_item(conn: &Connection, id: &str) -> Result<Option<Item>> {
    Ok(conn
        .query_row(
            &format!("SELECT {ITEM_COLUMNS} FROM items WHERE id = ?1"),
            [id],
            item_from_row,
        )
        .optional()?)
}

fn require_item(conn: &Connection, id: &str) -> Result<Item> {
    get_item(conn, id)?.ok_or_else(|| Error::NotFound(format!("item {id} doesn't exist")))
}

/// All items, active and omitted, for the Knowledge screen.
pub fn list_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(&format!("SELECT {ITEM_COLUMNS} FROM items ORDER BY id"))?;
    let rows = stmt.query_map([], item_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

struct NewItem<'a> {
    draft: &'a ItemDraft,
    source: Source,
    harness: &'a str,
    model: &'a str,
}

fn insert_item(tx: &Transaction, new: NewItem) -> Result<Item> {
    let id = new_id();
    let at = now();
    let d = new.draft;
    tx.execute(
        "INSERT INTO items (id, title, summary, guidance, applies_when, exceptions, tags, phases,
             repo_root, status, source, source_harness, source_model, evidence, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'active', ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            id,
            d.title,
            d.summary,
            d.guidance,
            d.applies_when,
            d.exceptions,
            to_json(&d.tags),
            to_json(&d.phases),
            d.repo_root,
            new.source.as_str(),
            new.harness,
            new.model,
            to_json(&d.evidence),
            at,
        ],
    )?;
    require_item(tx, &id)
}

fn write_item_fields(tx: &Transaction, id: &str, d: &ItemDraft) -> Result<()> {
    tx.execute(
        "UPDATE items SET title = ?2, summary = ?3, guidance = ?4, applies_when = ?5, exceptions = ?6,
             tags = ?7, phases = ?8, repo_root = ?9, evidence = ?10, updated_at = ?11
         WHERE id = ?1",
        params![
            id,
            d.title,
            d.summary,
            d.guidance,
            d.applies_when,
            d.exceptions,
            to_json(&d.tags),
            to_json(&d.phases),
            d.repo_root,
            to_json(&d.evidence),
            now(),
        ],
    )?;
    Ok(())
}

fn draft_of(item: &Item) -> ItemDraft {
    ItemDraft {
        title: item.title.clone(),
        summary: item.summary.clone(),
        guidance: item.guidance.clone(),
        applies_when: item.applies_when.clone(),
        exceptions: item.exceptions.clone(),
        tags: item.tags.clone(),
        phases: item.phases.clone(),
        repo_root: item.repo_root.clone(),
        evidence: item.evidence.clone(),
    }
}

/// Edits overwrite in place; there is no history table.
pub fn update_item(conn: &mut Connection, id: &str, draft: &ItemDraft) -> Result<Item> {
    let draft = validate::draft(draft)?;
    let tx = write_tx(conn)?;
    require_item(&tx, id)?;
    write_item_fields(&tx, id, &draft)?;
    let item = require_item(&tx, id)?;
    tx.commit()?;
    Ok(item)
}

/// Omit hides an item from agents (kept, reversible); enabling clears `omitted_at`.
pub fn set_item_status(conn: &mut Connection, id: &str, status: ItemStatus) -> Result<Item> {
    let tx = write_tx(conn)?;
    let item = require_item(&tx, id)?;
    if item.status != status {
        let omitted_at = (status == ItemStatus::Omitted).then(now);
        tx.execute(
            "UPDATE items SET status = ?2, omitted_at = ?3 WHERE id = ?1",
            params![id, status.as_str(), omitted_at],
        )?;
    }
    let item = require_item(&tx, id)?;
    tx.commit()?;
    Ok(item)
}

/// Hard delete, all or nothing. Pending learnings that target these items stay;
/// confirming them later is rejected with a clear message.
pub fn delete_items(conn: &mut Connection, ids: &[String]) -> Result<usize> {
    let tx = write_tx(conn)?;
    for id in ids {
        if tx.execute("DELETE FROM items WHERE id = ?1", [id])? == 0 {
            return Err(Error::NotFound(format!(
                "item {id} doesn't exist; nothing was deleted"
            )));
        }
    }
    tx.commit()?;
    Ok(ids.len())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CombinePrefill {
    pub draft: ItemDraft,
    pub evidence_truncated: bool,
    /// The sources' distinct scopes; more than one means the user must pick.
    pub scopes: Vec<Option<String>>,
}

/// The editor prefill for Combine: concatenated text, union of tags/phases,
/// concatenated evidence truncated to the cap.
pub fn combine_prefill(conn: &Connection, ids: &[String]) -> Result<CombinePrefill> {
    let items = ids
        .iter()
        .map(|id| require_item(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let join = |f: fn(&Item) -> &str| {
        let mut parts: Vec<&str> = Vec::new();
        for item in &items {
            let text = f(item).trim();
            if !text.is_empty() && !parts.contains(&text) {
                parts.push(text);
            }
        }
        parts.join("\n\n")
    };
    let mut tags = Vec::new();
    let mut phases = Vec::new();
    let mut scopes: Vec<Option<String>> = Vec::new();
    let mut evidence: Vec<String> = Vec::new();
    let mut truncated = false;
    let mut total = 0;
    for item in &items {
        tags.extend(item.tags.iter().cloned());
        phases.extend(item.phases.iter().copied());
        if !scopes.contains(&item.repo_root) {
            scopes.push(item.repo_root.clone());
        }
        for entry in &item.evidence {
            if total + entry.len() > validate::EVIDENCE_TOTAL_MAX {
                truncated = true;
            } else {
                total += entry.len();
                evidence.push(entry.clone());
            }
        }
    }
    let mut seen = HashSet::new();
    tags.retain(|t| seen.insert(t.clone()));
    let first = items
        .first()
        .ok_or_else(|| Error::invalid("select at least two items to combine"))?;
    Ok(CombinePrefill {
        draft: ItemDraft {
            title: first.title.clone(),
            summary: first.summary.clone(),
            guidance: join(|i| &i.guidance),
            applies_when: join(|i| &i.applies_when),
            exceptions: join(|i| &i.exceptions),
            tags,
            phases: validate::phases(&phases),
            repo_root: if scopes.len() == 1 {
                first.repo_root.clone()
            } else {
                None
            },
            evidence,
        },
        evidence_truncated: truncated,
        scopes,
    })
}

/// Creates one active item (`source = manual`) and omits the sources, atomically.
pub fn combine_items(conn: &mut Connection, ids: &[String], draft: &ItemDraft) -> Result<Item> {
    let unique: HashSet<&String> = ids.iter().collect();
    if unique.len() < 2 || unique.len() != ids.len() {
        return Err(Error::invalid(
            "select at least two different items to combine",
        ));
    }
    let draft = validate::draft(draft)?;
    let tx = write_tx(conn)?;
    let sources = ids
        .iter()
        .map(|id| require_item(&tx, id))
        .collect::<Result<Vec<_>>>()?;
    let same = |f: fn(&Item) -> &str| {
        let first = f(&sources[0]);
        if sources.iter().all(|s| f(s) == first) {
            first.to_string()
        } else {
            "mixed".to_string()
        }
    };
    let harness = same(|i| &i.source_harness);
    let model = same(|i| &i.source_model);
    let item = insert_item(
        &tx,
        NewItem {
            draft: &draft,
            source: Source::Manual,
            harness: &harness,
            model: &model,
        },
    )?;
    let at = now();
    for id in ids {
        tx.execute(
            "UPDATE items SET status = 'omitted', omitted_at = ?2 WHERE id = ?1",
            params![id, at],
        )?;
    }
    tx.commit()?;
    Ok(item)
}

// ------------------------------------------------------------ agent recall

fn index_entry(item: &Item) -> IndexEntry {
    IndexEntry {
        id: item.id.clone(),
        title: item.title.clone(),
        summary: item.summary.clone(),
        tags: item.tags.clone(),
        phases: item.phases.clone(),
        scope: if item.repo_root.is_some() {
            "repo"
        } else {
            "personal"
        }
        .into(),
    }
}

fn in_phase(item: &Item, phase: Option<Phase>) -> bool {
    match phase {
        None => true,
        Some(p) => item.phases.is_empty() || item.phases.contains(&p),
    }
}

fn active_in_scope(conn: &Connection, repo_root: Option<&str>) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ITEM_COLUMNS} FROM items
         WHERE status = 'active' AND (repo_root IS NULL OR repo_root = ?1)
         ORDER BY id"
    ))?;
    let rows = stmt.query_map([repo_root], item_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// `list`: active items for the phase whose scope is personal or this repo.
pub fn list_index(
    conn: &mut Connection,
    phase: Phase,
    repo_root: Option<&str>,
) -> Result<Recall<Vec<IndexEntry>>> {
    let tx = conn.transaction()?;
    if settings(&tx)?.participation == Participation::Paused {
        return Ok(Recall::Paused);
    }
    let entries = active_in_scope(&tx, repo_root)?
        .iter()
        .filter(|i| in_phase(i, Some(phase)))
        .map(index_entry)
        .collect();
    tx.commit()?;
    Ok(Recall::Ok(entries))
}

/// `get`: full items, bumping usage counters in the same transaction as the read.
pub fn get_items(
    conn: &mut Connection,
    ids: &[String],
    repo_root: Option<&str>,
) -> Result<Recall<GetResult>> {
    if ids.is_empty() || ids.len() > validate::GET_MAX_IDS {
        return Err(Error::invalid(format!(
            "get takes 1 to {} item IDs",
            validate::GET_MAX_IDS
        )));
    }
    let tx = write_tx(conn)?;
    if settings(&tx)?.participation == Participation::Paused {
        return Ok(Recall::Paused);
    }
    let mut seen = HashSet::new();
    let mut result = GetResult {
        items: Vec::new(),
        not_returned: Vec::new(),
    };
    let at = now();
    for id in ids.iter().filter(|id| seen.insert(id.as_str())) {
        let reason = match get_item(&tx, id)? {
            None => Some(NotReturnedReason::Unknown),
            Some(i) if i.status == ItemStatus::Omitted => Some(NotReturnedReason::Omitted),
            Some(i) if i.repo_root.is_some() && i.repo_root.as_deref() != repo_root => {
                Some(NotReturnedReason::OutOfScope)
            }
            Some(_) => None,
        };
        match reason {
            Some(reason) => result.not_returned.push(NotReturned {
                id: id.clone(),
                reason,
            }),
            None => {
                tx.execute(
                    "UPDATE items SET times_returned = times_returned + 1, last_returned_at = ?2
                     WHERE id = ?1",
                    params![id, at],
                )?;
                result.items.push(require_item(&tx, id)?);
            }
        }
    }
    tx.commit()?;
    Ok(Recall::Ok(result))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IndexStats {
    pub total: u32,
    pub visible: u32,
    /// Size of the JSON index agents would receive for all active items.
    pub index_bytes: u32,
}

pub fn index_stats(conn: &Connection) -> Result<IndexStats> {
    let items = list_items(conn)?;
    let visible: Vec<IndexEntry> = items
        .iter()
        .filter(|i| i.status == ItemStatus::Active)
        .map(index_entry)
        .collect();
    Ok(IndexStats {
        total: items.len() as u32,
        visible: visible.len() as u32,
        index_bytes: to_json(&visible).len() as u32,
    })
}

// --------------------------------------------------------------- learnings

const LEARNING_COLUMNS: &str = "id, source, kind, target_id, payload, evidence, harness, model, \
     session_model, session_ref, signal_score, temperature, status, created_at, decided_at";

fn learning_from_row(r: &Row) -> rusqlite::Result<Learning> {
    let temperature: Option<String> = r.get(11)?;
    Ok(Learning {
        id: r.get(0)?,
        source: text_col(r, 1, Source::parse)?,
        kind: text_col(r, 2, LearningKind::parse)?,
        target_id: r.get(3)?,
        payload: json_col(r, 4)?,
        evidence: r.get(5)?,
        harness: r.get(6)?,
        model: r.get(7)?,
        session_model: r.get(8)?,
        session_ref: r.get(9)?,
        signal_score: r.get(10)?,
        temperature: temperature.as_deref().and_then(AutoLearn::parse),
        status: text_col(r, 12, LearningStatus::parse)?,
        created_at: r.get(13)?,
        decided_at: r.get(14)?,
    })
}

pub fn get_learning(conn: &Connection, id: &str) -> Result<Option<Learning>> {
    Ok(conn
        .query_row(
            &format!("SELECT {LEARNING_COLUMNS} FROM learnings WHERE id = ?1"),
            [id],
            learning_from_row,
        )
        .optional()?)
}

/// Learnings oldest first, optionally limited to one status.
pub fn list_learnings(conn: &Connection, status: Option<LearningStatus>) -> Result<Vec<Learning>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {LEARNING_COLUMNS} FROM learnings
         WHERE ?1 IS NULL OR status = ?1
         ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map([status.map(|s| s.as_str())], learning_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn pending_count(conn: &Connection) -> Result<u32> {
    Ok(conn.query_row(
        "SELECT count(*) FROM learnings WHERE status = 'pending'",
        [],
        |r| r.get(0),
    )?)
}

/// The most recent denials, newest first, so auto-learn can avoid repeating them.
pub fn recent_denials(conn: &Connection, limit: u32) -> Result<Vec<Learning>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {LEARNING_COLUMNS} FROM learnings WHERE status = 'denied'
         ORDER BY decided_at DESC, id DESC LIMIT ?1"
    ))?;
    let rows = stmt.query_map([limit], learning_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// `propose`: validate, unsafe-check, and insert as pending. Rejected while paused.
pub fn propose(conn: &mut Connection, proposal: &Proposal, prov: &Provenance) -> Result<Learning> {
    let proposal = validate::proposal(proposal)?;
    let harness = validate::label("harness", &prov.harness)?;
    let model = validate::label("model", &prov.model)?;
    let session_model = prov
        .session_model
        .as_deref()
        .map(|m| validate::label("session model", m))
        .transpose()?;
    let session_ref = prov
        .session_ref
        .as_deref()
        .map(|s| validate::label("session", s))
        .transpose()?;

    let tx = write_tx(conn)?;
    let s = settings(&tx)?;
    if s.participation == Participation::Paused {
        return Err(Error::Paused);
    }
    let (signal_score, temperature) = if prov.source == Source::Auto {
        let min = s
            .auto_learn
            .min_score()
            .ok_or_else(|| Error::invalid("inception mode is off; nothing was stored"))?;
        let score = proposal
            .signal_score
            .ok_or_else(|| Error::invalid("signal_score is required for auto learnings"))?;
        if score < min {
            return Err(Error::invalid(format!(
                "signal_score {score} is below what {} saves (≥ {min}); nothing was stored",
                s.auto_learn
            )));
        }
        (Some(score), Some(s.auto_learn))
    } else {
        (None, None)
    };
    if let Some(target) = &proposal.target_id {
        if get_item(&tx, target)?.is_none() {
            return Err(Error::invalid(format!(
                "target_id {target} is not a known item"
            )));
        }
    }
    let id = new_id();
    tx.execute(
        "INSERT INTO learnings (id, source, kind, target_id, payload, evidence, harness, model,
             session_model, session_ref, signal_score, temperature, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'pending', ?13)",
        params![
            id,
            prov.source.as_str(),
            proposal.kind.as_str(),
            proposal.target_id,
            to_json(&proposal.payload),
            proposal.evidence,
            harness,
            model,
            session_model,
            session_ref,
            signal_score,
            temperature.map(|t| t.as_str()),
            now(),
        ],
    )?;
    let learning = get_learning(&tx, &id)?.expect("just inserted");
    tx.commit()?;
    Ok(learning)
}

fn require_pending(tx: &Transaction, id: &str) -> Result<Learning> {
    let learning = get_learning(tx, id)?
        .ok_or_else(|| Error::NotFound(format!("learning {id} doesn't exist")))?;
    if learning.status != LearningStatus::Pending {
        return Err(Error::Conflict(format!(
            "this learning was already {}",
            learning.status
        )));
    }
    Ok(learning)
}

fn require_target(tx: &Transaction, learning: &Learning) -> Result<Item> {
    let id = learning.target_id.as_deref().unwrap_or_default();
    get_item(tx, id)?.ok_or_else(|| {
        Error::Conflict(
            "the item this learning targets was deleted; deny or delete the learning instead"
                .into(),
        )
    })
}

fn apply_payload(base: &ItemDraft, p: &LearningPayload) -> ItemDraft {
    ItemDraft {
        title: p.title.clone().unwrap_or_else(|| base.title.clone()),
        summary: p.summary.clone().unwrap_or_else(|| base.summary.clone()),
        guidance: p.guidance.clone().unwrap_or_else(|| base.guidance.clone()),
        applies_when: p
            .applies_when
            .clone()
            .unwrap_or_else(|| base.applies_when.clone()),
        exceptions: p
            .exceptions
            .clone()
            .unwrap_or_else(|| base.exceptions.clone()),
        tags: p.tags.clone().unwrap_or_else(|| base.tags.clone()),
        phases: p.phases.clone().unwrap_or_else(|| base.phases.clone()),
        repo_root: base.repo_root.clone(),
        evidence: base.evidence.clone(),
    }
}

/// Confirms a pending learning, optionally with the user's edits, atomically:
/// `new` inserts an active item, `reinforce` appends evidence and bumps
/// `reinforced_count`, `correct` applies the change. Returns the affected item.
pub fn confirm_learning(
    conn: &mut Connection,
    id: &str,
    edit: Option<&LearningPayload>,
) -> Result<Item> {
    let tx = write_tx(conn)?;
    let learning = require_pending(&tx, id)?;
    let payload = validate::payload_for(learning.kind, edit.unwrap_or(&learning.payload))?;

    let item = match learning.kind {
        LearningKind::New => {
            let mut draft = apply_payload(&ItemDraft::default(), &payload);
            draft.repo_root = payload.repo_root.clone();
            draft.evidence = validate::append_evidence(&[], &learning.evidence);
            let draft = validate::draft(&draft)?;
            insert_item(
                &tx,
                NewItem {
                    draft: &draft,
                    source: learning.source,
                    harness: &learning.harness,
                    model: &learning.model,
                },
            )?
        }
        LearningKind::Reinforce => {
            let target = require_target(&tx, &learning)?;
            let evidence = validate::append_evidence(&target.evidence, &learning.evidence);
            tx.execute(
                "UPDATE items SET evidence = ?2, reinforced_count = reinforced_count + 1, updated_at = ?3
                 WHERE id = ?1",
                params![target.id, to_json(&evidence), now()],
            )?;
            require_item(&tx, &target.id)?
        }
        LearningKind::Correct => {
            let target = require_target(&tx, &learning)?;
            let mut draft = apply_payload(&draft_of(&target), &payload);
            draft.evidence = validate::append_evidence(&target.evidence, &learning.evidence);
            let draft = validate::draft(&draft)?;
            write_item_fields(&tx, &target.id, &draft)?;
            require_item(&tx, &target.id)?
        }
    };

    tx.execute(
        "UPDATE learnings SET status = 'confirmed', payload = ?2, decided_at = ?3 WHERE id = ?1",
        params![id, to_json(&payload), now()],
    )?;
    tx.commit()?;
    Ok(item)
}

/// Denied learnings are kept so auto-learn can avoid repeating them.
pub fn deny_learning(conn: &mut Connection, id: &str) -> Result<Learning> {
    let tx = write_tx(conn)?;
    require_pending(&tx, id)?;
    tx.execute(
        "UPDATE learnings SET status = 'denied', decided_at = ?2 WHERE id = ?1",
        params![id, now()],
    )?;
    let learning = get_learning(&tx, id)?.expect("exists");
    tx.commit()?;
    Ok(learning)
}

pub fn delete_learning(conn: &mut Connection, id: &str) -> Result<()> {
    let tx = write_tx(conn)?;
    if tx.execute("DELETE FROM learnings WHERE id = ?1", [id])? == 0 {
        return Err(Error::NotFound(format!("learning {id} doesn't exist")));
    }
    tx.commit()?;
    Ok(())
}

// ------------------------------------------------------------------ status

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StoreStatus {
    pub participation: Participation,
    pub auto_learn: AutoLearn,
    pub pending: u32,
    pub active_items: u32,
    pub omitted_items: u32,
}

pub fn status(conn: &mut Connection) -> Result<StoreStatus> {
    let tx = conn.transaction()?;
    let s = settings(&tx)?;
    let count = |status: &str| -> Result<u32> {
        Ok(tx.query_row(
            "SELECT count(*) FROM items WHERE status = ?1",
            [status],
            |r| r.get(0),
        )?)
    };
    let out = StoreStatus {
        participation: s.participation,
        auto_learn: s.auto_learn,
        pending: pending_count(&tx)?,
        active_items: count("active")?,
        omitted_items: count("omitted")?,
    };
    tx.commit()?;
    Ok(out)
}

// ------------------------------------------------------------------- gauge

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GaugeRow {
    /// Temperature or model label.
    pub label: String,
    pub candidates: u32,
    pub confirmed: u32,
    pub denied: u32,
    /// Distinct observed sessions (auto-learn only; 0 when unknown).
    pub sessions: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Gauge {
    pub since: String,
    /// Auto learnings by the temperature they were captured at.
    pub by_temperature: Vec<GaugeRow>,
    /// Auto and Learn learnings by the model that wrote them.
    pub by_model: Vec<GaugeRow>,
}

/// Temperature gauge (§10.4), from learnings created at or after `since`.
pub fn gauge(conn: &Connection, since: &str) -> Result<Gauge> {
    let rows = |sql: &str| -> Result<Vec<GaugeRow>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([since], |r| {
            Ok(GaugeRow {
                label: r.get(0)?,
                candidates: r.get(1)?,
                confirmed: r.get(2)?,
                denied: r.get(3)?,
                sessions: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    };
    let tallies =
        "count(*), sum(status = 'confirmed'), sum(status = 'denied'), count(DISTINCT session_ref)";
    let by_temp = rows(&format!(
        "SELECT temperature, {tallies} FROM learnings
         WHERE source = 'auto' AND temperature IS NOT NULL AND created_at >= ?1
         GROUP BY temperature"
    ))?;
    let mut by_temperature: BTreeMap<usize, GaugeRow> = BTreeMap::new();
    for row in by_temp {
        let order = AutoLearn::parse(&row.label)
            .and_then(|t| AutoLearn::ALL.iter().position(|a| *a == t))
            .unwrap_or(usize::MAX);
        by_temperature.insert(order, row);
    }
    let by_model = rows(&format!(
        "SELECT model, {tallies} FROM learnings
         WHERE source IN ('auto', 'learn') AND created_at >= ?1
         GROUP BY model ORDER BY count(*) DESC, model"
    ))?;
    Ok(Gauge {
        since: since.to_string(),
        by_temperature: by_temperature.into_values().collect(),
        by_model,
    })
}
