//! Where "Play" opens a download's play link (docs/decisions/0031): the
//! system's media player, through a playlist file; the browser; or a program
//! the user picked. A setting of this app on this computer, saved in its
//! config folder. The page can switch between these, but only the system's
//! file picker sets the program: a page must never choose what runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, Url};
use tauri_plugin_dialog::DialogExt;

const FILE: &str = "player.json";
/// Playlists written for the system's player are deleted after this long:
/// a player reads them when it starts.
const PLAYLIST_LIFETIME: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayWith {
    /// Whatever opens playlists (`.m3u`) here: VLC once installed, else the
    /// system's own player.
    #[default]
    System,
    Browser,
    /// The program in `Player::program`.
    Program,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub play_with: PlayWith,
    /// The program picked last; kept while another choice is made.
    pub program: Option<String>,
}

/// The player setting, as managed state.
#[derive(Default)]
pub struct Chosen(Mutex<Player>);

impl Chosen {
    /// The saved setting. Without one (or if it can't be read): VLC if it's
    /// installed where its installer puts it, else the system's player. A
    /// setting with no program of its own gets VLC's, ready to choose.
    pub fn load(app: &AppHandle) -> Chosen {
        let saved: Option<Player> = setting_path(app)
            .and_then(|path| Ok(fs::read_to_string(path)?))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        let player = match saved {
            Some(player) if player.program.is_some() => player,
            Some(player) => Player {
                program: find_vlc(),
                ..player
            },
            None => match find_vlc() {
                Some(vlc) => Player {
                    play_with: PlayWith::Program,
                    program: Some(vlc),
                },
                None => Player::default(),
            },
        };
        Chosen(Mutex::new(player))
    }

    fn lock(&self) -> MutexGuard<'_, Player> {
        // Nothing can panic while it's held, so a poisoned lock is still consistent.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The player setting.
#[tauri::command]
pub fn player(chosen: State<'_, Chosen>) -> Player {
    chosen.lock().clone()
}

/// Plays with the system's player, the browser, or the program already picked.
#[tauri::command]
pub fn set_play_with(
    app: AppHandle,
    chosen: State<'_, Chosen>,
    play_with: PlayWith,
) -> Result<Player, String> {
    let mut player = chosen.lock();
    if play_with == PlayWith::Program && player.program.is_none() {
        return Err("no program chosen yet".to_owned());
    }
    player.play_with = play_with;
    save(&app, &player).map_err(|err| format!("{err:#}"))?;
    Ok(player.clone())
}

/// Asks for a program with the system's file picker and plays with it from
/// now on. Cancelled: nothing changes. `title` and `programs` (the name of
/// the file filter on Windows) come in the page's language.
#[tauri::command]
pub async fn choose_player(
    app: AppHandle,
    chosen: State<'_, Chosen>,
    title: String,
    programs: String,
) -> Result<Player, String> {
    let mut dialog = app.dialog().file().set_title(title);
    if cfg!(windows) {
        dialog = dialog.add_filter(programs, &["exe"]);
    }
    // The picker waits for the user: not on this task.
    let picked = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_file())
        .await
        .ok()
        .flatten()
        .and_then(|file| file.into_path().ok());
    let mut player = chosen.lock();
    if let Some(program) = picked {
        let program = program
            .to_str()
            .ok_or("the program's path isn't valid Unicode")?;
        player.program = Some(program.to_owned());
        player.play_with = PlayWith::Program;
        save(&app, &player).map_err(|err| format!("{err:#}"))?;
    }
    Ok(player.clone())
}

/// Opens a play link as the setting says. If the player can't start, the
/// browser plays it instead.
pub fn play(app: &AppHandle, url: &Url) {
    let player = app.state::<Chosen>().lock().clone();
    let opened = match (player.play_with, player.program) {
        (PlayWith::Program, Some(program)) => {
            open::with_detached(url.as_str(), program).map_err(anyhow::Error::from)
        }
        (PlayWith::System, _) => playlist(app, url)
            .and_then(|file| open::that_detached(file).map_err(anyhow::Error::from)),
        _ => open::that_detached(url.as_str()).map_err(anyhow::Error::from),
    };
    if opened.is_err() {
        // Best effort: the user can still copy the link.
        let _ = open::that_detached(url.as_str());
    }
}

/// Writes a playlist that holds only `url`, for the system's player to open.
fn playlist(app: &AppHandle, url: &Url) -> anyhow::Result<PathBuf> {
    let folder = app.path().app_cache_dir()?.join("play");
    fs::create_dir_all(&folder).context("can't create the playlists' folder")?;
    forget_old(&folder);
    let file = folder.join(format!("{}.m3u", playlist_name(url)));
    fs::write(&file, format!("#EXTM3U\n{url}\n")).context("can't write the playlist")?;
    Ok(file)
}

/// From the link's ticket (`/api/v1/stream/<ticket>/<name>`): one per
/// download, and only hex digits.
fn playlist_name(url: &Url) -> String {
    let ticket = url.path_segments().and_then(|mut parts| parts.nth(3));
    match ticket {
        Some(ticket) if ticket.len() >= 16 && ticket.bytes().all(|b| b.is_ascii_hexdigit()) => {
            ticket[..16].to_owned()
        }
        _ => "play".to_owned(),
    }
}

/// Deletes playlists written long ago. Best effort.
fn forget_old(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| SystemTime::now().duration_since(at).ok())
            .is_some_and(|age| age > PLAYLIST_LIFETIME);
        if old && entry.path().extension().is_some_and(|ext| ext == "m3u") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// VLC where its installer puts it: Program Files on Windows, Applications on
/// macOS, the `PATH` elsewhere.
fn find_vlc() -> Option<String> {
    let places: Vec<PathBuf> = if cfg!(windows) {
        ["ProgramFiles", "ProgramFiles(x86)"]
            .into_iter()
            .filter_map(std::env::var_os)
            .map(|folder| PathBuf::from(folder).join(r"VideoLAN\VLC\vlc.exe"))
            .collect()
    } else if cfg!(target_os = "macos") {
        vec![PathBuf::from("/Applications/VLC.app")]
    } else {
        std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths)
                    .map(|folder| folder.join("vlc"))
                    .collect()
            })
            .unwrap_or_default()
    };
    places
        .into_iter()
        .find(|place| place.exists())
        .and_then(|place| place.to_str().map(str::to_owned))
}

fn setting_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join(FILE))
}

fn save(app: &AppHandle, player: &Player) -> anyhow::Result<()> {
    let path = setting_path(app)?;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).context("can't create the app's config folder")?;
    }
    let text = serde_json::to_string_pretty(player)?;
    fs::write(&path, text).with_context(|| format!("can't save {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_playlist_is_named_after_the_ticket() {
        let ticket = "9f2c".repeat(16);
        let url = Url::parse(&format!(
            "http://127.0.0.1:24068/api/v1/stream/{ticket}/a b.mkv"
        ))
        .unwrap();
        assert_eq!(playlist_name(&url), "9f2c9f2c9f2c9f2c");
        let odd = Url::parse("http://127.0.0.1:24068/api/v1/stream/..%2F..%2Fx/a.mkv").unwrap();
        assert_eq!(playlist_name(&odd), "play");
    }

    #[test]
    fn the_setting_reads_back() {
        let player = Player {
            play_with: PlayWith::Program,
            program: Some(r"C:\Program Files\VideoLAN\VLC\vlc.exe".to_owned()),
        };
        let text = serde_json::to_string(&player).unwrap();
        assert!(text.contains(r#""playWith":"program""#), "{text}");
        let back: Player = serde_json::from_str(&text).unwrap();
        assert_eq!(back.play_with, PlayWith::Program);
        assert_eq!(back.program, player.program);
    }
}
