//! Judgement.app: menu-bar tray, on-demand window, launch-at-login, single
//! instance, and schema migrations. The store is shared with the CLI through
//! `judgement_core`; this crate never writes SQL itself.

mod cli_link;
mod commands;
mod icons;
mod state;
mod tray;
mod window;

use std::time::Duration;

use judgement_core::store;
use rusqlite::{Connection, OpenFlags};
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime};
use tauri_plugin_autostart::MacosLauncher;

use state::AppState;

/// The window's menu bar (shown while the window is open): standard app, Edit,
/// and Window menus, plus Settings and the setup guide under Help.
fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let app_menu = SubmenuBuilder::new(app, "Judgement")
        .item(&PredefinedMenuItem::about(
            app,
            Some("About Judgement"),
            None,
        )?)
        .separator()
        .item(
            &MenuItemBuilder::with_id("menu_settings", "Settings…")
                .accelerator("CmdOrCtrl+,")
                .build(app)?,
        )
        .separator()
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some("Quit Judgement"))?)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .close_window()
        .build()?;
    let help = SubmenuBuilder::new(app, "Help")
        .item(&MenuItemBuilder::with_id("menu_guide", "Judgement Setup Guide").build(app)?)
        .build()?;
    MenuBuilder::new(app)
        .items(&[&app_menu, &edit, &window, &help])
        .build()
}

/// How often the app checks whether another process (the CLI) committed.
const WATCH_INTERVAL: Duration = Duration::from_millis(750);

/// Refreshes the tray and tells the window to refetch, after any commit.
pub(crate) fn store_changed<R: Runtime>(app: &AppHandle<R>) {
    tray::invalidate(app);
    tray::refresh(app);
    let _ = app.emit("store-changed", ());
}

pub(crate) fn reveal_backups(state: &AppState) {
    let dir = store::backup_dir(&state.dir);
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::process::Command::new("open").arg(&dir).spawn();
}

/// Polls `PRAGMA data_version` on a separate read-only connection; it changes
/// whenever any other connection (the CLI, or this app's writer) commits.
fn watch_store<R: Runtime>(app: AppHandle<R>) {
    let path = app.state::<AppState>().store_path();
    std::thread::spawn(move || {
        let Ok(conn) = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
            return;
        };
        let version = |c: &Connection| {
            c.query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0))
                .ok()
        };
        let mut last = version(&conn);
        loop {
            std::thread::sleep(WATCH_INTERVAL);
            let now = version(&conn);
            if now != last {
                last = now;
                store_changed(&app);
            }
        }
    });
}

fn data_dir<R: Runtime>(app: &AppHandle<R>) -> std::path::PathBuf {
    let tauri_dir = app.path().app_data_dir().ok();
    match store::default_data_dir() {
        Ok(dir) => {
            if std::env::var_os("JUDGEMENT_HOME").is_none() && tauri_dir.as_ref() != Some(&dir) {
                eprintln!("judgement: Tauri app-data dir {tauri_dir:?} differs from the CLI's {dir:?}; using the CLI's");
            }
            dir
        }
        Err(_) => tauri_dir.expect("an app-data directory"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // Must be registered first: a second launch just shows the window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show(app, None)
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(tray::TrayCache::default())
        .menu(app_menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "menu_settings" => window::show(app, Some("settings")),
            "menu_guide" => window::show(app, Some("help")),
            _ => {}
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let state = AppState::open(data_dir(app.handle()));
            let ready = state.failure().is_none();
            app.manage(state);
            if let Some(main) = app.get_webview_window(window::MAIN) {
                window::hide_on_close(&main);
            }
            tray::create(app.handle())?;
            if ready {
                watch_store(app.handle().clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::store_info,
            commands::overview,
            commands::set_participation,
            commands::set_auto_learn,
            commands::set_analyzer,
            commands::list_pending,
            commands::confirm_learning,
            commands::deny_learning,
            commands::delete_learning,
            commands::list_items,
            commands::update_item,
            commands::set_item_status,
            commands::delete_items,
            commands::combine_prefill,
            commands::combine_items,
            commands::gauge,
            commands::backup_summary,
            commands::backup_now,
            commands::reveal_backups,
            commands::get_autostart,
            commands::set_autostart,
            commands::cli_link_status,
            commands::install_cli_link,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Judgement");

    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let RunEvent::Reopen { .. } = event {
            window::show(app, None);
        }
        let _ = (app, event);
    });
}
