//! Notification sounds (docs/decisions/0038): the system's own, none, one of
//! Ottie's sets (a sound for a finished download and one for a failed
//! download), or a mix of them. On Windows a toast can only play the
//! system's sounds, so the app plays Ottie's itself, with the toast silent.

use std::borrow::Cow;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};

use crate::notify::Kind;

/// A set's two sounds, from `sounds/<id>-finished.ogg` and `-failed.ogg`.
macro_rules! set {
    ($id:literal) => {
        (
            $id,
            include_bytes!(concat!("../sounds/", $id, "-finished.ogg")),
            include_bytes!(concat!("../sounds/", $id, "-failed.ogg")),
        )
    };
}

/// Ottie's sound sets, by ID, in the order the page lists them.
const SETS: [(&str, &[u8], &[u8]); 10] = [
    set!("marimba"),
    set!("log"),
    set!("driftwood"),
    set!("shell"),
    set!("stream"),
    set!("splash"),
    set!("burrow"),
    set!("tide"),
    set!("holt"),
    set!("kelp"),
];

/// The silence between a set's two sounds when both are played to hear them.
const BETWEEN: Duration = Duration::from_millis(500);

/// The IDs of Ottie's sets.
pub fn sets() -> Vec<&'static str> {
    SETS.iter().map(|(id, _, _)| *id).collect()
}

fn set(name: &str) -> Option<&'static str> {
    SETS.iter()
        .find(|(id, _, _)| *id == name)
        .map(|(id, _, _)| *id)
}

/// What a notification sounds like.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(into = "String")]
pub enum Sound {
    /// What the system plays for notifications.
    #[default]
    System,
    Silent,
    /// The sound of one of Ottie's sets for that kind of notification.
    Ottie(&'static str, Kind),
}

impl Sound {
    /// The sound named `name` (`system`, `silent`, `<set>-finished` or
    /// `<set>-failed`), if there's one.
    pub fn named(name: &str) -> Option<Sound> {
        match name {
            "system" => Some(Sound::System),
            "silent" => Some(Sound::Silent),
            _ => {
                let (id, kind) = name.rsplit_once('-')?;
                let kind = match kind {
                    "finished" => Kind::Finished,
                    "failed" => Kind::Failed,
                    _ => return None,
                };
                set(id).map(|id| Sound::Ottie(id, kind))
            }
        }
    }

    /// One of Ottie's sounds: its file.
    fn bytes(self) -> Option<&'static [u8]> {
        let Sound::Ottie(id, kind) = self else {
            return None;
        };
        let (_, finished, failed) = SETS.iter().find(|(set, _, _)| *set == id)?;
        Some(match kind {
            Kind::Finished => finished,
            Kind::Failed => failed,
        })
    }
}

/// As saved: a sound a later version no longer has is the system's. (Not
/// derived: serde would tie `Ottie`'s `&'static str` to the input.)
impl<'de> Deserialize<'de> for Sound {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Sound, D::Error> {
        let name = String::deserialize(input)?;
        Ok(Sound::named(&name).unwrap_or_default())
    }
}

impl From<Sound> for String {
    fn from(sound: Sound) -> String {
        match sound {
            Sound::System => "system".to_owned(),
            Sound::Silent => "silent".to_owned(),
            Sound::Ottie(id, Kind::Finished) => format!("{id}-finished"),
            Sound::Ottie(id, Kind::Failed) => format!("{id}-failed"),
        }
    }
}

/// The sounds chosen for notifications.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(into = "String")]
pub enum Sounds {
    /// The system's sound for both kinds.
    #[default]
    System,
    Silent,
    /// One of Ottie's sets.
    Set(&'static str),
    /// A sound chosen for each kind (`Prefs::sound_finished`, `sound_failed`).
    Custom,
}

impl Sounds {
    /// The choice named `name` (`system`, `silent`, `custom` or a set's ID).
    pub fn named(name: &str) -> Option<Sounds> {
        match name {
            "system" => Some(Sounds::System),
            "silent" => Some(Sounds::Silent),
            "custom" => Some(Sounds::Custom),
            _ => set(name).map(Sounds::Set),
        }
    }
}

/// As saved: a set a later version no longer has is the system's sound.
impl<'de> Deserialize<'de> for Sounds {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Sounds, D::Error> {
        let name = String::deserialize(input)?;
        Ok(Sounds::named(&name).unwrap_or_default())
    }
}

impl From<Sounds> for String {
    fn from(sounds: Sounds) -> String {
        match sounds {
            Sounds::System => "system".to_owned(),
            Sounds::Silent => "silent".to_owned(),
            Sounds::Set(id) => id.to_owned(),
            Sounds::Custom => "custom".to_owned(),
        }
    }
}

/// Plays one of Ottie's sounds. Best effort: without a sound device,
/// nothing is heard.
pub fn play(sound: Sound) {
    if let Some(bytes) = sound.bytes() {
        output(move || vec![Cow::Borrowed(bytes)]);
    }
}

/// Plays a sound to hear it before choosing it: one of Ottie's, or the
/// system's where it's a file (Windows). False if there's nothing to play.
pub fn preview(sound: Sound) -> bool {
    match sound {
        Sound::Ottie(..) => {
            play(sound);
            true
        }
        Sound::System => match system_file() {
            Some(path) => {
                output(move || {
                    std::fs::read(path)
                        .ok()
                        .map(Cow::Owned)
                        .into_iter()
                        .collect()
                });
                true
            }
            None => false,
        },
        Sound::Silent => false,
    }
}

/// Plays a set's two sounds, one after the other, to hear it before
/// choosing it.
pub fn preview_set(id: &'static str) {
    let both = [Kind::Finished, Kind::Failed].map(|kind| Sound::Ottie(id, kind).bytes());
    output(move || both.into_iter().flatten().map(Cow::Borrowed).collect());
}

/// Whether the system's notification sound can be heard before choosing it.
pub fn system_previews() -> bool {
    system_file().is_some()
}

/// Plays what `load` gives on the default output, one after the other, on
/// a thread of its own.
fn output(load: impl FnOnce() -> Vec<Cow<'static, [u8]>> + Send + 'static) {
    let _ = std::thread::Builder::new()
        .name("sound".into())
        .spawn(move || {
            let sounds = load();
            if sounds.is_empty() {
                return;
            }
            // Opened for each sound: the default output may have changed
            // (headphones) since the last one.
            let Ok(mut sink) = rodio::DeviceSinkBuilder::open_default_sink() else {
                return;
            };
            sink.log_on_drop(false);
            for (index, bytes) in sounds.into_iter().enumerate() {
                if index > 0 {
                    std::thread::sleep(BETWEEN);
                }
                if let Ok(player) = rodio::play(sink.mixer(), Cursor::new(bytes)) {
                    player.sleep_until_end();
                }
            }
        });
}

/// Windows: the file played for notifications, as set in the Control
/// Panel's Sounds (none when set to "(None)"). It's what a toast's
/// `Default` sound plays.
#[cfg(windows)]
fn system_file() -> Option<PathBuf> {
    let path = windows_registry::CURRENT_USER
        .open(r"AppEvents\Schemes\Apps\.Default\Notification.Default\.Current")
        .and_then(|key| key.get_string(""))
        .ok()?;
    let path = PathBuf::from(expand(&path));
    path.is_file().then_some(path)
}

/// Elsewhere the notification server chooses, if it plays anything.
#[cfg(not(windows))]
fn system_file() -> Option<PathBuf> {
    None
}

/// `%NAME%` replaced by that environment variable (`%SystemRoot%\Media\…`);
/// unknown ones stay as they are.
#[cfg_attr(not(windows), allow(dead_code))]
fn expand(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = &after[..end];
        match std::env::var(name) {
            Ok(value) if !name.is_empty() => out.push_str(&value),
            _ => out.push_str(&rest[start..start + end + 2]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sound_is_saved_by_its_name() {
        let sounds = [
            Sound::System,
            Sound::Silent,
            Sound::Ottie("kelp", Kind::Failed),
        ];
        let json = serde_json::to_string(&sounds).unwrap();
        assert_eq!(json, r#"["system","silent","kelp-failed"]"#);
        assert_eq!(serde_json::from_str::<Vec<Sound>>(&json).unwrap(), sounds);
        assert_eq!(
            serde_json::from_str::<Sound>(r#""gone-failed""#).unwrap(),
            Sound::System
        );
        assert_eq!(Sound::named("kelp"), None);
        assert_eq!(Sound::named("kelp-lost"), None);
    }

    #[test]
    fn a_choice_of_sounds_is_saved_by_its_name() {
        let choices = [
            Sounds::System,
            Sounds::Silent,
            Sounds::Set("holt"),
            Sounds::Custom,
        ];
        let json = serde_json::to_string(&choices).unwrap();
        assert_eq!(json, r#"["system","silent","holt","custom"]"#);
        assert_eq!(serde_json::from_str::<Vec<Sounds>>(&json).unwrap(), choices);
        assert_eq!(
            serde_json::from_str::<Sounds>(r#""gone""#).unwrap(),
            Sounds::System
        );
    }

    #[test]
    fn ottie_s_sounds_decode() {
        for (id, finished, failed) in SETS {
            for bytes in [finished, failed] {
                assert!(rodio::Decoder::new(Cursor::new(bytes)).is_ok(), "{id}");
            }
        }
    }

    #[test]
    fn environment_variables_in_a_path_are_expanded() {
        let path = std::env::var("PATH").unwrap();
        assert_eq!(expand(r"%PATH%\a.wav"), format!(r"{path}\a.wav"));
        assert_eq!(expand("%NOT_OTTIE_S%/50%"), "%NOT_OTTIE_S%/50%");
        assert_eq!(expand("%%a"), "%%a");
    }
}
