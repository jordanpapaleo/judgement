//! The menu-bar item: one switch for agents, a door to the window, and the
//! count of learnings to review. It reflects the store only after writes commit.

use std::sync::Mutex;

use judgement_core::model::Participation;
use judgement_core::ops;
use tauri::menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIcon as TauriTray, TrayIconBuilder};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;

use crate::icons::TrayIcon;
use crate::state::{AppState, TrayModel};
use crate::window;

const TRAY_ID: &str = "judgement";

/// The last model the tray was built from, so polling doesn't rebuild an unchanged menu.
#[derive(Default)]
pub struct TrayCache(Mutex<Option<TrayModel>>);

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Judgement")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .build(app)?;
    refresh(app);
    Ok(())
}

fn autostart_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Rebuilds the tray from the committed store state, if it changed.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let model = app.state::<AppState>().tray_model(autostart_enabled(app));
    let cache = app.state::<TrayCache>();
    {
        let mut last = cache.0.lock().expect("tray cache");
        if last.as_ref() == Some(&model) {
            return;
        }
        *last = Some(model.clone());
    }
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Err(e) = apply(app, &tray, &model) {
        eprintln!("judgement: tray update failed: {e}");
    }
}

fn apply<R: Runtime>(
    app: &AppHandle<R>,
    tray: &TauriTray<R>,
    model: &TrayModel,
) -> tauri::Result<()> {
    let (icon, menu) = match model {
        TrayModel::Ready {
            participation,
            pending,
            autostart,
        } => (
            icon_for(*participation),
            ready_menu(app, *participation, *pending, *autostart)?,
        ),
        TrayModel::Failed => (TrayIcon::Unavailable, failed_menu(app)?),
    };
    tray.set_icon_with_as_template(Some(icon.image()), icon.is_template())?;
    tray.set_menu(Some(menu))?;
    Ok(())
}

fn icon_for(p: Participation) -> TrayIcon {
    match p {
        Participation::Active => TrayIcon::Active,
        Participation::Paused => TrayIcon::Paused,
    }
}

fn ready_menu<R: Runtime>(
    app: &AppHandle<R>,
    participation: Participation,
    pending: u32,
    autostart: bool,
) -> tauri::Result<Menu<R>> {
    let paused = participation == Participation::Paused;
    let header = if paused {
        "Judgement — Paused"
    } else {
        "Judgement — Active"
    };
    let mut menu = MenuBuilder::new(app).item(
        &MenuItemBuilder::with_id("header", header)
            .enabled(false)
            .build(app)?,
    );
    if paused {
        menu = menu.item(
            &MenuItemBuilder::with_id("note", "Agents get no guidance and can't propose.")
                .enabled(false)
                .build(app)?,
        );
    }
    let toggle = if paused {
        "Resume Judgement"
    } else {
        "Pause Judgement"
    };
    menu.separator()
        .item(&MenuItemBuilder::with_id("toggle", toggle).build(app)?)
        .item(&MenuItemBuilder::with_id("open", "Open Judgement…").build(app)?)
        .item(
            &MenuItemBuilder::with_id("learnings", format!("Learnings: {pending} to review"))
                .build(app)?,
        )
        .separator()
        .item(
            &CheckMenuItemBuilder::with_id("autostart", "Launch at Login")
                .checked(autostart)
                .build(app)?,
        )
        .item(&MenuItemBuilder::with_id("quit", "Quit Judgement").build(app)?)
        .build()
}

fn failed_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    MenuBuilder::new(app)
        .item(
            &MenuItemBuilder::with_id("header", "Judgement — Can't Open Store")
                .enabled(false)
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                "note",
                "The store wasn't touched. Agents continue without guidance.",
            )
            .enabled(false)
            .build(app)?,
        )
        .separator()
        .item(&MenuItemBuilder::with_id("details", "Show Details…").build(app)?)
        .item(&MenuItemBuilder::with_id("reveal", "Reveal Backups in Finder").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id("quit", "Quit Judgement").build(app)?)
        .build()
}

fn on_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "toggle" => {
            let state = app.state::<AppState>();
            let result = state.with(|conn| {
                let next = match ops::settings(conn)?.participation {
                    Participation::Active => Participation::Paused,
                    Participation::Paused => Participation::Active,
                };
                ops::set_participation(conn, next)
            });
            if let Err(e) = result {
                eprintln!("judgement: pause toggle failed: {e}");
            }
            crate::store_changed(app);
        }
        "open" | "details" => window::show(app, None),
        "learnings" => window::show(app, Some("learnings")),
        "autostart" => {
            let launcher = app.autolaunch();
            let result = if autostart_enabled(app) {
                launcher.disable()
            } else {
                launcher.enable()
            };
            if let Err(e) = result {
                eprintln!("judgement: launch at login failed: {e}");
            }
            crate::store_changed(app);
        }
        "reveal" => crate::reveal_backups(&app.state::<AppState>()),
        "quit" => app.exit(0),
        _ => {}
    }
}

/// Forgets the cached model so the next `refresh` rebuilds (e.g. after a
/// failed Launch at Login toggle left the check mark out of sync).
pub fn invalidate<R: Runtime>(app: &AppHandle<R>) {
    *app.state::<TrayCache>().0.lock().expect("tray cache") = None;
}
