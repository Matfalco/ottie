//! Ottie's desktop app: a window on the daemon's web UI (docs/decisions/0017).
//! Downloads go through the daemon's HTTP API, never through the app.

// Release builds on Windows: no console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod daemon;

use std::sync::{Mutex, PoisonError};

use tauri::ipc::CapabilityBuilder;
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, RunEvent, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::daemon::Started;

const WINDOW: &str = "main";
/// The daemon's play links (docs/decisions/0029).
const PLAY_PATH: &str = "/api/v1/stream/";

/// What the page may do in the app: control its window, since it draws its
/// own title bar (docs/decisions/0019), and open the system folder picker,
/// which only hands back the path the user chose. Only these, and only for
/// the daemon's pages.
const PAGE_PERMISSIONS: [&str; 9] = [
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-close",
    "core:window:allow-is-maximized",
    // Dragging the title bar, and double-clicking it to maximize.
    "core:window:allow-start-dragging",
    "core:window:allow-internal-toggle-maximize",
    // To hear when the window is resized (maximize or restore button).
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    // "Browse…" next to a category's folder.
    "dialog:allow-open",
];

/// The daemon this app started, if it did: stopped when the app quits.
#[derive(Default)]
struct Owned(Mutex<Option<Started>>);

fn main() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        // First, so a second launch only brings this window forward (and
        // can't stop the daemon under it when it quits).
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_dialog::init())
        .manage(Owned::default())
        .setup(|app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = start(&app).await {
                    app.dialog()
                        .message(format!("{err:#}"))
                        .title("Ottie can't start")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                    app.exit(1);
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())?;
    app.run(|app, event| {
        if let RunEvent::Exit = event {
            stop(app);
        }
    });
    Ok(())
}

/// Finds or starts the daemon, then opens the window on it.
async fn start(app: &AppHandle) -> anyhow::Result<()> {
    let connection = daemon::connect(&daemon::sidecar()?).await?;
    *owned(app) = connection.started;
    open_window(app, connection.link)?;
    Ok(())
}

fn open_window(app: &AppHandle, link: Url) -> tauri::Result<()> {
    let daemon = link.origin();
    let player = daemon.clone();
    let page = PAGE_PERMISSIONS.into_iter().fold(
        CapabilityBuilder::new("page")
            // The daemon's address is only known now, hence a runtime capability.
            .remote(daemon.ascii_serialization())
            .local(false)
            .window(WINDOW),
        CapabilityBuilder::permission,
    );
    app.add_capability(page)?;
    WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(link))
        .title("Ottie")
        // No system title bar: the page draws its own.
        .decorations(false)
        .inner_size(960.0, 680.0)
        .min_inner_size(400.0, 320.0)
        // Only the daemon's pages: a link to elsewhere must not turn this
        // window into a browser.
        .on_navigation(move |url| url.origin() == daemon)
        // "Play" opens the daemon's play link in a new tab: here, in the
        // system's browser. No other page opens a window.
        .on_new_window(move |url, _| {
            if url.origin() == player && url.path().starts_with(PLAY_PATH) {
                // Best effort: the user can still copy the link.
                let _ = open::that_detached(url.as_str());
            }
            NewWindowResponse::Deny
        })
        .build()?;
    Ok(())
}

fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW) {
        // Best effort: the user can still find the window themselves.
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// The window is closed: stop the daemon if we started it. One that was
/// already running (a terminal, a service) keeps running.
fn stop(app: &AppHandle) {
    if let Some(started) = owned(app).take() {
        tauri::async_runtime::block_on(started.stop());
    }
}

fn owned(app: &AppHandle) -> std::sync::MutexGuard<'_, Option<Started>> {
    // A panic while holding the lock leaves nothing half-done here.
    app.state::<Owned>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}
