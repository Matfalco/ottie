//! Opening a download's file, or its folder with the file selected
//! (docs/decisions/0045). The page names a download by its ID, never a path:
//! the app asks the daemon where its file is, so a page can only open what
//! Ottie downloaded. A program asks first, in a dialog of the app's own,
//! which no page can answer in the user's place.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::client::{Daemon, Placed, is_job_id};
use crate::prefs;
use crate::texts::Texts;

/// Appended to a file's name while it downloads (`PART_SUFFIX` in the engine).
const PART_SUFFIX: &str = ".part.ottie";

/// Files that run something when opened: programs, installers, scripts,
/// shortcuts. Whatever the system, since a file may come from another one
/// (and a `.py` runs on Windows once Python is installed).
#[rustfmt::skip]
const PROGRAMS: &[&str] = &[
    // Windows
    "exe", "com", "scr", "pif", "cpl", "msi", "msp", "mst", "msix", "msixbundle", "appx",
    "appxbundle", "appinstaller", "application", "appref-ms", "gadget", "bat", "cmd", "ps1",
    "psm1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "wsc", "hta", "msc", "lnk", "url", "scf",
    "reg", "inf", "chm", "jar", "py", "pyw",
    // macOS
    "app", "pkg", "mpkg", "command", "tool", "workflow", "scpt", "applescript",
    // Linux
    "sh", "bash", "run", "bin", "appimage", "desktop", "deb", "rpm", "flatpakref", "snap",
];

/// Why it couldn't open: a code the page puts in words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Refusal {
    /// The file (or for the folder, the folder too) isn't there any more.
    Gone,
    /// The daemon couldn't say where it is, it isn't finished, or the system
    /// couldn't open it.
    Failed,
}

/// Opens a finished download's file with the system's app for its type.
/// A program asks first; declined, nothing happens.
#[tauri::command]
pub async fn open_file(app: AppHandle, id: String) -> Result<(), Refusal> {
    let placed = placed(&app, &id).await?;
    if placed.state != "completed" {
        return Err(Refusal::Failed);
    }
    let file = file_of(&placed, false).ok_or(Refusal::Failed)?;
    if !file.is_file() {
        return Err(Refusal::Gone);
    }
    if is_program(&file) && !confirmed(&app, &file).await {
        return Ok(());
    }
    open::that_detached(&file).map_err(|_| Refusal::Failed)
}

/// Opens the download's folder with its file selected: the finished file,
/// else the one still downloading, else just the folder.
#[tauri::command]
pub async fn show_in_folder(app: AppHandle, id: String) -> Result<(), Refusal> {
    let placed = placed(&app, &id).await?;
    let unfinished = placed.state != "completed";
    if let Some(file) = file_of(&placed, unfinished).filter(|file| file.is_file()) {
        return tauri_plugin_opener::reveal_item_in_dir(file).map_err(|_| Refusal::Failed);
    }
    let folder = Path::new(&placed.dest_dir);
    if !folder.is_absolute() || !folder.is_dir() {
        return Err(Refusal::Gone);
    }
    open::that_detached(folder).map_err(|_| Refusal::Failed)
}

/// Where the daemon says the download is.
async fn placed(app: &AppHandle, id: &str) -> Result<Placed, Refusal> {
    if !is_job_id(id) {
        return Err(Refusal::Failed);
    }
    // Only missing before the window opens, so before any page can ask.
    let daemon = app.try_state::<Arc<Daemon>>().ok_or(Refusal::Failed)?;
    daemon.placed(id).await.map_err(|_| Refusal::Failed)
}

/// The download's file (`unfinished`: its `.part.ottie`), if its folder is a
/// full path and its name only a name: the daemon makes them so, and this
/// keeps it from pointing anywhere else.
fn file_of(placed: &Placed, unfinished: bool) -> Option<PathBuf> {
    let folder = Path::new(&placed.dest_dir);
    let name = placed.file_name.as_deref()?;
    let mut parts = Path::new(name).components();
    let plain = matches!(parts.next(), Some(Component::Normal(part)) if part == OsStr::new(name))
        && parts.next().is_none()
        // On Windows, `a:b` would be a stream inside the file `a`.
        && !(cfg!(windows) && name.contains(':'));
    if !folder.is_absolute() || !plain {
        return None;
    }
    Some(if unfinished {
        folder.join(format!("{name}{PART_SUFFIX}"))
    } else {
        folder.join(name)
    })
}

/// Whether opening it would run something: by its extension, and on Linux
/// and macOS also by its permission to run.
fn is_program(file: &Path) -> bool {
    let listed = file
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| PROGRAMS.iter().any(|known| known.eq_ignore_ascii_case(ext)));
    listed || runnable(file)
}

#[cfg(unix)]
fn runnable(file: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(file).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn runnable(_file: &Path) -> bool {
    false
}

/// Asks, in the page's language, whether to run this program. Over the
/// main window, which waits for the answer.
async fn confirmed(app: &AppHandle, file: &Path) -> bool {
    let language = app.state::<prefs::Saved>().get().language;
    let texts = Texts::for_language(language.as_deref());
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let mut dialog = app
        .dialog()
        .message(texts.run_text(&name))
        .title(texts.run_title.clone())
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            texts.run.clone(),
            texts.cancel.clone(),
        ));
    if let Some(window) = app.get_webview_window(crate::WINDOW) {
        dialog = dialog.parent(&window);
    }
    // The dialog waits for the user: not on this task.
    tauri::async_runtime::spawn_blocking(move || dialog.blocking_show())
        .await
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(dest_dir: &str, file_name: Option<&str>) -> Placed {
        Placed {
            state: "completed".to_owned(),
            dest_dir: dest_dir.to_owned(),
            file_name: file_name.map(str::to_owned),
        }
    }

    #[test]
    fn a_file_is_its_name_in_its_folder() {
        let folder = if cfg!(windows) {
            r"C:\Downloads"
        } else {
            "/home/me/Downloads"
        };
        let file = file_of(&placed(folder, Some("a b.zip")), false).unwrap();
        assert_eq!(file, Path::new(folder).join("a b.zip"));
        let part = file_of(&placed(folder, Some("a b.zip")), true).unwrap();
        assert_eq!(part, Path::new(folder).join("a b.zip.part.ottie"));
    }

    #[test]
    fn a_name_never_leads_elsewhere() {
        let folder = if cfg!(windows) {
            r"C:\Downloads"
        } else {
            "/home/me/Downloads"
        };
        for name in ["..", "../x.exe", "sub/x.exe", "", "."] {
            assert_eq!(file_of(&placed(folder, Some(name)), false), None, "{name}");
        }
        assert_eq!(file_of(&placed(folder, None), false), None);
        assert_eq!(file_of(&placed("Downloads", Some("x.zip")), false), None);
        if cfg!(windows) {
            assert_eq!(file_of(&placed(folder, Some(r"..\x.exe")), false), None);
            assert_eq!(file_of(&placed(folder, Some("x:y")), false), None);
        }
    }

    #[test]
    fn programs_are_known_by_their_extension() {
        assert!(is_program(Path::new("setup.EXE")));
        assert!(is_program(Path::new("install.sh")));
        assert!(is_program(Path::new("tool.AppImage")));
        assert!(!is_program(Path::new("film.mkv")));
        assert!(!is_program(Path::new("archive.zip")));
        assert!(!is_program(Path::new("README")));
    }
}
