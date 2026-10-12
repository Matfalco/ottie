//! Where Ottie keeps its files, and the daemon's options. See
//! docs/decisions/0014.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use directories::BaseDirs;
use serde::Deserialize;

pub const DEFAULT_PORT: u16 = 16889;
pub const CONFIG_FILE: &str = "config.toml";
pub const DATABASE_FILE: &str = "ottie.db";

/// Folders, from flags or environment variables; otherwise the OS's usual places.
#[derive(Args, Debug, Clone, Default)]
pub struct DirArgs {
    /// Folder for config.toml and the token [default: %APPDATA%\Ottie, ~/.config/ottie]
    #[arg(long, env = "OTTIE_CONFIG_DIR", value_name = "FOLDER")]
    pub config_dir: Option<PathBuf>,
    /// Folder for the database [default: %LOCALAPPDATA%\Ottie, ~/.local/share/ottie]
    #[arg(long, env = "OTTIE_DATA_DIR", value_name = "FOLDER")]
    pub data_dir: Option<PathBuf>,
}

pub struct Dirs {
    pub config: PathBuf,
    pub data: PathBuf,
}

impl Dirs {
    pub fn resolve(args: &DirArgs) -> Result<Dirs> {
        // "Ottie" where folder names are usually capitalized, "ottie" on Linux.
        let name = if cfg!(any(windows, target_os = "macos")) {
            "Ottie"
        } else {
            "ottie"
        };
        let base = || {
            BaseDirs::new().context("can't find the home folder; use --config-dir and --data-dir")
        };
        let config = match &args.config_dir {
            Some(dir) => dir.clone(),
            None => base()?.config_dir().join(name),
        };
        let data = match &args.data_dir {
            Some(dir) => dir.clone(),
            None => base()?.data_local_dir().join(name),
        };
        Ok(Dirs { config, data })
    }
}

/// `config.toml`. Every key is optional; flags and `OTTIE_*` variables win.
#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    /// `"0.0.0.0"` opens the API to other computers (use a VPN or reverse proxy).
    pub address: Option<IpAddr>,
    pub port: Option<u16>,
}

impl FileConfig {
    /// A missing file is fine: every option has a default.
    pub fn load(path: &Path) -> Result<FileConfig> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(FileConfig::default()),
            Err(err) => return Err(err).with_context(|| format!("can't read {}", path.display())),
        };
        toml::from_str(&text).with_context(|| format!("{} is not valid", path.display()))
    }
}

/// Flag or variable, else config file, else 127.0.0.1:16889.
pub fn listen_addr(address: Option<IpAddr>, port: Option<u16>, file: &FileConfig) -> SocketAddr {
    let ip = address
        .or(file.address)
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    SocketAddr::new(ip, port.or(file.port).unwrap_or(DEFAULT_PORT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_this_computer_only() {
        let addr = listen_addr(None, None, &FileConfig::default());
        assert_eq!(addr.to_string(), "127.0.0.1:16889");
    }

    #[test]
    fn flags_win_over_the_file() {
        let file: FileConfig = toml::from_str("address = \"0.0.0.0\"\nport = 9000").unwrap();
        assert_eq!(listen_addr(None, None, &file).to_string(), "0.0.0.0:9000");
        let flags = listen_addr(Some(Ipv4Addr::LOCALHOST.into()), Some(7000), &file);
        assert_eq!(flags.to_string(), "127.0.0.1:7000");
    }

    #[test]
    fn config_files_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE);
        assert_eq!(FileConfig::load(&path).unwrap(), FileConfig::default());
        std::fs::write(&path, "prot = 80").unwrap();
        let typo = FileConfig::load(&path).unwrap_err();
        assert!(format!("{typo:#}").contains("prot"), "{typo:#}");
    }

    #[test]
    fn flags_choose_the_folders() {
        let args = DirArgs {
            config_dir: Some("/c".into()),
            data_dir: Some("/d".into()),
        };
        let dirs = Dirs::resolve(&args).unwrap();
        assert_eq!((dirs.config, dirs.data), ("/c".into(), "/d".into()));
    }
}
