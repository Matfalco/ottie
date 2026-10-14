//! The app's own settings files, in its config folder: what belongs to this
//! computer's app rather than to the daemon (the player, the tray).

use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tauri::{AppHandle, Manager};

/// The settings saved in `file`; `None` if there are none or they can't be read.
pub fn load<T: DeserializeOwned>(app: &AppHandle, file: &str) -> Option<T> {
    let text = path(app, file).and_then(|path| Ok(fs::read_to_string(path)?));
    serde_json::from_str(&text.ok()?).ok()
}

pub fn save<T: Serialize>(app: &AppHandle, file: &str, value: &T) -> anyhow::Result<()> {
    let path = path(app, file)?;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).context("can't create the app's config folder")?;
    }
    let text = serde_json::to_string_pretty(value)?;
    fs::write(&path, text).with_context(|| format!("can't save {}", path.display()))
}

fn path(app: &AppHandle, file: &str) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join(file))
}
