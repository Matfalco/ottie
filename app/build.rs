use tauri_build::{AppManifest, Attributes};

fn main() {
    // The app's own commands. Listing them makes Tauri generate their
    // permissions (`allow-player`…): only pages given those may call them.
    let manifest = AppManifest::new().commands(&[
        "player",
        "set_play_with",
        "choose_player",
        "desktop",
        "set_close_to_tray",
        "set_notify",
        "set_notify_failed",
        "set_language",
        "menu_ready",
        "show_main",
        "quit_app",
    ]);
    if let Err(err) = tauri_build::try_build(Attributes::new().app_manifest(manifest)) {
        panic!("{err:#}");
    }
}
