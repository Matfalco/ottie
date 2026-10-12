//! Choosing safe, collision-free file names inside the target folder.

use std::io;
use std::path::Path;

use percent_encoding::percent_decode_str;
use reqwest::Url;
use tokio::fs::{self, File, OpenOptions};

const FALLBACK_NAME: &str = "download";
/// Leaves room for a " (999)" suffix and `PART_SUFFIX` within common 255-byte limits.
const MAX_NAME_BYTES: usize = 200;
const MAX_EXTENSION_BYTES: usize = 16;
const MAX_COLLISION_ATTEMPTS: u32 = 1000;
const WINDOWS_RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// File name suggested by the last path segment of `url`, sanitized.
pub fn from_url(url: &Url) -> String {
    let last = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or("");
    sanitize(&percent_decode_str(last).decode_utf8_lossy())
}

/// Turns untrusted text into a plain file name: no path separators, no
/// characters Windows rejects, no leading/trailing dots or spaces, no reserved
/// device names. The result can always be joined to a folder without escaping it.
pub fn sanitize(raw: &str) -> String {
    let replaced: String = raw
        .chars()
        .map(|c| match c {
            '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let truncated = truncate(&replaced);
    let trimmed = truncated.trim_matches(|c| c == ' ' || c == '.');
    if trimmed.is_empty() {
        return FALLBACK_NAME.to_owned();
    }
    let stem = trimmed.split('.').next().unwrap_or(trimmed).trim_end();
    if WINDOWS_RESERVED
        .iter()
        .any(|r| r.eq_ignore_ascii_case(stem))
    {
        return format!("_{trimmed}");
    }
    trimmed.to_owned()
}

/// Appended to the final name while downloading. The OS sees `.ottie` as the
/// extension, so every Ottie file shares one extension (see docs/decisions/0007).
pub const PART_SUFFIX: &str = ".part.ottie";

pub fn part_name(name: &str) -> String {
    format!("{name}{PART_SUFFIX}")
}

/// Picks the first free name among `name`, `name (1)`, `name (2)`… and creates
/// its `.part.ottie` file. `create_new` makes the reservation atomic, so two
/// downloads can't grab the same name.
pub async fn reserve(dir: &Path, name: &str) -> io::Result<(String, File)> {
    // A category's folder may not exist yet.
    fs::create_dir_all(dir).await?;
    for n in 0..MAX_COLLISION_ATTEMPTS {
        let candidate = numbered(name, n);
        if fs::try_exists(dir.join(&candidate)).await? {
            continue;
        }
        let part = dir.join(part_name(&candidate));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
            .await
        {
            Ok(file) => return Ok((candidate, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(no_free_name())
}

/// Renames the `.part.ottie` file to the first free name. Normally that's `name`,
/// unless a file with that name appeared while we were downloading.
pub async fn finalize(dir: &Path, name: &str, part: &Path) -> io::Result<String> {
    for n in 0..MAX_COLLISION_ATTEMPTS {
        let candidate = numbered(name, n);
        let target = dir.join(&candidate);
        if !fs::try_exists(&target).await? {
            fs::rename(part, &target).await?;
            return Ok(candidate);
        }
    }
    Err(no_free_name())
}

fn no_free_name() -> io::Error {
    io::Error::new(io::ErrorKind::AlreadyExists, "no free file name")
}

fn numbered(name: &str, n: u32) -> String {
    if n == 0 {
        return name.to_owned();
    }
    match split_extension(name) {
        (stem, Some(ext)) => format!("{stem} ({n}).{ext}"),
        (stem, None) => format!("{stem} ({n})"),
    }
}

fn split_extension(name: &str) -> (&str, Option<&str>) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], Some(&name[i + 1..])),
        _ => (name, None),
    }
}

/// Shortens long names to `MAX_NAME_BYTES`, keeping a short extension.
fn truncate(name: &str) -> String {
    if name.len() <= MAX_NAME_BYTES {
        return name.to_owned();
    }
    let (stem, ext) = match split_extension(name) {
        (stem, Some(ext)) if ext.len() <= MAX_EXTENSION_BYTES => (stem, Some(ext)),
        _ => (name, None),
    };
    let budget = MAX_NAME_BYTES - ext.map_or(0, |e| e.len() + 1);
    let mut cut = budget.min(stem.len());
    while !stem.is_char_boundary(cut) {
        cut -= 1;
    }
    match ext {
        Some(ext) => format!("{}.{ext}", &stem[..cut]),
        None => stem[..cut].to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_comes_from_last_path_segment() {
        let url = Url::parse("https://example.com/a/b/My%20File.zip?token=x").unwrap();
        assert_eq!(from_url(&url), "My File.zip");
    }

    #[test]
    fn empty_path_falls_back() {
        let url = Url::parse("https://example.com/").unwrap();
        assert_eq!(from_url(&url), "download");
    }

    #[test]
    fn encoded_separators_cannot_escape_the_folder() {
        let url = Url::parse("https://example.com/..%2F..%2Fetc%2Fpasswd").unwrap();
        assert_eq!(from_url(&url), "_.._etc_passwd");
        assert_eq!(sanitize(".."), "download");
        assert_eq!(sanitize("..\\..\\x"), "_.._x");
    }

    #[test]
    fn windows_rules() {
        assert_eq!(sanitize("a<b>c:d\"e|f?g*h"), "a_b_c_d_e_f_g_h");
        assert_eq!(sanitize("name. . "), "name");
        assert_eq!(sanitize("con.txt"), "_con.txt");
        assert_eq!(sanitize("console.txt"), "console.txt");
        assert_eq!(sanitize("tab\there"), "tab_here");
    }

    #[test]
    fn long_names_keep_their_extension() {
        let long = format!("{}.tar", "é".repeat(150)); // 300 bytes of stem
        let out = sanitize(&long);
        assert!(out.len() <= MAX_NAME_BYTES);
        assert!(out.ends_with(".tar"));
    }

    #[test]
    fn partial_files_end_in_ottie() {
        assert_eq!(part_name("video.mp4"), "video.mp4.part.ottie");
        let longest = format!("{} (999){PART_SUFFIX}", "x".repeat(MAX_NAME_BYTES));
        assert!(longest.len() <= 255, "names must fit common file systems");
    }

    #[test]
    fn collision_suffix_goes_before_extension() {
        assert_eq!(numbered("file.zip", 0), "file.zip");
        assert_eq!(numbered("file.zip", 2), "file (2).zip");
        assert_eq!(numbered("README", 1), "README (1)");
        assert_eq!(numbered(".hidden", 1), ".hidden (1)");
    }
}
