//! The API token: 32 random bytes as hex, in `<config folder>/token`,
//! created the first time it's needed.

use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use crate::config::{DirArgs, Dirs};

const TOKEN_FILE: &str = "token";

pub fn path(config_dir: &Path) -> PathBuf {
    config_dir.join(TOKEN_FILE)
}

/// `ottie token`: prints it (creating it if this is the first run).
pub fn print(args: &DirArgs) -> Result<()> {
    let dirs = Dirs::resolve(args)?;
    println!("{}", load_or_create(&dirs.config)?);
    Ok(())
}

pub fn load_or_create(config_dir: &Path) -> Result<String> {
    let path = path(config_dir);
    if let Some(token) = read(&path)? {
        return Ok(token);
    }
    fs::create_dir_all(config_dir)
        .with_context(|| format!("can't create {}", config_dir.display()))?;
    let token = generate()?;
    match write_private(&path, &token) {
        Ok(()) => Ok(token),
        // Another `ottie` created it at the same moment: use that one.
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            read(&path)?.ok_or_else(|| anyhow!("{} is empty", path.display()))
        }
        Err(err) => Err(err).with_context(|| format!("can't write {}", path.display())),
    }
}

/// `None` if the file doesn't exist yet.
fn read(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let token = text.trim();
            if token.is_empty() {
                bail!("{} is empty; delete it to get a new token", path.display());
            }
            Ok(Some(token.to_owned()))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("can't read {}", path.display())),
    }
}

fn generate() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|err| anyhow!("no random numbers available: {err}"))?;
    let mut hex = String::with_capacity(64);
    for byte in bytes {
        // Writing to a String can't fail.
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

/// Only the owner can read it on Unix. On Windows the config folder
/// (%APPDATA%) is already private to the user.
fn write_private(path: &Path, token: &str) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(token.as_bytes())?;
    file.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_once_then_reused() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("new folder");
        let first = load_or_create(&config).unwrap();
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(load_or_create(&config).unwrap(), first);
        assert_ne!(load_or_create(dir.path()).unwrap(), first, "random");
    }

    #[test]
    fn an_empty_token_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(path(dir.path()), "\n").unwrap();
        assert!(load_or_create(dir.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn only_the_owner_can_read_it() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        load_or_create(dir.path()).unwrap();
        let mode = fs::metadata(path(dir.path())).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
