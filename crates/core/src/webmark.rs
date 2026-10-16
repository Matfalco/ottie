//! Windows' "Mark of the Web" (docs/decisions/0045): a finished file is
//! marked as coming from the Internet, as browsers mark theirs, so Windows
//! warns before running an unknown program (SmartScreen) and Office opens
//! documents in Protected View. The mark is a small stream beside the file's
//! data (`<file>:Zone.Identifier`), which renames and copies keep. Elsewhere,
//! nothing.

use std::path::Path;

use reqwest::Url;

/// Marks the file at `path` as downloaded from `url`. Best effort: some
/// drives (FAT32, exFAT, some network shares) can't hold the mark, and a
/// policy can ask for none.
#[cfg(windows)]
pub(crate) async fn mark(path: &Path, url: &Url) {
    if !wanted() {
        return;
    }
    let mut stream = path.as_os_str().to_owned();
    stream.push(":Zone.Identifier");
    if let Err(err) = tokio::fs::write(&stream, zone_identifier(url)).await {
        tracing::debug!("no Mark of the Web on {}: {err}", path.display());
    }
}

#[cfg(not(windows))]
pub(crate) async fn mark(_path: &Path, _url: &Url) {}

/// The mark: the Internet zone (3), and where the file came from without
/// what could be secret in a link (credentials, the query, the fragment).
#[cfg(any(windows, test))]
fn zone_identifier(url: &Url) -> String {
    let mut from = url.clone();
    from.set_query(None);
    from.set_fragment(None);
    // Only fails for links that can't hold credentials: nothing to remove.
    let _ = from.set_username("");
    let _ = from.set_password(None);
    format!("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl={from}\r\n")
}

/// Unless the "Do not preserve zone information in file attachments" policy
/// is on (1), for the computer or the user, as browsers honour it.
#[cfg(windows)]
fn wanted() -> bool {
    const POLICY: &str = r"Software\Microsoft\Windows\CurrentVersion\Policies\Attachments";
    ![
        windows_registry::LOCAL_MACHINE,
        windows_registry::CURRENT_USER,
    ]
    .iter()
    .any(|root| {
        root.open(POLICY)
            .and_then(|key| key.get_u32("SaveZoneInformation"))
            .is_ok_and(|value| value == 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_names_the_link_without_its_secrets() {
        let url = Url::parse("https://me:pw@example.com/a/setup.exe?sig=abc#x").unwrap();
        assert_eq!(
            zone_identifier(&url),
            "[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.com/a/setup.exe\r\n"
        );
    }
}
