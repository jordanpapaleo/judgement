//! Where the store lives, how each process opens it, migrations, and backups.
//!
//! Only the app creates or migrates the store (`open_for_app`). The CLI opens
//! an existing store and refuses on a schema mismatch (`open_for_cli`).

use std::fs::{self, DirBuilder, OpenOptions};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior, MAIN_DB};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result};
use crate::model::Analyzer;
use crate::now;

pub const BUNDLE_ID: &str = "com.jordanpapaleo.judgement";
pub const STORE_FILE: &str = "judgement.sqlite";
pub const BACKUP_DIR: &str = "backups";
pub const BACKUPS_KEPT: usize = 5;

/// Long enough to ride out another process's write, short enough that the CLI
/// still answers within its 3-second budget (A8).
pub const BUSY_TIMEOUT: Duration = Duration::from_millis(2000);

/// Ordered migrations; index + 1 is the `user_version` each one produces.
const MIGRATIONS: &[&str] = &[include_str!("../migrations/001_init.sql")];

pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// The app-data directory: Tauri's `app_data_dir()` for this bundle ID on macOS.
/// `JUDGEMENT_HOME` overrides it (tests, alternate stores).
pub fn default_data_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("JUDGEMENT_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            Error::Unavailable("HOME is not set; can't locate the Judgement store".into())
        })?;
    let home = PathBuf::from(home);
    if cfg!(target_os = "macos") {
        Ok(home.join("Library/Application Support").join(BUNDLE_ID))
    } else {
        let data = std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        Ok(data.join(BUNDLE_ID))
    }
}

pub fn store_path(dir: &Path) -> PathBuf {
    dir.join(STORE_FILE)
}

pub fn backup_dir(dir: &Path) -> PathBuf {
    dir.join(BACKUP_DIR)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(())
}

fn user_version(conn: &Connection) -> Result<i64> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| Error::Unavailable(format!("the store can't be read ({e})")))
}

/// Opens an existing store for the CLI. Never creates or migrates.
pub fn open_for_cli(dir: &Path) -> Result<Connection> {
    let path = store_path(dir);
    if !path.exists() {
        return Err(Error::Unavailable(format!(
            "no Judgement store at {}; open Judgement once to create it",
            path.display()
        )));
    }
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| Error::Unavailable(format!("can't open {} ({e})", path.display())))?;
    configure(&conn)?;
    let found = user_version(&conn)?;
    if found != SCHEMA_VERSION {
        return Err(Error::SchemaMismatch {
            found,
            expected: SCHEMA_VERSION,
        });
    }
    Ok(conn)
}

/// What `open_for_app` did, for the tray and logs.
#[derive(Debug)]
pub struct Opened {
    pub conn: Connection,
    pub created: bool,
    pub migrated_from: Option<i64>,
    pub backup: Option<PathBuf>,
}

/// Opens the store for the app: creates it (dir 0700, file 0600) on first run,
/// backs up and migrates an older schema. A damaged or foreign file is reported,
/// never recreated or touched.
pub fn open_for_app(dir: &Path) -> Result<Opened> {
    if !dir.exists() {
        DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    }
    let path = store_path(dir);
    let created = !path.exists();
    if created {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
    }
    let mut conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| Error::Unavailable(format!("can't open {} ({e})", path.display())))?;
    configure(&conn)?;

    let found = user_version(&conn)?;
    if found > SCHEMA_VERSION {
        return Err(Error::Unavailable(format!(
            "the store is schema {found}, newer than this build ({SCHEMA_VERSION}); install a newer Judgement"
        )));
    }
    if found == 0 {
        let tables: i64 = conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))
            .map_err(|e| Error::Unavailable(format!("the store can't be read ({e})")))?;
        if tables > 0 {
            return Err(Error::Unavailable(format!(
                "{} is not a Judgement store; it was left untouched",
                path.display()
            )));
        }
    }
    conn.pragma_update(None, "journal_mode", "WAL")?;

    let mut backup = None;
    if found < SCHEMA_VERSION {
        if found > 0 {
            backup = Some(backup_now(&conn, dir, BackupKind::Auto)?);
        }
        migrate(&mut conn, found)?;
    }
    Ok(Opened {
        conn,
        created,
        migrated_from: (found < SCHEMA_VERSION).then_some(found),
        backup,
    })
}

fn migrate(conn: &mut Connection, from: i64) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(from as usize) {
        tx.execute_batch(sql)?;
        if index == 0 {
            // First launch is paused until the user resumes (§5.1).
            tx.execute(
                "INSERT INTO settings (id, participation, auto_learn, analyzer, updated_at)
                 VALUES (1, 'paused', 'off', ?1, ?2)",
                params![
                    serde_json::to_string(&Analyzer::default()).expect("serializable"),
                    now()
                ],
            )?;
        }
    }
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum BackupKind {
    /// Taken automatically before a migration.
    Auto,
    /// Taken from "Back Up Now".
    Manual,
}

impl BackupKind {
    fn prefix(self) -> &'static str {
        match self {
            BackupKind::Auto => "auto-",
            BackupKind::Manual => "manual-",
        }
    }
}

/// Copies the live store with SQLite's online backup API, then keeps the
/// newest `BACKUPS_KEPT` of that kind.
pub fn backup_now(conn: &Connection, dir: &Path, kind: BackupKind) -> Result<PathBuf> {
    let backups = backup_dir(dir);
    if !backups.exists() {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&backups)?;
    }
    let stamp = Utc::now().format("%Y%m%dT%H%M%S%3fZ");
    let path = backups.join(format!("{}{stamp}.sqlite", kind.prefix()));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    conn.backup(MAIN_DB, &path, None)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;

    let mut existing = backup_files(dir, kind)?;
    while existing.len() > BACKUPS_KEPT {
        fs::remove_file(existing.remove(0))?;
    }
    Ok(path)
}

/// Backups of one kind, oldest first (names sort by timestamp).
fn backup_files(dir: &Path, kind: BackupKind) -> Result<Vec<PathBuf>> {
    let backups = backup_dir(dir);
    if !backups.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(backups)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(kind.prefix()) && n.ends_with(".sqlite"))
        })
        .collect();
    files.sort();
    Ok(files)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BackupSummary {
    pub dir: String,
    pub manual_count: u32,
    pub auto_count: u32,
    /// Modification time of the newest backup of either kind.
    pub last_backup_at: Option<String>,
}

pub fn backup_summary(dir: &Path) -> Result<BackupSummary> {
    let manual = backup_files(dir, BackupKind::Manual)?;
    let auto = backup_files(dir, BackupKind::Auto)?;
    let last = manual
        .iter()
        .chain(auto.iter())
        .filter_map(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
        .max()
        .map(|t| DateTime::<Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    Ok(BackupSummary {
        dir: backup_dir(dir).display().to_string(),
        manual_count: manual.len() as u32,
        auto_count: auto.len() as u32,
        last_backup_at: last,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creates_store_paused_with_private_permissions() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("data");
        let opened = open_for_app(&dir).unwrap();
        assert!(opened.created);
        assert_eq!(opened.migrated_from, Some(0));
        assert!(opened.backup.is_none(), "nothing to back up on first run");

        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&store_path(&dir)), 0o600);

        let s = crate::ops::settings(&opened.conn).unwrap();
        assert_eq!(s.participation, crate::model::Participation::Paused);
        assert_eq!(s.auto_learn, crate::model::AutoLearn::Off);
    }

    #[test]
    fn reopening_is_a_no_op() {
        let tmp = tempfile::tempdir().unwrap();
        open_for_app(tmp.path()).unwrap();
        let again = open_for_app(tmp.path()).unwrap();
        assert!(!again.created);
        assert_eq!(again.migrated_from, None);
    }

    #[test]
    fn cli_refuses_missing_store() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            open_for_cli(tmp.path()),
            Err(Error::Unavailable(_))
        ));
    }

    #[test]
    fn cli_refuses_schema_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let opened = open_for_app(tmp.path()).unwrap();
        opened
            .conn
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        drop(opened);
        match open_for_cli(tmp.path()) {
            Err(Error::SchemaMismatch { found, expected }) => {
                assert_eq!((found, expected), (SCHEMA_VERSION + 1, SCHEMA_VERSION));
            }
            other => panic!("expected schema mismatch, got {other:?}"),
        }
        assert!(
            open_for_app(tmp.path()).is_err(),
            "app refuses a newer store too"
        );
    }

    #[test]
    fn app_never_overwrites_a_damaged_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = store_path(tmp.path());
        fs::write(&path, b"this is not sqlite at all, just some bytes padding padding padding padding padding padding padding padding").unwrap();
        assert!(open_for_app(tmp.path()).is_err());
        assert!(fs::read(&path).unwrap().starts_with(b"this is not sqlite"));
    }

    #[test]
    fn app_refuses_a_foreign_database() {
        let tmp = tempfile::tempdir().unwrap();
        let conn = Connection::open(store_path(tmp.path())).unwrap();
        conn.execute_batch("CREATE TABLE notes (x)").unwrap();
        drop(conn);
        let err = open_for_app(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("not a Judgement store"));
    }

    #[test]
    fn backups_roll_at_five_per_kind() {
        let tmp = tempfile::tempdir().unwrap();
        let opened = open_for_app(tmp.path()).unwrap();
        for _ in 0..7 {
            backup_now(&opened.conn, tmp.path(), BackupKind::Manual).unwrap();
            std::thread::sleep(Duration::from_millis(2));
        }
        backup_now(&opened.conn, tmp.path(), BackupKind::Auto).unwrap();
        let summary = backup_summary(tmp.path()).unwrap();
        assert_eq!((summary.manual_count, summary.auto_count), (5, 1));
        assert!(summary.last_backup_at.is_some());

        let backup = backup_files(tmp.path(), BackupKind::Auto)
            .unwrap()
            .remove(0);
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let copy = Connection::open(backup).unwrap();
        assert_eq!(user_version(&copy).unwrap(), SCHEMA_VERSION);
    }
}
