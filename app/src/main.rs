//! Ottie's desktop app: a window on the daemon's web UI (docs/decisions/0017).
//! Downloads go through the daemon's HTTP API, never through the app.

// Release builds on Windows: no console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod client;
mod clipboard;
mod config;
mod daemon;
mod files;
mod finish;
mod notify;
mod player;
mod popup;
mod power;
mod prefs;
mod sound;
mod status;
mod texts;
mod tray;

use std::sync::{Arc, Mutex, PoisonError};

use tauri::ipc::CapabilityBuilder;
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, RunEvent, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::client::Daemon;
use crate::daemon::Started;

const WINDOW: &str = "main";
/// The daemon's play links (docs/decisions/0029).
const PLAY_PATH: &str = "/api/v1/stream/";
/// Ottie's own pages on the web (e.g. to help translate it), which the page
/// may open in the system's browser.
const PROJECT: &str = "https://github.com/Matfalco/ottie/";

/// What the page may do in the app: control its window, since it draws its
/// own title bar (docs/decisions/0019), open the system folder picker, which
/// only hands back the path the user chose, choose how to play
/// (docs/decisions/0031), set what closing does, when to notify
/// (docs/decisions/0035), what happens at sign-in (0039) and the tray's
/// language, in Ottie's tray menu, open the window, quit, and hide the
/// menu (docs/decisions/0034), choose what happens when everything has
/// finished, or cancel it (0040), and watch the clipboard, then offer what
/// it caught (0043), and open a download's file or folder (0045), named by
/// its ID. Only these, and only for the daemon's pages.
const PAGE_PERMISSIONS: [&str; 42] = [
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
    // "Play with" in Settings: this app's own commands (see build.rs).
    "allow-player",
    "allow-set-play-with",
    "allow-choose-player",
    // Settings → Desktop app and Notifications (and the first-start setup's
    // sign-in choice), and the page's language for the tray.
    "allow-desktop",
    "allow-set-startup",
    "allow-set-close-to-tray",
    "allow-set-watch-clipboard",
    "allow-set-copied",
    "allow-set-notify",
    "allow-set-notify-failed",
    "allow-set-sounds",
    "allow-set-sound",
    "allow-play-sound",
    "allow-set-language",
    // Ottie's tray menu (`popup.rs`).
    "core:window:allow-hide",
    "allow-menu-ready",
    "allow-show-main",
    "allow-quit-app",
    // When everything has finished (`finish.rs`): Settings, the Downloads
    // page, the tray menu and the countdown's window.
    "allow-finish",
    "allow-set-finish",
    "allow-set-finish-once",
    "allow-set-countdown",
    "allow-set-countdown-in",
    "allow-set-finish-waits",
    "allow-cancel-finish",
    "allow-finish-now",
    "allow-countdown-ready",
    // The window offering links copied elsewhere (`clipboard.rs`).
    "allow-caught",
    "allow-caught-ready",
    "allow-caught-done",
    "allow-caught-more",
    // Open and Show in folder (`files.rs`).
    "allow-open-file",
    "allow-show-in-folder",
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
        .plugin(tauri_plugin_notification::init())
        .manage(Owned::default())
        .invoke_handler(tauri::generate_handler![
            player::player,
            player::set_play_with,
            player::choose_player,
            prefs::desktop,
            prefs::set_startup,
            prefs::set_close_to_tray,
            prefs::set_watch_clipboard,
            prefs::set_copied,
            prefs::set_notify,
            prefs::set_notify_failed,
            prefs::set_sounds,
            prefs::set_sound,
            prefs::play_sound,
            prefs::set_language,
            popup::menu_ready,
            popup::show_main,
            popup::quit_app,
            finish::finish,
            finish::set_finish,
            finish::set_finish_once,
            finish::set_countdown,
            finish::set_countdown_in,
            finish::set_finish_waits,
            finish::cancel_finish,
            finish::finish_now,
            finish::countdown_ready,
            clipboard::caught,
            clipboard::caught_ready,
            clipboard::caught_done,
            clipboard::caught_more,
            files::open_file,
            files::show_in_folder
        ])
        .setup(|app| {
            app.manage(player::Chosen::load(app.handle()));
            let saved = prefs::Saved::load(app.handle());
            let startup = saved.get().startup;
            app.manage(saved);
            autostart::refresh(app.handle());
            let at_sign_in = autostart::at_sign_in();
            let app = app.handle().clone();
            if at_sign_in && startup == autostart::Mode::Daemon {
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = start_alone(&app).await {
                        cant_start(&app, &err);
                    }
                    app.exit(0);
                });
                return Ok(());
            }
            notify::listen(&app);
            tauri::async_runtime::spawn(async move {
                // At sign-in, Ottie starts in the tray.
                if let Err(err) = start(&app, !at_sign_in).await {
                    cant_start(&app, &err);
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

/// Finds or starts the daemon, then opens the window on it (shown or not)
/// and puts Ottie in the tray.
async fn start(app: &AppHandle, shown: bool) -> anyhow::Result<()> {
    let connection = daemon::connect(&daemon::sidecar()?).await?;
    *owned(app) = connection.started;
    let daemon = Arc::new(Daemon::new(&connection.link)?);
    app.manage(daemon.clone());
    app.manage(popup::Popup::new(&connection.link));
    app.manage(finish::Finisher::new(&connection.link));
    app.manage(clipboard::Catcher::new(&connection.link, daemon.clone()));
    open_window(app, connection.link, shown)?;
    tray::start(app, daemon)?;
    clipboard::start(app);
    Ok(())
}

/// At sign-in, set to start only the daemon (docs/decisions/0039): it runs
/// on its own, its log in the app's log folder, and the app quits. Opened
/// later, the app finds it, and quitting the app leaves it running.
async fn start_alone(app: &AppHandle) -> anyhow::Result<()> {
    let log = app.path().app_log_dir()?.join("daemon.log");
    daemon::start_alone(&daemon::sidecar()?, &log).await
}

fn cant_start(app: &AppHandle, err: &anyhow::Error) {
    app.dialog()
        .message(format!("{err:#}"))
        .title("Ottie can't start")
        .kind(MessageDialogKind::Error)
        .blocking_show();
}

fn open_window(app: &AppHandle, link: Url, shown: bool) -> tauri::Result<()> {
    let daemon = link.origin();
    let links = daemon.clone();
    let handle = app.clone();
    let page = PAGE_PERMISSIONS.into_iter().fold(
        CapabilityBuilder::new("page")
            // The daemon's address is only known now, hence a runtime capability.
            .remote(daemon.ascii_serialization())
            .local(false)
            .windows([WINDOW, popup::LABEL, finish::LABEL, clipboard::LABEL]),
        CapabilityBuilder::permission,
    );
    app.add_capability(page)?;
    let window = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::External(link))
        .title("Ottie")
        // No system title bar: the page draws its own.
        .decorations(false)
        .inner_size(960.0, 680.0)
        // Room for the title bar folded as far as it goes (tabs as icons)
        // beside the window's buttons, and for a page under it.
        .min_inner_size(720.0, 480.0)
        .visible(shown)
        // Only the daemon's pages: a link to elsewhere must not turn this
        // window into a browser.
        .on_navigation(move |url| url.origin() == daemon)
        // "Play" opens the daemon's play link in a new tab: here, in the
        // player chosen in Settings. Ottie's own pages on the web open in
        // the system's browser. No other page opens a window.
        .on_new_window(move |url, _| {
            if url.origin() == links && url.path().starts_with(PLAY_PATH) {
                player::play(&handle, &url);
            } else if url.as_str().starts_with(PROJECT) {
                // Best effort: without a browser, the link does nothing.
                let _ = open::that_detached(url.as_str());
            }
            NewWindowResponse::Deny
        })
        .build()?;
    let hidden = window.clone();
    let handle = app.clone();
    window.on_window_event(move |event| {
        // Closing hides the window, unless Settings say it quits
        // (docs/decisions/0034). The tray's "Quit Ottie" always quits.
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if handle.state::<prefs::Saved>().get().close_to_tray {
                // Best effort: if it can't hide, it stays open.
                let _ = hidden.hide();
            } else {
                // Quits even with the tray's menu window still there, hidden.
                handle.exit(0);
            }
        }
    });
    Ok(())
}

/// Brings the window forward: from the tray, a second launch, or the
/// clipboard's More….
fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WINDOW) {
        // Best effort: the user can still find the window themselves.
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// The app quits: stop the daemon if we started it. One that was already
/// running (a terminal, a service) keeps running.
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
