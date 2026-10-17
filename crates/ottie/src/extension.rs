//! `ottie extension install|uninstall|status` (docs/decisions/0055): tells
//! each browser found on this computer where the extension's native
//! messaging host is (`ottie` itself, `native_host.rs`), so the extension
//! signs in to Ottie here by itself, and starts it.
//!
//! Browsers look for a small JSON manifest: in a folder of their own on
//! Linux and macOS, named by a registry key on Windows. Only browsers whose
//! own folder (or key) exists get one, and a manifest already right is left
//! alone: the desktop app runs `install` at every start, and security tools
//! frown on a program that rewrites the registry each time.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use directories::BaseDirs;
use serde_json::json;

use crate::config::{DirArgs, Dirs};
use crate::native_host::{GECKO_ID, HOST_NAME};

/// Chromium-based browsers' IDs for the extension: the one it has loaded
/// unpacked from `extension/dist/chrome` (fixed by the manifest's `key`);
/// the stores' are added once it's published there.
pub const CHROMIUM_IDS: &[&str] = &["njjhemepabjjkdpcdplfcgjiooggdpol"];

#[derive(Args, Debug)]
pub struct ExtensionArgs {
    #[command(subcommand)]
    command: ExtensionCommand,
}

#[derive(Subcommand, Debug)]
enum ExtensionCommand {
    /// Set it up for each browser found (again after moving `ottie`).
    Install {
        /// Print nothing unless something fails (the desktop app's start).
        #[arg(long)]
        quiet: bool,
    },
    /// Take it away from every browser.
    Uninstall,
    /// Which browsers have it.
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Chromium,
    Firefox,
}

/// Where a browser looks for the host.
#[derive(Debug, Clone, Copy)]
enum Place {
    /// Linux and macOS: the manifest goes in `hosts`, if `own` (the
    /// browser's folder) exists; both under `root`.
    Folder {
        root: Root,
        own: &'static str,
        hosts: &'static str,
    },
    /// Windows: under `HKEY_CURRENT_USER\Software\<own>`, if that key exists,
    /// `NativeMessagingHosts\<host>` names the manifest.
    Registry { own: &'static str },
}

/// What a `Place::Folder` is under.
#[derive(Debug, Clone, Copy)]
enum Root {
    /// `~/.config` on Linux, `~/Library/Application Support` on macOS.
    Config,
    Home,
}

#[derive(Debug, Clone, Copy)]
struct Browser {
    name: &'static str,
    family: Family,
    place: Place,
}

const fn folder(
    name: &'static str,
    family: Family,
    root: Root,
    own: &'static str,
    hosts: &'static str,
) -> Browser {
    Browser {
        name,
        family,
        place: Place::Folder { root, own, hosts },
    }
}

const fn key(name: &'static str, family: Family, own: &'static str) -> Browser {
    Browser {
        name,
        family,
        place: Place::Registry { own },
    }
}

use Family::{Chromium, Firefox};
use Root::{Config, Home};

#[rustfmt::skip]
const LINUX: &[Browser] = &[
    folder("Google Chrome", Chromium, Config, "google-chrome", "google-chrome/NativeMessagingHosts"),
    folder("Google Chrome Beta", Chromium, Config, "google-chrome-beta", "google-chrome-beta/NativeMessagingHosts"),
    folder("Google Chrome Dev", Chromium, Config, "google-chrome-unstable", "google-chrome-unstable/NativeMessagingHosts"),
    folder("Chromium", Chromium, Config, "chromium", "chromium/NativeMessagingHosts"),
    folder("Microsoft Edge", Chromium, Config, "microsoft-edge", "microsoft-edge/NativeMessagingHosts"),
    folder("Brave", Chromium, Config, "BraveSoftware/Brave-Browser", "BraveSoftware/Brave-Browser/NativeMessagingHosts"),
    folder("Vivaldi", Chromium, Config, "vivaldi", "vivaldi/NativeMessagingHosts"),
    folder("Firefox", Firefox, Home, ".mozilla", ".mozilla/native-messaging-hosts"),
    folder("LibreWolf", Firefox, Home, ".librewolf", ".librewolf/native-messaging-hosts"),
];

#[rustfmt::skip]
const MACOS: &[Browser] = &[
    folder("Google Chrome", Chromium, Config, "Google/Chrome", "Google/Chrome/NativeMessagingHosts"),
    folder("Google Chrome Beta", Chromium, Config, "Google/Chrome Beta", "Google/Chrome Beta/NativeMessagingHosts"),
    folder("Chromium", Chromium, Config, "Chromium", "Chromium/NativeMessagingHosts"),
    folder("Microsoft Edge", Chromium, Config, "Microsoft Edge", "Microsoft Edge/NativeMessagingHosts"),
    folder("Brave", Chromium, Config, "BraveSoftware/Brave-Browser", "BraveSoftware/Brave-Browser/NativeMessagingHosts"),
    folder("Vivaldi", Chromium, Config, "Vivaldi", "Vivaldi/NativeMessagingHosts"),
    folder("Firefox", Firefox, Config, "Mozilla", "Mozilla/NativeMessagingHosts"),
];

/// Brave, Vivaldi and Opera read Google Chrome's key on Windows.
const WINDOWS: &[Browser] = &[
    key(
        "Google Chrome (and Brave, Vivaldi, Opera)",
        Chromium,
        r"Google\Chrome",
    ),
    key("Chromium", Chromium, "Chromium"),
    key("Microsoft Edge", Chromium, r"Microsoft\Edge"),
    key("Firefox", Firefox, "Mozilla"),
];

fn browsers() -> &'static [Browser] {
    if cfg!(windows) {
        WINDOWS
    } else if cfg!(target_os = "macos") {
        MACOS
    } else {
        LINUX
    }
}

/// The host's manifest for a family of browsers, naming `exe`.
fn manifest(family: Family, exe: &Path) -> String {
    let mut manifest = json!({
        "name": HOST_NAME,
        "description": "Ottie, for its browser extension",
        "path": exe,
        "type": "stdio",
    });
    match family {
        Chromium => {
            let origins: Vec<String> = CHROMIUM_IDS
                .iter()
                .map(|id| format!("chrome-extension://{id}/"))
                .collect();
            manifest["allowed_origins"] = json!(origins);
        }
        Firefox => manifest["allowed_extensions"] = json!([GECKO_ID]),
    }
    // `json!` objects keep their keys sorted; that's fine for browsers.
    let mut text = serde_json::to_string_pretty(&manifest).unwrap_or_default();
    text.push('\n');
    text
}

/// The folders a `Place::Folder` is under.
struct Roots {
    config: PathBuf,
    home: PathBuf,
}

impl Roots {
    fn find() -> Result<Roots> {
        let base = BaseDirs::new().context("can't find the home folder")?;
        // macOS: `config_dir` is ~/Library/Application Support, as browsers use.
        Ok(Roots {
            config: base.config_dir().to_owned(),
            home: base.home_dir().to_owned(),
        })
    }

    fn of(&self, root: Root) -> &Path {
        match root {
            Config => &self.config,
            Home => &self.home,
        }
    }
}

/// What happened for one browser.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Written(PathBuf),
    AlreadyThere,
    NotFound,
    Removed,
    NotThere,
}

pub fn run(args: &ExtensionArgs) -> Result<()> {
    let roots = Roots::find()?;
    match args.command {
        ExtensionCommand::Install { quiet } => {
            let exe = std::env::current_exe().context("can't find ottie itself")?;
            let mut any = false;
            for browser in browsers() {
                let outcome = install_one(browser, &roots, &exe)?;
                any |= outcome != Outcome::NotFound;
                if !quiet {
                    println!("{}: {}", browser.name, describe(&outcome));
                }
            }
            if !any && !quiet {
                println!("No browser found: sign the extension in with a link instead.");
            }
            Ok(())
        }
        ExtensionCommand::Uninstall => {
            for browser in browsers() {
                println!(
                    "{}: {}",
                    browser.name,
                    describe(&uninstall_one(browser, &roots)?)
                );
            }
            Ok(())
        }
        ExtensionCommand::Status => {
            for browser in browsers() {
                println!("{}: {}", browser.name, status_one(browser, &roots)?);
            }
            Ok(())
        }
    }
}

fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Written(path) => format!("set up ({})", path.display()),
        Outcome::AlreadyThere => "already set up".to_owned(),
        Outcome::NotFound => "not found".to_owned(),
        Outcome::Removed => "taken away".to_owned(),
        Outcome::NotThere => "wasn't set up".to_owned(),
    }
}

fn install_one(browser: &Browser, roots: &Roots, exe: &Path) -> Result<Outcome> {
    let text = manifest(browser.family, exe);
    match browser.place {
        Place::Folder { root, own, hosts } => {
            let root = roots.of(root);
            if !root.join(own).is_dir() {
                return Ok(Outcome::NotFound);
            }
            let path = root.join(hosts).join(format!("{HOST_NAME}.json"));
            Ok(if write_if_changed(&path, &text)? {
                Outcome::Written(path)
            } else {
                Outcome::AlreadyThere
            })
        }
        Place::Registry { own } => {
            if registry::value(&format!(r"HKCU\Software\{own}"), false)?.is_none() {
                return Ok(Outcome::NotFound);
            }
            let path = windows_manifest(browser.family)?;
            let wrote = write_if_changed(&path, &text)?;
            let key = host_key(own);
            let named = path.to_string_lossy();
            if registry::value(&key, true)?.as_deref() == Some(&*named) && !wrote {
                return Ok(Outcome::AlreadyThere);
            }
            registry::set_default(&key, &named)?;
            Ok(Outcome::Written(path))
        }
    }
}

fn uninstall_one(browser: &Browser, roots: &Roots) -> Result<Outcome> {
    match browser.place {
        Place::Folder { root, hosts, .. } => {
            let path = roots.of(root).join(hosts).join(format!("{HOST_NAME}.json"));
            match std::fs::remove_file(&path) {
                Ok(()) => Ok(Outcome::Removed),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Outcome::NotThere),
                Err(err) => Err(err).with_context(|| format!("can't remove {}", path.display())),
            }
        }
        Place::Registry { own } => {
            let key = host_key(own);
            if registry::value(&key, true)?.is_none() {
                return Ok(Outcome::NotThere);
            }
            registry::delete(&key)?;
            Ok(Outcome::Removed)
        }
    }
}

fn status_one(browser: &Browser, roots: &Roots) -> Result<String> {
    let exe = std::env::current_exe().context("can't find ottie itself")?;
    let named = match browser.place {
        Place::Folder { root, own, hosts } => {
            let root = roots.of(root);
            if !root.join(own).is_dir() {
                return Ok("not found".to_owned());
            }
            let path = root.join(hosts).join(format!("{HOST_NAME}.json"));
            std::fs::read_to_string(&path).ok()
        }
        Place::Registry { own } => match registry::value(&host_key(own), true)? {
            Some(path) => std::fs::read_to_string(path).ok(),
            None => None,
        },
    };
    Ok(match named.as_deref().and_then(manifest_exe) {
        None => "not set up".to_owned(),
        Some(path) if Path::new(&path) == exe => "set up".to_owned(),
        Some(path) => format!("set up for another ottie ({path})"),
    })
}

/// The program a manifest names.
fn manifest_exe(text: &str) -> Option<String> {
    let manifest: serde_json::Value = serde_json::from_str(text).ok()?;
    Some(manifest.get("path")?.as_str()?.to_owned())
}

/// Writes `text` unless the file already holds it; whether it wrote.
fn write_if_changed(path: &Path, text: &str) -> Result<bool> {
    if std::fs::read_to_string(path).is_ok_and(|there| there == text) {
        return Ok(false);
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)
            .with_context(|| format!("can't create {}", folder.display()))?;
    }
    std::fs::write(path, text).with_context(|| format!("can't write {}", path.display()))?;
    Ok(true)
}

/// Windows: the manifests sit in Ottie's own data folder.
fn windows_manifest(family: Family) -> Result<PathBuf> {
    let dirs = Dirs::resolve(&DirArgs::default())?;
    let name = match family {
        Chromium => "chromium.json",
        Firefox => "firefox.json",
    };
    Ok(dirs.data.join("native-messaging").join(name))
}

fn host_key(own: &str) -> String {
    format!(r"HKCU\Software\{own}\NativeMessagingHosts\{HOST_NAME}")
}

/// The registry through `reg.exe`, as `service/windows.rs` uses Windows'
/// tools: nothing to link, and what it does is plain to see.
mod registry {
    use anyhow::Result;

    use crate::service;

    /// A key's default value (`default`), or whether it exists at all
    /// (`Some("")`); `None` if it doesn't.
    pub fn value(key: &str, default: bool) -> Result<Option<String>> {
        let mut args = vec!["query", key];
        if default {
            args.push("/ve");
        }
        match service::tool("reg", &args) {
            Ok(printed) if default => Ok(default_value(&printed)),
            Ok(_) => Ok(Some(String::new())),
            // `reg query` fails for a key (or value) that isn't there.
            Err(_) => Ok(None),
        }
    }

    pub fn set_default(key: &str, value: &str) -> Result<()> {
        service::tool(
            "reg",
            &["add", key, "/ve", "/t", "REG_SZ", "/d", value, "/f"],
        )?;
        Ok(())
    }

    pub fn delete(key: &str) -> Result<()> {
        service::tool("reg", &["delete", key, "/f"])?;
        Ok(())
    }

    /// From what `reg query <key> /ve` prints: the text after `REG_SZ` (the
    /// value's name before it, "(Default)", is in the system's language).
    pub fn default_value(printed: &str) -> Option<String> {
        printed.lines().find_map(|line| {
            let (_, value) = line.split_once("REG_SZ")?;
            Some(value.trim().to_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(dir: &Path) -> Roots {
        Roots {
            config: dir.join("config"),
            home: dir.join("home"),
        }
    }

    #[test]
    fn manifests_name_ottie_and_the_extension() {
        let exe = Path::new("/opt/ottie/ottie");
        let chromium: serde_json::Value = serde_json::from_str(&manifest(Chromium, exe)).unwrap();
        assert_eq!(chromium["name"], HOST_NAME);
        assert_eq!(chromium["path"], "/opt/ottie/ottie");
        assert_eq!(chromium["type"], "stdio");
        assert_eq!(
            chromium["allowed_origins"][0],
            "chrome-extension://njjhemepabjjkdpcdplfcgjiooggdpol/"
        );
        let firefox: serde_json::Value = serde_json::from_str(&manifest(Firefox, exe)).unwrap();
        assert_eq!(firefox["allowed_extensions"][0], GECKO_ID);
        assert!(firefox.get("allowed_origins").is_none());
    }

    #[test]
    fn only_browsers_found_get_one_and_only_once() {
        let dir = tempfile::tempdir().unwrap();
        let roots = roots(dir.path());
        std::fs::create_dir_all(roots.config.join("chromium")).unwrap();
        std::fs::create_dir_all(roots.home.join(".mozilla")).unwrap();
        let exe = Path::new("/usr/bin/ottie");

        let outcomes: Vec<_> = LINUX
            .iter()
            .map(|browser| (browser.name, install_one(browser, &roots, exe).unwrap()))
            .collect();
        let written: Vec<_> = outcomes
            .iter()
            .filter(|(_, outcome)| matches!(outcome, Outcome::Written(_)))
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(written, ["Chromium", "Firefox"]);
        let path = roots
            .home
            .join(".mozilla/native-messaging-hosts/io.github.matfalco.ottie.json");
        assert_eq!(
            manifest_exe(&std::fs::read_to_string(&path).unwrap()).as_deref(),
            Some("/usr/bin/ottie")
        );

        // Again: nothing to write. Moved: written again.
        let firefox = &LINUX[7];
        assert_eq!(
            install_one(firefox, &roots, exe).unwrap(),
            Outcome::AlreadyThere
        );
        let moved = Path::new("/opt/ottie");
        assert!(matches!(
            install_one(firefox, &roots, moved).unwrap(),
            Outcome::Written(_)
        ));

        assert_eq!(uninstall_one(firefox, &roots).unwrap(), Outcome::Removed);
        assert_eq!(uninstall_one(firefox, &roots).unwrap(), Outcome::NotThere);
        assert!(!path.exists());
    }

    #[test]
    fn the_registrys_default_value_is_read_in_any_language() {
        let printed = "\r\nHKEY_CURRENT_USER\\Software\\Google\\Chrome\\NativeMessagingHosts\\io.github.matfalco.ottie\r\n    (Standard)    REG_SZ    C:\\Users\\Me\\AppData\\Local\\Ottie\\native-messaging\\chromium.json\r\n";
        assert_eq!(
            registry::default_value(printed).as_deref(),
            Some(r"C:\Users\Me\AppData\Local\Ottie\native-messaging\chromium.json")
        );
        assert_eq!(registry::default_value("HKEY_CURRENT_USER\\Software"), None);
    }

    #[test]
    fn every_systems_list_is_well_formed() {
        for list in [LINUX, MACOS, WINDOWS] {
            assert!(list.iter().any(|b| b.family == Chromium));
            assert!(list.iter().any(|b| b.family == Firefox));
        }
    }
}
