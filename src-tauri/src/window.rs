//! The on-demand window. Judgement runs as an accessory app (no Dock icon);
//! while the window is open it becomes a regular app so it can be focused and
//! Cmd-Tabbed to. Closing the window hides it; the app keeps running.

use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow, WindowEvent};

pub const MAIN: &str = "main";

fn set_regular<R: Runtime>(app: &AppHandle<R>, regular: bool) {
    #[cfg(target_os = "macos")]
    {
        let policy = if regular {
            tauri::ActivationPolicy::Regular
        } else {
            tauri::ActivationPolicy::Accessory
        };
        let _ = app.set_activation_policy(policy);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, regular);
}

/// Shows and focuses the window, optionally switching it to a screen.
pub fn show<R: Runtime>(app: &AppHandle<R>, screen: Option<&str>) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    set_regular(app, true);
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    if let Some(screen) = screen {
        let _ = window.emit("navigate", screen);
    }
}

/// Close hides instead of quitting, and drops back to accessory (no Dock icon).
pub fn hide_on_close<R: Runtime>(window: &WebviewWindow<R>) {
    let handle = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = handle.hide();
            set_regular(handle.app_handle(), false);
        }
    });
}
