use tauri_build::{AppManifest, Attributes};

fn main() {
    // The app's own commands. Listing them makes Tauri generate their
    // permissions (`allow-player`…): only pages given those may call them.
    let manifest = AppManifest::new().commands(&[
        "player",
        "set_play_with",
        "choose_player",
        "desktop",
        "set_startup",
        "set_close_to_tray",
        "set_notify",
        "set_notify_failed",
        "set_sounds",
        "set_sound",
        "play_sound",
        "set_language",
        "menu_ready",
        "show_main",
        "quit_app",
        "finish",
        "set_finish",
        "set_finish_once",
        "set_countdown",
        "set_countdown_in",
        "set_finish_waits",
        "cancel_finish",
        "finish_now",
        "countdown_ready",
    ]);
    if let Err(err) = tauri_build::try_build(Attributes::new().app_manifest(manifest)) {
        panic!("{err:#}");
    }
}
