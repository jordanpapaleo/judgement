//! Tauri commands: the window's only path to the store. Each one is a thin
//! call into `judgement_core::ops`; every write is validated and unsafe-checked
//! there. After a write, the tray and window refresh from committed state.

use judgement_core::model::*;
use judgement_core::ops::{self, CombinePrefill, Gauge, IndexStats};
use judgement_core::store::{self, BackupKind, BackupSummary};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use ts_rs::TS;

use crate::cli_link::{self, CliLink};
use crate::state::AppState;

type Res<T> = Result<T, String>;

/// Everything the shell (sidebar, header, Settings) needs in one read.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Overview {
    pub settings: Settings,
    pub pending: u32,
    pub index: IndexStats,
}

/// Where the store lives and, if it couldn't be opened, why. Never fails.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StoreInfo {
    pub store_path: String,
    pub backups_dir: String,
    pub error: Option<String>,
}

const GAUGE_DAYS: i64 = 30;

/// Runs a write, then refreshes the tray and tells the window the store changed.
fn write<T>(
    app: &AppHandle,
    state: &AppState,
    f: impl FnOnce(&mut rusqlite::Connection) -> judgement_core::Result<T>,
) -> Res<T> {
    let out = state.with(f);
    crate::store_changed(app);
    out
}

#[tauri::command]
pub fn store_info(state: State<AppState>) -> StoreInfo {
    StoreInfo {
        store_path: state.store_path().display().to_string(),
        backups_dir: store::backup_dir(&state.dir).display().to_string(),
        error: state.failure(),
    }
}

#[tauri::command]
pub fn overview(state: State<AppState>) -> Res<Overview> {
    state.with(|conn| {
        Ok(Overview {
            settings: ops::settings(conn)?,
            pending: ops::pending_count(conn)?,
            index: ops::index_stats(conn)?,
        })
    })
}

#[tauri::command]
pub fn set_participation(
    app: AppHandle,
    state: State<AppState>,
    participation: Participation,
) -> Res<Settings> {
    write(&app, &state, |conn| {
        ops::set_participation(conn, participation)
    })
}

#[tauri::command]
pub fn set_auto_learn(
    app: AppHandle,
    state: State<AppState>,
    auto_learn: AutoLearn,
) -> Res<Settings> {
    write(&app, &state, |conn| ops::set_auto_learn(conn, auto_learn))
}

#[tauri::command]
pub fn set_analyzer(app: AppHandle, state: State<AppState>, analyzer: Analyzer) -> Res<Settings> {
    write(&app, &state, |conn| ops::set_analyzer(conn, &analyzer))
}

#[tauri::command]
pub fn list_pending(state: State<AppState>) -> Res<Vec<Learning>> {
    state.with(|conn| ops::list_learnings(conn, Some(LearningStatus::Pending)))
}

#[tauri::command]
pub fn confirm_learning(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    edit: Option<LearningPayload>,
) -> Res<Item> {
    write(&app, &state, |conn| {
        ops::confirm_learning(conn, &id, edit.as_ref())
    })
}

#[tauri::command]
pub fn deny_learning(app: AppHandle, state: State<AppState>, id: String) -> Res<Learning> {
    write(&app, &state, |conn| ops::deny_learning(conn, &id))
}

#[tauri::command]
pub fn delete_learning(app: AppHandle, state: State<AppState>, id: String) -> Res<()> {
    write(&app, &state, |conn| ops::delete_learning(conn, &id))
}

#[tauri::command]
pub fn list_items(state: State<AppState>) -> Res<Vec<Item>> {
    state.with(|conn| ops::list_items(conn))
}

#[tauri::command]
pub fn update_item(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    draft: ItemDraft,
) -> Res<Item> {
    write(&app, &state, |conn| ops::update_item(conn, &id, &draft))
}

#[tauri::command]
pub fn set_item_status(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    status: ItemStatus,
) -> Res<Item> {
    write(&app, &state, |conn| ops::set_item_status(conn, &id, status))
}

#[tauri::command]
pub fn delete_items(app: AppHandle, state: State<AppState>, ids: Vec<String>) -> Res<u32> {
    write(&app, &state, |conn| {
        ops::delete_items(conn, &ids).map(|n| n as u32)
    })
}

#[tauri::command]
pub fn combine_prefill(state: State<AppState>, ids: Vec<String>) -> Res<CombinePrefill> {
    state.with(|conn| ops::combine_prefill(conn, &ids))
}

#[tauri::command]
pub fn combine_items(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
    draft: ItemDraft,
) -> Res<Item> {
    write(&app, &state, |conn| ops::combine_items(conn, &ids, &draft))
}

#[tauri::command]
pub fn gauge(state: State<AppState>) -> Res<Gauge> {
    state.with(|conn| ops::gauge(conn, &judgement_core::days_ago(GAUGE_DAYS)))
}

#[tauri::command]
pub fn backup_summary(state: State<AppState>) -> Res<BackupSummary> {
    store::backup_summary(&state.dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn backup_now(state: State<AppState>) -> Res<BackupSummary> {
    state.with(|conn| store::backup_now(conn, &state.dir, BackupKind::Manual))?;
    store::backup_summary(&state.dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reveal_backups(state: State<AppState>) {
    crate::reveal_backups(&state);
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Res<bool> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Res<bool> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    crate::store_changed(&app);
    result.map_err(|e| e.to_string())?;
    launcher.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cli_link_status() -> CliLink {
    cli_link::status()
}

#[tauri::command]
pub fn install_cli_link() -> Res<CliLink> {
    cli_link::install()
}
