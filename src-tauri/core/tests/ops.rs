//! Store operations against a temp DB (§5–§7, A2–A7).

use judgement_core::model::*;
use judgement_core::{ops, store, Error};
use rusqlite::Connection;

struct Store {
    _tmp: tempfile::TempDir,
    dir: std::path::PathBuf,
    conn: Connection,
}

fn store() -> Store {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let mut conn = store::open_for_app(&dir).unwrap().conn;
    ops::set_participation(&mut conn, Participation::Active).unwrap();
    Store {
        _tmp: tmp,
        dir,
        conn,
    }
}

fn prov(source: Source) -> Provenance {
    Provenance {
        source,
        harness: "claude-code".into(),
        model: "claude-opus-5-5".into(),
        session_model: None,
        session_ref: None,
    }
}

fn new_proposal(title: &str) -> Proposal {
    Proposal {
        kind: LearningKind::New,
        target_id: None,
        payload: LearningPayload {
            title: Some(title.into()),
            summary: Some(format!("{title} summary")),
            guidance: Some(format!("{title} guidance")),
            ..Default::default()
        },
        evidence: "The user said so twice.".into(),
        signal_score: None,
    }
}

/// Proposes and confirms a new item.
fn item(conn: &mut Connection, title: &str) -> Item {
    let l = ops::propose(conn, &new_proposal(title), &prov(Source::Learn)).unwrap();
    ops::confirm_learning(conn, &l.id, None).unwrap()
}

fn draft(title: &str) -> ItemDraft {
    ItemDraft {
        title: title.into(),
        summary: "one line".into(),
        guidance: "the advice".into(),
        ..Default::default()
    }
}

#[test]
fn propose_lands_pending_and_confirm_creates_active_item() {
    let mut s = store();
    let l = ops::propose(
        &mut s.conn,
        &new_proposal("Reuse first"),
        &prov(Source::Learn),
    )
    .unwrap();
    assert_eq!(l.status, LearningStatus::Pending);
    assert!(
        ops::list_items(&s.conn).unwrap().is_empty(),
        "nothing active before confirm (A3)"
    );

    let item = ops::confirm_learning(&mut s.conn, &l.id, None).unwrap();
    assert_eq!(item.status, ItemStatus::Active);
    assert_eq!(item.source, Source::Learn);
    assert_eq!(
        (item.source_harness.as_str(), item.source_model.as_str()),
        ("claude-code", "claude-opus-5-5")
    );
    assert_eq!(item.evidence, vec!["The user said so twice."]);
    assert_eq!(
        ops::get_learning(&s.conn, &l.id).unwrap().unwrap().status,
        LearningStatus::Confirmed
    );
    assert_eq!(ops::pending_count(&s.conn).unwrap(), 0);
}

#[test]
fn confirm_with_edit_applies_and_records_the_edit() {
    let mut s = store();
    let l = ops::propose(&mut s.conn, &new_proposal("Draft"), &prov(Source::Learn)).unwrap();
    let mut edit = l.payload.clone();
    edit.title = Some("Edited".into());
    edit.phases = Some(vec![Phase::Review, Phase::Planning, Phase::Review]);
    let item = ops::confirm_learning(&mut s.conn, &l.id, Some(&edit)).unwrap();
    assert_eq!(item.title, "Edited");
    assert_eq!(item.phases, vec![Phase::Planning, Phase::Review]);
    let stored = ops::get_learning(&s.conn, &l.id).unwrap().unwrap();
    assert_eq!(stored.payload.title.as_deref(), Some("Edited"));
}

#[test]
fn confirm_with_unsafe_edit_stores_nothing() {
    let mut s = store();
    let l = ops::propose(&mut s.conn, &new_proposal("Draft"), &prov(Source::Learn)).unwrap();
    let mut edit = l.payload.clone();
    edit.guidance = Some("export AWS key AKIAIOSFODNN7EXAMPLE".into());
    assert!(matches!(
        ops::confirm_learning(&mut s.conn, &l.id, Some(&edit)),
        Err(Error::Unsafe { .. })
    ));
    assert!(ops::list_items(&s.conn).unwrap().is_empty());
    assert_eq!(
        ops::get_learning(&s.conn, &l.id).unwrap().unwrap().status,
        LearningStatus::Pending
    );
}

#[test]
fn reinforce_appends_evidence_and_bumps_count() {
    let mut s = store();
    let target = item(&mut s.conn, "Parent tests");
    let p = Proposal {
        kind: LearningKind::Reinforce,
        target_id: Some(target.id.clone()),
        payload: Default::default(),
        evidence: "The user held it up again.".into(),
        signal_score: None,
    };
    let l = ops::propose(&mut s.conn, &p, &prov(Source::Learn)).unwrap();
    let updated = ops::confirm_learning(&mut s.conn, &l.id, None).unwrap();
    assert_eq!(updated.reinforced_count, 1);
    assert_eq!(updated.evidence.len(), 2);
    assert_eq!(updated.guidance, target.guidance);
}

#[test]
fn correct_applies_only_changed_fields() {
    let mut s = store();
    let target = item(&mut s.conn, "Parent tests");
    let p = Proposal {
        kind: LearningKind::Correct,
        target_id: Some(target.id.clone()),
        payload: LearningPayload {
            guidance: Some("Assert only the parent's decisions.".into()),
            ..Default::default()
        },
        evidence: "The user cut five tests.".into(),
        signal_score: None,
    };
    let l = ops::propose(&mut s.conn, &p, &prov(Source::Learn)).unwrap();
    let updated = ops::confirm_learning(&mut s.conn, &l.id, None).unwrap();
    assert_eq!(updated.guidance, "Assert only the parent's decisions.");
    assert_eq!(updated.title, target.title);
}

#[test]
fn confirm_rejects_when_target_was_deleted() {
    let mut s = store();
    let target = item(&mut s.conn, "Doomed");
    let p = Proposal {
        kind: LearningKind::Reinforce,
        target_id: Some(target.id.clone()),
        payload: Default::default(),
        evidence: "Held up.".into(),
        signal_score: None,
    };
    let l = ops::propose(&mut s.conn, &p, &prov(Source::Learn)).unwrap();
    ops::delete_items(&mut s.conn, std::slice::from_ref(&target.id)).unwrap();
    let err = ops::confirm_learning(&mut s.conn, &l.id, None).unwrap_err();
    assert!(err.to_string().contains("was deleted"), "{err}");
    assert_eq!(
        ops::get_learning(&s.conn, &l.id).unwrap().unwrap().status,
        LearningStatus::Pending
    );
}

#[test]
fn propose_rejects_unknown_target() {
    let mut s = store();
    let p = Proposal {
        kind: LearningKind::Reinforce,
        target_id: Some(judgement_core::new_id()),
        payload: Default::default(),
        evidence: "Held up.".into(),
        signal_score: None,
    };
    assert!(matches!(
        ops::propose(&mut s.conn, &p, &prov(Source::Learn)),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn deny_is_kept_and_cannot_be_decided_twice() {
    let mut s = store();
    let l = ops::propose(&mut s.conn, &new_proposal("Nope"), &prov(Source::Learn)).unwrap();
    ops::deny_learning(&mut s.conn, &l.id).unwrap();
    assert_eq!(ops::recent_denials(&s.conn, 20).unwrap().len(), 1);
    assert!(matches!(
        ops::confirm_learning(&mut s.conn, &l.id, None),
        Err(Error::Conflict(_))
    ));
    ops::delete_learning(&mut s.conn, &l.id).unwrap();
    assert!(ops::get_learning(&s.conn, &l.id).unwrap().is_none());
}

#[test]
fn unsafe_proposal_is_rejected_and_nothing_stored() {
    let mut s = store();
    let mut p = new_proposal("Leaky");
    p.evidence = "-----BEGIN OPENSSH PRIVATE KEY----- was in the log".into();
    match ops::propose(&mut s.conn, &p, &prov(Source::Learn)) {
        Err(Error::Unsafe { field, .. }) => assert_eq!(field, "evidence"),
        other => panic!("expected unsafe, got {other:?}"),
    }
    let mut p = new_proposal("Leaky label");
    p.evidence = "fine".into();
    let mut bad = prov(Source::Learn);
    bad.model = "ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789".into();
    assert!(matches!(
        ops::propose(&mut s.conn, &p, &bad),
        Err(Error::Unsafe { .. })
    ));
    assert!(ops::list_learnings(&s.conn, None).unwrap().is_empty());
}

#[test]
fn pause_gates_agents_but_not_curation() {
    let mut s = store();
    let a = item(&mut s.conn, "Kept");
    ops::set_participation(&mut s.conn, Participation::Paused).unwrap();

    assert_eq!(
        ops::list_index(&mut s.conn, Phase::Coding, None).unwrap(),
        Recall::Paused
    );
    assert_eq!(
        ops::get_items(&mut s.conn, std::slice::from_ref(&a.id), None).unwrap(),
        Recall::Paused
    );
    assert_eq!(
        ops::get_item(&s.conn, &a.id)
            .unwrap()
            .unwrap()
            .times_returned,
        0,
        "no counter bumps"
    );
    assert!(matches!(
        ops::propose(&mut s.conn, &new_proposal("x"), &prov(Source::Learn)),
        Err(Error::Paused)
    ));

    ops::update_item(&mut s.conn, &a.id, &draft("Still editable")).unwrap();
    ops::set_item_status(&mut s.conn, &a.id, ItemStatus::Omitted).unwrap();
    ops::delete_items(&mut s.conn, std::slice::from_ref(&a.id)).unwrap();
}

#[test]
fn pause_is_linearizable_across_connections() {
    let s = store();
    let path = store::store_path(&s.dir);
    let mut agent = Connection::open(&path).unwrap();
    agent.busy_timeout(store::BUSY_TIMEOUT).unwrap();
    let mut human = Connection::open(&path).unwrap();
    human.busy_timeout(store::BUSY_TIMEOUT).unwrap();

    ops::set_participation(&mut human, Participation::Paused).unwrap();
    assert_eq!(
        ops::list_index(&mut agent, Phase::Review, None).unwrap(),
        Recall::Paused
    );
    assert!(matches!(
        ops::propose(&mut agent, &new_proposal("x"), &prov(Source::Learn)),
        Err(Error::Paused)
    ));
    ops::set_participation(&mut human, Participation::Active).unwrap();
    assert!(matches!(
        ops::list_index(&mut agent, Phase::Review, None).unwrap(),
        Recall::Ok(_)
    ));
}

#[test]
fn concurrent_writers_all_land() {
    let s = store();
    let path = store::store_path(&s.dir);
    let threads: Vec<_> = (0..4)
        .map(|t| {
            let path = path.clone();
            std::thread::spawn(move || {
                let mut conn = Connection::open(&path).unwrap();
                conn.busy_timeout(store::BUSY_TIMEOUT).unwrap();
                for i in 0..10 {
                    ops::propose(
                        &mut conn,
                        &new_proposal(&format!("t{t} n{i}")),
                        &prov(Source::Learn),
                    )
                    .unwrap();
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(ops::pending_count(&s.conn).unwrap(), 40);
}

#[test]
fn list_filters_by_status_phase_and_scope() {
    let mut s = store();
    let repo = "/work/app/.git";
    let personal_all = item(&mut s.conn, "Personal all phases");
    let review_only = item(&mut s.conn, "Review only");
    let mut d = ops::get_item(&s.conn, &review_only.id).unwrap().unwrap();
    d.phases = vec![Phase::Review];
    ops::update_item(
        &mut s.conn,
        &d.id,
        &ItemDraft {
            phases: d.phases.clone(),
            ..draft(&d.title)
        },
    )
    .unwrap();
    let repo_item = item(&mut s.conn, "Repo item");
    ops::update_item(
        &mut s.conn,
        &repo_item.id,
        &ItemDraft {
            repo_root: Some(repo.into()),
            ..draft("Repo item")
        },
    )
    .unwrap();
    let omitted = item(&mut s.conn, "Omitted");
    ops::set_item_status(&mut s.conn, &omitted.id, ItemStatus::Omitted).unwrap();

    let ids = |r: Recall<Vec<IndexEntry>>| match r {
        Recall::Ok(entries) => entries.into_iter().map(|e| e.id).collect::<Vec<_>>(),
        Recall::Paused => panic!("paused"),
    };
    assert_eq!(
        ids(ops::list_index(&mut s.conn, Phase::Coding, None).unwrap()),
        vec![personal_all.id.clone()]
    );
    assert_eq!(
        ids(ops::list_index(&mut s.conn, Phase::Review, Some(repo)).unwrap()),
        vec![
            personal_all.id.clone(),
            review_only.id.clone(),
            repo_item.id.clone()
        ]
    );
    assert_eq!(
        ids(ops::list_index(&mut s.conn, Phase::Coding, Some("/other/.git")).unwrap()),
        vec![personal_all.id.clone()]
    );
    match ops::list_index(&mut s.conn, Phase::Coding, Some(repo)).unwrap() {
        Recall::Ok(entries) => assert_eq!(entries[1].scope, "repo"),
        Recall::Paused => panic!(),
    }
}

#[test]
fn get_bumps_counters_and_reports_what_it_withholds() {
    let mut s = store();
    let a = item(&mut s.conn, "A");
    let omitted = item(&mut s.conn, "B");
    ops::set_item_status(&mut s.conn, &omitted.id, ItemStatus::Omitted).unwrap();
    let scoped = item(&mut s.conn, "C");
    ops::update_item(
        &mut s.conn,
        &scoped.id,
        &ItemDraft {
            repo_root: Some("/r/.git".into()),
            ..draft("C")
        },
    )
    .unwrap();
    let unknown = judgement_core::new_id();

    let ids = vec![
        a.id.clone(),
        omitted.id.clone(),
        scoped.id.clone(),
        unknown.clone(),
        a.id.clone(),
    ];
    let Recall::Ok(result) = ops::get_items(&mut s.conn, &ids, None).unwrap() else {
        panic!()
    };
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].times_returned, 1, "duplicate IDs bump once");
    assert!(result.items[0].last_returned_at.is_some());
    let reasons: Vec<_> = result
        .not_returned
        .iter()
        .map(|n| n.reason.clone())
        .collect();
    assert_eq!(
        reasons,
        vec![
            NotReturnedReason::Omitted,
            NotReturnedReason::OutOfScope,
            NotReturnedReason::Unknown
        ]
    );
    assert!(ops::get_items(&mut s.conn, &vec![a.id.clone(); 11], None).is_err());
}

#[test]
fn omit_and_enable_round_trip() {
    let mut s = store();
    let a = item(&mut s.conn, "A");
    let omitted = ops::set_item_status(&mut s.conn, &a.id, ItemStatus::Omitted).unwrap();
    assert!(omitted.omitted_at.is_some());
    let enabled = ops::set_item_status(&mut s.conn, &a.id, ItemStatus::Active).unwrap();
    assert!(enabled.omitted_at.is_none());
    assert_eq!(enabled.guidance, a.guidance, "omit keeps the item intact");
}

#[test]
fn combine_creates_manual_item_and_omits_sources() {
    let mut s = store();
    let a = item(&mut s.conn, "A");
    let mut auto_prov = prov(Source::Learn);
    auto_prov.model = "gpt-6.1-sol".into();
    let l = ops::propose(&mut s.conn, &new_proposal("B"), &auto_prov).unwrap();
    let b = ops::confirm_learning(&mut s.conn, &l.id, None).unwrap();

    let ids = vec![a.id.clone(), b.id.clone()];
    let prefill = ops::combine_prefill(&s.conn, &ids).unwrap();
    assert!(
        prefill.draft.guidance.contains("A guidance")
            && prefill.draft.guidance.contains("B guidance")
    );
    assert_eq!(
        prefill.draft.evidence.len(),
        2,
        "each source's evidence is carried over"
    );
    let combined = ops::combine_items(&mut s.conn, &ids, &draft("Reuse before you build")).unwrap();
    assert_eq!(combined.source, Source::Manual);
    assert_eq!(combined.source_harness, "claude-code");
    assert_eq!(combined.source_model, "mixed");
    for id in &ids {
        assert_eq!(
            ops::get_item(&s.conn, id).unwrap().unwrap().status,
            ItemStatus::Omitted
        );
    }
}

#[test]
fn combine_is_atomic() {
    let mut s = store();
    let a = item(&mut s.conn, "A");
    let ids = vec![a.id.clone(), judgement_core::new_id()];
    assert!(matches!(
        ops::combine_items(&mut s.conn, &ids, &draft("X")),
        Err(Error::NotFound(_))
    ));
    let mut bad = draft("X");
    bad.guidance = "password = Sup3rSecret!".into();
    let b = item(&mut s.conn, "B");
    assert!(ops::combine_items(&mut s.conn, &[a.id.clone(), b.id.clone()], &bad).is_err());
    let items = ops::list_items(&s.conn).unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.status == ItemStatus::Active));
}

#[test]
fn combine_prefill_truncates_evidence_and_reports_mixed_scopes() {
    let mut s = store();
    let big: Vec<String> = (0..4)
        .map(|i| format!("{i} {}", "e".repeat(1000)))
        .collect();
    let a = item(&mut s.conn, "A");
    ops::update_item(
        &mut s.conn,
        &a.id,
        &ItemDraft {
            evidence: big.clone(),
            ..draft("A")
        },
    )
    .unwrap();
    let b = item(&mut s.conn, "B");
    ops::update_item(
        &mut s.conn,
        &b.id,
        &ItemDraft {
            evidence: big,
            repo_root: Some("/r/.git".into()),
            ..draft("B")
        },
    )
    .unwrap();
    let prefill = ops::combine_prefill(&s.conn, &[a.id, b.id]).unwrap();
    assert!(prefill.evidence_truncated);
    assert_eq!(prefill.scopes.len(), 2);
    assert_eq!(prefill.draft.repo_root, None);
}

#[test]
fn delete_items_is_all_or_nothing() {
    let mut s = store();
    let a = item(&mut s.conn, "A");
    assert!(ops::delete_items(&mut s.conn, &[a.id.clone(), judgement_core::new_id()]).is_err());
    assert_eq!(ops::list_items(&s.conn).unwrap().len(), 1);
}

#[test]
fn auto_learnings_follow_temperature() {
    let mut s = store();
    let mut p = new_proposal("Auto");
    p.signal_score = Some(3);
    let auto = prov(Source::Auto);
    assert!(ops::propose(&mut s.conn, &p, &auto)
        .unwrap_err()
        .to_string()
        .contains("inception mode is off"));

    ops::set_auto_learn(&mut s.conn, AutoLearn::Cool).unwrap();
    assert!(ops::propose(&mut s.conn, &p, &auto)
        .unwrap_err()
        .to_string()
        .contains("below"));

    ops::set_auto_learn(&mut s.conn, AutoLearn::Warm).unwrap();
    let l = ops::propose(&mut s.conn, &p, &auto).unwrap();
    assert_eq!(
        (l.signal_score, l.temperature),
        (Some(3), Some(AutoLearn::Warm))
    );

    p.signal_score = None;
    assert!(
        ops::propose(&mut s.conn, &p, &auto).is_err(),
        "auto requires a score"
    );
}

#[test]
fn gauge_counts_by_temperature_and_model() {
    let mut s = store();
    ops::set_auto_learn(&mut s.conn, AutoLearn::Hot).unwrap();
    let mut auto = prov(Source::Auto);
    auto.model = "gpt-6.1-sol".into();
    for (i, session) in ["s1", "s1", "s2"].iter().enumerate() {
        auto.session_ref = Some(session.to_string());
        let mut p = new_proposal(&format!("auto {i}"));
        p.signal_score = Some(2);
        let l = ops::propose(&mut s.conn, &p, &auto).unwrap();
        if i == 0 {
            ops::confirm_learning(&mut s.conn, &l.id, None).unwrap();
        } else {
            ops::deny_learning(&mut s.conn, &l.id).unwrap();
        }
    }
    item(&mut s.conn, "learned");

    let g = ops::gauge(&s.conn, "2000-01-01T00:00:00Z").unwrap();
    assert_eq!(g.by_temperature.len(), 1);
    let hot = &g.by_temperature[0];
    assert_eq!(
        (
            hot.label.as_str(),
            hot.candidates,
            hot.confirmed,
            hot.denied,
            hot.sessions
        ),
        ("hot", 3, 1, 2, 2)
    );
    let models: Vec<_> = g
        .by_model
        .iter()
        .map(|r| (r.label.as_str(), r.candidates))
        .collect();
    assert_eq!(models, vec![("gpt-6.1-sol", 3), ("claude-opus-5-5", 1)]);
}

#[test]
fn index_stats_counts_visible_items() {
    let mut s = store();
    item(&mut s.conn, "A");
    let b = item(&mut s.conn, "B");
    ops::set_item_status(&mut s.conn, &b.id, ItemStatus::Omitted).unwrap();
    let stats = ops::index_stats(&s.conn).unwrap();
    assert_eq!((stats.total, stats.visible), (2, 1));
    assert!(stats.index_bytes > 0);
}
