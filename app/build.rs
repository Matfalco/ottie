use tauri_build::{AppManifest, Attributes};

fn main() {
    // The app's own commands. Listing them makes Tauri generate their
    // permissions (`allow-player`…): only pages given those may call them.
    let manifest = AppManifest::new().commands(&["player", "set_play_with", "choose_player"]);
    if let Err(err) = tauri_build::try_build(Attributes::new().app_manifest(manifest)) {
        panic!("{err:#}");
    }
}
