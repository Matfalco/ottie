//! What this app remembers on this computer besides the player
//! (docs/decisions/0034): whether closing the window keeps Ottie in the tray,
//! the language the page shows (for the tray's texts), and the last total
//! speed limit (for the tray's "Limit speed").

use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::config;

const FILE: &str = "app.json";
/// A language tag (`en`, `pt-BR`) is never this long.
const MAX_LANGUAGE: usize = 35;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    /// Closing the window hides it; Ottie goes on in the tray.
    pub close_to_tray: bool,
    /// The page's language, as it said last.
    pub language: Option<String>,
    /// The total limit last set (bytes per second), by anyone.
    pub last_limit: Option<u64>,
}

impl Default for Prefs {
    fn default() -> Prefs {
        Prefs {
            close_to_tray: true,
            language: None,
            last_limit: None,
        }
    }
}

/// The part the page sees and changes, in Settings → Desktop app.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Desktop {
    pub close_to_tray: bool,
}

/// The preferences, as managed state.
pub struct Saved(Mutex<Prefs>);

impl Saved {
    pub fn load(app: &AppHandle) -> Saved {
        Saved(Mutex::new(config::load(app, FILE).unwrap_or_default()))
    }

    pub fn get(&self) -> Prefs {
        self.lock().clone()
    }

    /// Applies `change` and saves, unless it changed nothing.
    pub fn change(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut Prefs),
    ) -> anyhow::Result<Prefs> {
        let mut prefs = self.lock();
        let mut next = prefs.clone();
        change(&mut next);
        if next != *prefs {
            config::save(app, FILE, &next)?;
            *prefs = next;
        }
        Ok(prefs.clone())
    }

    fn lock(&self) -> MutexGuard<'_, Prefs> {
        // Nothing can panic while it's held, so a poisoned lock is still consistent.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[tauri::command]
pub fn desktop(saved: State<'_, Saved>) -> Desktop {
    Desktop {
        close_to_tray: saved.get().close_to_tray,
    }
}

#[tauri::command]
pub fn set_close_to_tray(
    app: AppHandle,
    saved: State<'_, Saved>,
    on: bool,
) -> Result<Desktop, String> {
    let prefs = saved
        .change(&app, |prefs| prefs.close_to_tray = on)
        .map_err(|err| format!("{err:#}"))?;
    Ok(Desktop {
        close_to_tray: prefs.close_to_tray,
    })
}

/// The page says which language it shows, each time it loads; the tray
/// follows (on its next refresh).
#[tauri::command]
pub fn set_language(
    app: AppHandle,
    saved: State<'_, Saved>,
    language: String,
) -> Result<(), String> {
    if language.is_empty() || language.len() > MAX_LANGUAGE {
        return Err("not a language tag".to_owned());
    }
    saved
        .change(&app, |prefs| prefs.language = Some(language))
        .map(drop)
        .map_err(|err| format!("{err:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_take_their_defaults() {
        let prefs: Prefs = serde_json::from_str(r#"{"language":"fr"}"#).unwrap();
        assert!(prefs.close_to_tray);
        assert_eq!(prefs.language.as_deref(), Some("fr"));
        assert_eq!(prefs.last_limit, None);
    }
}
