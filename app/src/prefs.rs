//! What this app remembers on this computer besides the player
//! (docs/decisions/0034): whether closing the window keeps Ottie in the tray,
//! when to notify (docs/decisions/0035) and with which sounds (0038), what
//! starting at sign-in starts (0039), what happens when everything has
//! finished (0040), whether it watches the clipboard and what a copied
//! link does (0043), whether this computer's Ottie always starts when the
//! window shows another one's (0053), the language the page shows (for the
//! tray's texts and notifications), and the last total speed limit (for the
//! tray's "Limit speed").

use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::autostart::{self, Mode, Startup};
use crate::clipboard::Copied;
use crate::config;
use crate::finish::{self, CountdownIn, Finish};
use crate::notify::Kind;
use crate::sound::{self, Sound, Sounds};

const FILE: &str = "app.json";
/// What the pages hear when the part they see changes, with a `Desktop`.
const EVENT: &str = "desktop";
/// A language tag (`en`, `pt-BR`) is never this long.
const MAX_LANGUAGE: usize = 35;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    /// Closing the window hides it; Ottie goes on in the tray.
    pub close_to_tray: bool,
    /// When finished downloads are notified.
    pub notify: Notify,
    /// A failed download is notified (after its last retry).
    pub notify_failed: bool,
    /// The notifications' sounds: the system's, none, a set of Ottie's, or
    /// the two below.
    pub sounds: Sounds,
    /// With `Sounds::Custom`, a finished download's sound…
    pub sound_finished: Sound,
    /// …and a failed one's.
    pub sound_failed: Sound,
    /// What the system's entry starts at sign-in, when there is one.
    pub startup: Mode,
    /// When everything has finished, every time…
    pub finish: Finish,
    /// …or this time only, instead (cleared once its time has come).
    pub finish_once: Option<Finish>,
    /// Seconds to cancel before the computer sleeps, hibernates or shuts down.
    pub countdown: u32,
    pub countdown_in: CountdownIn,
    /// Downloads waiting for their start time or held by a slot count as
    /// not finished.
    pub finish_waits_for_scheduled: bool,
    /// Links copied elsewhere are offered or added (opt-in).
    pub watch_clipboard: bool,
    pub copied: Copied,
    /// Whether this computer's Ottie starts with the app when the window
    /// shows another computer's.
    pub local_daemon: LocalDaemon,
    /// The page's language, as it said last.
    pub language: Option<String>,
    /// The total limit last set (bytes per second), by anyone.
    pub last_limit: Option<u64>,
}

impl Default for Prefs {
    fn default() -> Prefs {
        Prefs {
            close_to_tray: true,
            notify: Notify::default(),
            notify_failed: true,
            sounds: Sounds::System,
            sound_finished: Sound::System,
            sound_failed: Sound::System,
            startup: Mode::Tray,
            finish: Finish::Nothing,
            finish_once: None,
            countdown: finish::DEFAULT_COUNTDOWN,
            countdown_in: CountdownIn::Window,
            finish_waits_for_scheduled: true,
            watch_clipboard: false,
            copied: Copied::Ask,
            local_daemon: LocalDaemon::Always,
            language: None,
            last_limit: None,
        }
    }
}

impl Prefs {
    /// What a notification of this kind sounds like.
    pub fn sound(&self, kind: Kind) -> Sound {
        match (self.sounds, kind) {
            (Sounds::System, _) => Sound::System,
            (Sounds::Silent, _) => Sound::Silent,
            (Sounds::Set(id), kind) => Sound::Ottie(id, kind),
            (Sounds::Custom, Kind::Finished) => self.sound_finished,
            (Sounds::Custom, Kind::Failed) => self.sound_failed,
        }
    }
}

/// When finished downloads are notified.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Notify {
    /// As each one finishes.
    Each,
    /// Once nothing is downloading or waiting any more.
    #[default]
    All,
    Never,
}

/// When this computer's Ottie runs, with the window on another computer's
/// (docs/decisions/0053).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalDaemon {
    /// It starts with the app whatever the window shows, so its downloads
    /// carry on.
    #[default]
    Always,
    /// Only once the window shows it (then until the app quits).
    WhenShown,
}

/// The part the page sees and changes, in Settings → Desktop app,
/// Clipboard and Notifications, and the tray menu.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Desktop {
    /// What happens at sign-in: the system's entry, then `Prefs::startup`.
    pub startup: Startup,
    pub close_to_tray: bool,
    pub watch_clipboard: bool,
    pub copied: Copied,
    pub local_daemon: LocalDaemon,
    pub notify: Notify,
    pub notify_failed: bool,
    pub sounds: Sounds,
    pub sound_finished: Sound,
    pub sound_failed: Sound,
    /// Ottie's sound sets, to choose from.
    pub sets: Vec<&'static str>,
    /// The system's sound can be played to hear it (Windows).
    pub system_sound: bool,
}

impl Desktop {
    fn new(app: &AppHandle, prefs: Prefs) -> Desktop {
        Desktop {
            startup: autostart::current(app, prefs.startup),
            close_to_tray: prefs.close_to_tray,
            watch_clipboard: prefs.watch_clipboard,
            copied: prefs.copied,
            local_daemon: prefs.local_daemon,
            notify: prefs.notify,
            notify_failed: prefs.notify_failed,
            sounds: prefs.sounds,
            sound_finished: prefs.sound_finished,
            sound_failed: prefs.sound_failed,
            sets: sound::sets(),
            system_sound: sound::system_previews(),
        }
    }
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
pub fn desktop(app: AppHandle, saved: State<'_, Saved>) -> Desktop {
    Desktop::new(&app, saved.get())
}

/// What happens at sign-in: the system's entry is added or removed, and
/// what it starts is saved.
#[tauri::command]
pub fn set_startup(
    app: AppHandle,
    saved: State<'_, Saved>,
    startup: Startup,
) -> Result<Desktop, String> {
    autostart::set(&app, startup != Startup::Off).map_err(|err| format!("{err:#}"))?;
    change_desktop(&app, &saved, |prefs| match startup {
        Startup::Off => {}
        Startup::Tray => prefs.startup = Mode::Tray,
        Startup::Daemon => prefs.startup = Mode::Daemon,
    })
}

#[tauri::command]
pub fn set_close_to_tray(
    app: AppHandle,
    saved: State<'_, Saved>,
    on: bool,
) -> Result<Desktop, String> {
    change_desktop(&app, &saved, |prefs| prefs.close_to_tray = on)
}

#[tauri::command]
pub fn set_watch_clipboard(
    app: AppHandle,
    saved: State<'_, Saved>,
    on: bool,
) -> Result<Desktop, String> {
    change_desktop(&app, &saved, |prefs| prefs.watch_clipboard = on)
}

#[tauri::command]
pub fn set_copied(
    app: AppHandle,
    saved: State<'_, Saved>,
    copied: Copied,
) -> Result<Desktop, String> {
    change_desktop(&app, &saved, |prefs| prefs.copied = copied)
}

/// From the next start: a running Ottie isn't stopped.
#[tauri::command]
pub fn set_local_daemon(
    app: AppHandle,
    saved: State<'_, Saved>,
    local: LocalDaemon,
) -> Result<Desktop, String> {
    change_desktop(&app, &saved, |prefs| prefs.local_daemon = local)
}

#[tauri::command]
pub fn set_notify(
    app: AppHandle,
    saved: State<'_, Saved>,
    notify: Notify,
) -> Result<Desktop, String> {
    change_desktop(&app, &saved, |prefs| prefs.notify = notify)
}

#[tauri::command]
pub fn set_notify_failed(
    app: AppHandle,
    saved: State<'_, Saved>,
    on: bool,
) -> Result<Desktop, String> {
    change_desktop(&app, &saved, |prefs| prefs.notify_failed = on)
}

/// The notifications' sounds; `sounds` is a name: `system`, `silent`, a
/// set's ID or `custom`.
#[tauri::command]
pub fn set_sounds(
    app: AppHandle,
    saved: State<'_, Saved>,
    sounds: String,
) -> Result<Desktop, String> {
    let sounds = Sounds::named(&sounds).ok_or("no such sounds")?;
    change_desktop(&app, &saved, |prefs| {
        // Mixing starts from what was heard until now.
        if sounds == Sounds::Custom && prefs.sounds != Sounds::Custom {
            prefs.sound_finished = prefs.sound(Kind::Finished);
            prefs.sound_failed = prefs.sound(Kind::Failed);
        }
        prefs.sounds = sounds;
    })
}

/// With `Sounds::Custom`: the sound of the notifications of this kind;
/// `sound` is a name: `system`, `silent` or a set's (`kelp-failed`).
#[tauri::command]
pub fn set_sound(
    app: AppHandle,
    saved: State<'_, Saved>,
    kind: Kind,
    sound: String,
) -> Result<Desktop, String> {
    let sound = Sound::named(&sound).ok_or("no such sound")?;
    change_desktop(&app, &saved, |prefs| match kind {
        Kind::Finished => prefs.sound_finished = sound,
        Kind::Failed => prefs.sound_failed = sound,
    })
}

/// Plays a sound to hear it before choosing it: one of Ottie's, a set's
/// two (by the set's ID), or the system's where it can.
#[tauri::command]
pub fn play_sound(sound: String) -> Result<(), String> {
    let played = match (Sound::named(&sound), Sounds::named(&sound)) {
        (Some(sound), _) => sound::preview(sound),
        (None, Some(Sounds::Set(id))) => {
            sound::preview_set(id);
            true
        }
        _ => return Err("no such sound".to_owned()),
    };
    if played {
        Ok(())
    } else {
        Err("nothing to play".to_owned())
    }
}

/// Every window hears of it: the tray menu and Settings show the same.
pub fn change_desktop(
    app: &AppHandle,
    saved: &Saved,
    change: impl FnOnce(&mut Prefs),
) -> Result<Desktop, String> {
    let prefs = saved
        .change(app, change)
        .map_err(|err| format!("{err:#}"))?;
    let desktop = Desktop::new(app, prefs);
    // Best effort: a page that misses it shows the change when it asks again.
    let _ = app.emit(EVENT, &desktop);
    Ok(desktop)
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
        assert_eq!(prefs.notify, Notify::All);
        assert!(prefs.notify_failed);
        assert_eq!(prefs.sounds, Sounds::System);
        assert_eq!(prefs.startup, Mode::Tray);
        assert_eq!((prefs.finish, prefs.finish_once), (Finish::Nothing, None));
        assert_eq!(prefs.countdown, 60);
        assert!(prefs.finish_waits_for_scheduled);
        assert!(!prefs.watch_clipboard);
        assert_eq!(prefs.copied, Copied::Ask);
        assert_eq!(prefs.language.as_deref(), Some("fr"));
        assert_eq!(prefs.last_limit, None);
    }

    #[test]
    fn a_set_gives_both_sounds_unless_mixed() {
        let mut prefs = Prefs {
            sounds: Sounds::Set("kelp"),
            sound_finished: Sound::Silent,
            ..Prefs::default()
        };
        assert_eq!(
            prefs.sound(Kind::Failed),
            Sound::Ottie("kelp", Kind::Failed)
        );
        prefs.sounds = Sounds::Custom;
        assert_eq!(prefs.sound(Kind::Finished), Sound::Silent);
        assert_eq!(prefs.sound(Kind::Failed), Sound::System);
    }
}
