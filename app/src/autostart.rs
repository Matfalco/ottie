//! Starting Ottie when one signs in (docs/decisions/0039): the system starts
//! the app with `--autostart`, and the app then does what `app.json` says:
//! open in the tray, or only start the daemon and quit. The system's entry
//! (the Run key on Windows, a `.desktop` file in `~/.config/autostart` on
//! Linux, a LaunchAgent on macOS) is the truth for whether it starts at all,
//! so turning it off in Task Manager shows here too.

use std::path::Path;

use anyhow::{Context, Result};
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// What the system's entry passes to the app.
pub const ARG: &str = "--autostart";

/// The entry's name. Tauri's uninstaller deletes the Run value named after
/// the product, so it must be "Ottie". A development build has its own, so
/// trying it never takes over the installed app's.
const NAME: &str = if cfg!(debug_assertions) {
    "Ottie (development)"
} else {
    "Ottie"
};

/// What happens when one signs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Startup {
    /// Nothing: Ottie starts when it's opened.
    Off,
    /// The app, its window hidden: the tray icon, notifications, and the
    /// daemon it starts.
    Tray,
    /// Only the daemon, with nothing on screen.
    Daemon,
}

/// What the entry starts when it's there: `Tray` or `Daemon`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Tray,
    Daemon,
}

impl From<Mode> for Startup {
    fn from(mode: Mode) -> Startup {
        match mode {
            Mode::Tray => Startup::Tray,
            Mode::Daemon => Startup::Daemon,
        }
    }
}

/// Whether the system started this app, as one signed in.
pub fn at_sign_in() -> bool {
    std::env::args().skip(1).any(|arg| arg == ARG)
}

/// What happens at sign-in now: `Off` without an entry (or one turned off
/// in Task Manager), else `mode`.
pub fn current(app: &AppHandle, mode: Mode) -> Startup {
    // An entry that can't be read is one that won't start Ottie either.
    match launcher(app).and_then(|launcher| Ok(launcher.is_enabled()?)) {
        Ok(true) => mode.into(),
        Ok(false) | Err(_) => Startup::Off,
    }
}

/// Adds or removes the system's entry; `mode` is saved by the caller.
pub fn set(app: &AppHandle, on: bool) -> Result<()> {
    let launcher = launcher(app)?;
    if on {
        launcher
            .enable()
            .context("can't add Ottie to the startup programs")
    } else {
        launcher
            .disable()
            .context("can't take Ottie off the startup programs")
    }
}

/// Writes the entry again if it's there but starts another path, so it
/// follows the app when it's moved (an AppImage, a reinstall elsewhere).
/// Only then: a program that rewrites its startup entry each time it starts
/// looks like malware settling in, and a security suite's behaviour
/// monitoring (G DATA's) paused the whole app for ~45 s at every start for
/// it. Best effort: at worst Ottie doesn't start at sign-in, and Settings
/// say so.
pub fn refresh(app: &AppHandle) {
    if let Ok(launcher) = launcher(app)
        && let Ok(true) = launcher.is_enabled()
        && written(&launcher).as_deref() != Some(command(&launcher).as_str())
    {
        let _ = launcher.enable();
    }
}

/// The entry's command line, as auto-launch writes it.
fn command(launcher: &AutoLaunch) -> String {
    format!(
        "{} {}",
        launcher.get_app_path(),
        launcher.get_args().join(" ")
    )
}

/// What the entry holds now: the Run value on Windows. Elsewhere it isn't
/// read, so the entry is written again at each start.
#[cfg(windows)]
fn written(launcher: &AutoLaunch) -> Option<String> {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .and_then(|key| key.get_string(launcher.get_app_name()))
        .ok()
}

#[cfg(not(windows))]
fn written(_: &AutoLaunch) -> Option<String> {
    None
}

fn launcher(app: &AppHandle) -> Result<AutoLaunch> {
    let exe = std::env::current_exe().context("can't find where the app is")?;
    // On Linux, an AppImage runs from a temporary mount: start the file itself.
    #[cfg(target_os = "linux")]
    let exe = app.env().appimage.unwrap_or(exe);
    #[cfg(not(target_os = "linux"))]
    let _ = app;
    let mut builder = AutoLaunchBuilder::new();
    builder
        .set_app_name(NAME)
        .set_app_path(&command_path(&exe))
        .set_args(&[ARG]);
    #[cfg(windows)]
    builder.set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);
    Ok(builder.build()?)
}

/// The app's path as the entry's command line needs it. auto-launch writes
/// `path args` as they are, so a path with spaces (`C:\Users\Ann Lee\…`)
/// must be quoted on Windows and Linux; macOS gets a list of arguments.
fn command_path(exe: &Path) -> String {
    let path = exe.display().to_string();
    if cfg!(windows) {
        // Windows paths can't hold a `"`.
        format!("\"{path}\"")
    } else if cfg!(target_os = "linux") {
        desktop_quoted(&path)
    } else {
        path
    }
}

/// A `.desktop` file's quoted argument: `"`, `` ` ``, `$` and `\` escaped by a
/// backslash, then every backslash doubled (the file's own escaping), and
/// `%` doubled (field codes).
fn desktop_quoted(arg: &str) -> String {
    let mut quoted = String::from("\"");
    for c in arg.chars() {
        match c {
            '"' | '`' | '$' => {
                quoted.push_str("\\\\");
                quoted.push(c);
            }
            '\\' => quoted.push_str("\\\\\\\\"),
            '%' => quoted.push_str("%%"),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_arguments_are_quoted_and_escaped() {
        assert_eq!(
            desktop_quoted("/home/ann lee/Ottie.AppImage"),
            r#""/home/ann lee/Ottie.AppImage""#
        );
        assert_eq!(
            desktop_quoted(r#"/a"b$c`d\e%f"#),
            r#""/a\\"b\\$c\\`d\\\\e%%f""#
        );
    }

    #[test]
    #[cfg(windows)]
    fn the_entry_is_compared_as_auto_launch_writes_it() {
        let launcher = AutoLaunchBuilder::new()
            .set_app_name("Ottie (test)")
            .set_app_path(&command_path(Path::new(r"C:\Ann Lee\Ottie\ottie-app.exe")))
            .set_args(&[ARG])
            .set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser)
            .build()
            .unwrap();
        assert_eq!(
            command(&launcher),
            r#""C:\Ann Lee\Ottie\ottie-app.exe" --autostart"#
        );
    }

    #[test]
    fn the_mode_is_what_an_entry_starts() {
        assert_eq!(Startup::from(Mode::default()), Startup::Tray);
        assert_eq!(Startup::from(Mode::Daemon), Startup::Daemon);
        assert_eq!(serde_json::to_string(&Startup::Off).unwrap(), r#""off""#);
    }
}
