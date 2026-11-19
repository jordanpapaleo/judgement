//! The app's one store connection. If the store failed to open or migrate,
//! the error is kept instead and every command reports it; the file on disk
//! is left untouched.

use std::path::PathBuf;
use std::sync::Mutex;

use judgement_core::model::Participation;
use judgement_core::{ops, store};
use rusqlite::Connection;

pub enum Store {
    Ready(Connection),
    Failed(String),
}

pub struct AppState {
    pub dir: PathBuf,
    pub store: Mutex<Store>,
}

/// What the tray shows; rebuilt only when it changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayModel {
    Ready {
        participation: Participation,
        pending: u32,
        autostart: bool,
    },
    Failed,
}

impl AppState {
    /// Startup: open store → back up → migrate if needed. Failure is recorded, not fatal.
    pub fn open(dir: PathBuf) -> Self {
        let store = match store::open_for_app(&dir) {
            Ok(opened) => Store::Ready(opened.conn),
            Err(e) => Store::Failed(e.to_string()),
        };
        AppState {
            dir,
            store: Mutex::new(store),
        }
    }

    pub fn store_path(&self) -> PathBuf {
        store::store_path(&self.dir)
    }

    /// Runs `f` with the connection, mapping every error to a message for the UI.
    pub fn with<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> judgement_core::Result<T>,
    ) -> Result<T, String> {
        let mut guard = self
            .store
            .lock()
            .map_err(|_| "the store lock was poisoned".to_string())?;
        match &mut *guard {
            Store::Ready(conn) => f(conn).map_err(|e| e.to_string()),
            Store::Failed(message) => Err(format!("Judgement can't open its store: {message}")),
        }
    }

    pub fn failure(&self) -> Option<String> {
        match &*self.store.lock().ok()? {
            Store::Ready(_) => None,
            Store::Failed(message) => Some(message.clone()),
        }
    }

    pub fn tray_model(&self, autostart: bool) -> TrayModel {
        let read = self.with(|conn| {
            Ok((
                ops::settings(conn)?.participation,
                ops::pending_count(conn)?,
            ))
        });
        match read {
            Ok((participation, pending)) => TrayModel::Ready {
                participation,
                pending,
                autostart,
            },
            Err(_) => TrayModel::Failed,
        }
    }
}
