//! Clipboard monitoring (docs/decisions/0043), opt-in: a link copied
//! elsewhere, to a file type a category lists, is offered in a small window
//! in the screen's corner (the daemon's `/caught` page), or added at once
//! with a notification, as Settings say (`app.json`). The clipboard is this
//! computer's, so the app watches it, not the daemon; which links the text
//! holds, the daemon says (`POST /jobs/preview`), and they're added through
//! its API like any other.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, State, Url, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use crate::client::{Daemon, Found};
use crate::notify;
use crate::prefs::{self, Prefs};
use crate::texts::Texts;

/// The window that offers the links.
pub const LABEL: &str = "caught";
/// What its page hears when the links change.
const EVENT: &str = "caught";
/// What the main window hears to open its list of links, with their text.
const ADD_EVENT: &str = "add-links";
/// How often the clipboard is looked at.
const EVERY: Duration = Duration::from_millis(500);
/// Longer text is a document, not links to download.
const MAX_TEXT: usize = 1024 * 1024;
/// The window's width in CSS pixels; the page says how tall it is.
const WIDTH: f64 = 360.0;
const FIRST_HEIGHT: f64 = 150.0;
const HEIGHTS: (f64, f64) = (100.0, 420.0);
/// Between the window and the screen's edges, in CSS pixels.
const MARGIN: f64 = 12.0;

/// What a link copied elsewhere does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Copied {
    /// It's offered in a small window: Download, More…, Ignore.
    #[default]
    Ask,
    /// It's added at once, in its automatic category, and a notification
    /// says so.
    Add,
}

/// The links caught and not dealt with yet, as managed state.
pub struct Catcher {
    daemon: Arc<Daemon>,
    /// The daemon's origin: its own links (sign-in, play links) are never
    /// caught.
    origin: String,
    page: Url,
    /// Offered in the window, oldest first.
    links: Mutex<Vec<String>>,
}

impl Catcher {
    /// From the main window's signed-in link (`http://…/#token=…`).
    pub fn new(link: &Url, daemon: Arc<Daemon>) -> Catcher {
        let mut page = link.clone();
        page.set_path("/caught");
        Catcher {
            daemon,
            origin: link.origin().ascii_serialization(),
            page,
            links: Mutex::new(Vec::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<String>> {
        // Nothing can panic while it's held, so a poisoned lock is still consistent.
        self.links.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The links in `text` worth catching, as the daemon reads them.
    async fn links_in(&self, text: &str) -> anyhow::Result<Vec<String>> {
        let (found, categories) =
            tokio::try_join!(self.daemon.preview(text), self.daemon.categories())?;
        let listed = categories
            .into_iter()
            .flat_map(|category| category.extensions)
            .collect();
        Ok(wanted(found, &listed, &self.origin))
    }
}

/// The links worth catching: to a file type a category lists, not on the
/// download list yet, and not the daemon's own.
fn wanted(found: Vec<Found>, listed: &HashSet<String>, daemon: &str) -> Vec<String> {
    found
        .into_iter()
        .filter_map(|found| {
            let Found::Link {
                url,
                matched,
                known: false,
            } = found
            else {
                return None;
            };
            let extension = matched.extension?.to_ascii_lowercase();
            let own = Url::parse(&url)
                .is_ok_and(|parsed| parsed.origin().ascii_serialization() == daemon);
            (listed.contains(&extension) && !own).then_some(url)
        })
        .collect()
}

/// Starts watching, in a thread of its own: reading the clipboard blocks.
/// Only while Settings say so.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    // Without it, no clipboard monitoring; everything else works.
    let _ = thread::Builder::new()
        .name("clipboard".to_owned())
        .spawn(move || watch(&app));
}

fn watch(app: &AppHandle) {
    let mut reader: Option<Reader> = None;
    loop {
        thread::sleep(EVERY);
        if !app.state::<prefs::Saved>().get().watch_clipboard {
            // Switched on again, what's there by then isn't news.
            reader = None;
            continue;
        }
        if reader.is_none() {
            reader = Reader::new();
        }
        let Some(text) = reader.as_mut().and_then(Reader::copied) else {
            continue;
        };
        // Copied in Ottie itself: a download's link, the sign-in link.
        let in_ottie = app
            .webview_windows()
            .values()
            .any(|window| window.is_focused().unwrap_or(false));
        if in_ottie {
            continue;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move { copied(&app, text).await });
    }
}

/// The system's clipboard, and what was in it last.
struct Reader {
    clipboard: arboard::Clipboard,
    /// Windows: the clipboard's change count; elsewhere, the text itself.
    #[cfg(windows)]
    last: Option<u32>,
    #[cfg(not(windows))]
    last: Option<String>,
}

impl Reader {
    fn new() -> Option<Reader> {
        let mut reader = Reader {
            clipboard: arboard::Clipboard::new().ok()?,
            last: None,
        };
        // What's there already isn't news.
        let _ = reader.copied();
        Some(reader)
    }

    /// Text copied since the last look that may hold a link. Windows counts
    /// the clipboard's changes, so it's read only after one, and never when
    /// a password manager marks what it copied as private.
    #[cfg(windows)]
    fn copied(&mut self) -> Option<String> {
        let count = windows::changes();
        if self.last.replace(count) == Some(count) || windows::private() {
            return None;
        }
        self.clipboard
            .get_text()
            .ok()
            .filter(|text| may_hold_link(text))
    }

    /// Elsewhere, the text is read each time and compared.
    #[cfg(not(windows))]
    fn copied(&mut self) -> Option<String> {
        let text = self.clipboard.get_text().unwrap_or_default();
        if self.last.as_ref() == Some(&text) {
            return None;
        }
        self.last = Some(text.clone());
        may_hold_link(&text).then_some(text)
    }
}

/// Worth asking the daemon about: not too long, and with `scheme://` in it.
fn may_hold_link(text: &str) -> bool {
    text.len() <= MAX_TEXT && text.contains("://")
}

#[cfg(windows)]
mod windows {
    use windows_sys::Win32::System::DataExchange::{
        GetClipboardSequenceNumber, IsClipboardFormatAvailable, RegisterClipboardFormatW,
    };

    /// Formats password managers (KeePass, 1Password, Bitwarden…) add to
    /// what they copy, asking clipboard tools to leave it alone.
    const PRIVATE: [&str; 2] = [
        "ExcludeClipboardContentFromMonitorProcessing",
        "Clipboard Viewer Ignore",
    ];

    /// Goes up with every change to the clipboard.
    pub fn changes() -> u32 {
        // SAFETY: no arguments; it only reads a counter.
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn private() -> bool {
        PRIVATE.iter().any(|name| {
            let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
            // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives
            // the call; registering a format that exists returns its ID.
            let format = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
            // SAFETY: takes any format ID, and opens nothing.
            format != 0 && unsafe { IsClipboardFormatAvailable(format) } != 0
        })
    }
}

/// Text copied elsewhere: offered or added, if it holds links worth it.
async fn copied(app: &AppHandle, text: String) {
    let catcher = app.state::<Catcher>();
    // Without the daemon, nothing is caught.
    let Ok(links) = catcher.links_in(&text).await else {
        return;
    };
    if links.is_empty() {
        return;
    }
    let prefs = app.state::<prefs::Saved>().get();
    match prefs.copied {
        Copied::Ask => offer(app, links),
        Copied::Add => add(app, &catcher.daemon, &links, &prefs).await,
    }
}

/// Adds the links to those the window offers, and shows it.
fn offer(app: &AppHandle, links: Vec<String>) {
    {
        let catcher = app.state::<Catcher>();
        let mut waiting = catcher.lock();
        for link in links {
            if !waiting.contains(&link) {
                waiting.push(link);
            }
        }
    }
    if app.get_webview_window(LABEL).is_some() {
        // Its page asks for them again, then says how tall it is, which
        // shows it (`caught_ready`).
        let _ = app.emit(EVENT, ());
    } else if open_window(app).is_err() {
        // Without its window, the main one offers them.
        let links = std::mem::take(&mut *app.state::<Catcher>().lock());
        more(app, &links);
    }
}

fn open_window(app: &AppHandle) -> tauri::Result<()> {
    let page = app.state::<Catcher>().page.clone();
    let daemon = page.origin();
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(page))
        .title("Ottie")
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        // Never takes the focus from what one is doing; its buttons still
        // take clicks.
        .focusable(false)
        .focused(false)
        // Shown once the page says how tall it is.
        .visible(false)
        .inner_size(WIDTH, FIRST_HEIGHT)
        .on_navigation(move |url| url.origin() == daemon)
        .build()
        .map(drop)
}

/// Sizes it and puts it in the corner of the screen nearest the tray icon
/// (where notifications show), else at the bottom right.
fn place(app: &AppHandle, window: &WebviewWindow, height: f64) {
    let icon = app
        .tray_by_id("main")
        .and_then(|tray| tray.rect().ok().flatten());
    let monitor = icon
        .as_ref()
        .and_then(|rect| {
            let at = rect.position.to_physical::<f64>(1.0);
            app.monitor_from_point(at.x, at.y).ok().flatten()
        })
        .or_else(|| app.primary_monitor().ok().flatten());
    let _ = window.set_size(LogicalSize::new(WIDTH, height));
    let Some(monitor) = monitor else {
        return;
    };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (left, top) = (f64::from(area.position.x), f64::from(area.position.y));
    let area = (
        left,
        top,
        left + f64::from(area.size.width),
        top + f64::from(area.size.height),
    );
    let anchor = icon.map(|rect| {
        let at = rect.position.to_physical::<f64>(scale);
        let size = rect.size.to_physical::<f64>(scale);
        (at.x + size.width / 2.0, at.y + size.height / 2.0)
    });
    let (x, y) = corner(
        anchor,
        (WIDTH * scale, height * scale),
        area,
        MARGIN * scale,
    );
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// The window's top-left corner, `margin` inside the corner of `area`
/// (left, top, right, bottom) nearest `anchor`, else the bottom right one.
fn corner(
    anchor: Option<(f64, f64)>,
    (width, height): (f64, f64),
    (left, top, right, bottom): (f64, f64, f64, f64),
    margin: f64,
) -> (f64, f64) {
    let (x, y) = anchor.unwrap_or((right, bottom));
    let x = if x < (left + right) / 2.0 {
        left + margin
    } else {
        right - width - margin
    };
    let y = if y < (top + bottom) / 2.0 {
        top + margin
    } else {
        bottom - height - margin
    };
    (x.max(left), y.max(top))
}

/// Adds the links at once (each in its automatic category), and says so.
async fn add(app: &AppHandle, daemon: &Daemon, links: &[String], prefs: &Prefs) {
    let texts = Texts::for_language(prefs.language.as_deref());
    let added = match daemon.add(links).await {
        Ok(added) => added,
        Err(err) => {
            notify::warn(app, texts.not_added.clone(), Some(format!("{err:#}")));
            return;
        }
    };
    let names: Vec<String> = links
        .iter()
        .zip(&added.ids)
        .filter(|(_, id)| id.is_some())
        .map(|(url, _)| notify::name_of(url))
        .collect();
    match names.as_slice() {
        [] => {}
        [name] => notify::warn(app, texts.added.clone(), Some(name.clone())),
        _ => notify::warn(
            app,
            texts.added.clone(),
            Some(texts.added_count(names.len())),
        ),
    }
    if let Some(refused) = added.refused.first() {
        notify::warn(app, texts.not_added.clone(), Some(refused.error.clone()));
    }
}

/// Opens the main window's list of links on them, to choose their
/// categories and folder.
fn more(app: &AppHandle, links: &[String]) {
    if links.is_empty() {
        return;
    }
    crate::show(app);
    // Best effort: the links are still in the clipboard to paste.
    let _ = app.emit_to(crate::WINDOW, ADD_EVENT, links.join("\n"));
}

/// Takes `links` off those offered, and hides the window if none are left;
/// otherwise its page shows the rest.
fn let_go(app: &AppHandle, links: &[String]) {
    let left = {
        let catcher = app.state::<Catcher>();
        let mut waiting = catcher.lock();
        waiting.retain(|link| !links.contains(link));
        waiting.len()
    };
    if left > 0 {
        let _ = app.emit(EVENT, ());
    } else if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

/// The links the window offers.
#[tauri::command]
pub fn caught(catcher: State<'_, Catcher>) -> Vec<String> {
    catcher.lock().clone()
}

/// The window's page is drawn and says how tall it is: shown, if it has
/// links to offer.
#[tauri::command]
pub fn caught_ready(app: AppHandle, window: WebviewWindow, height: f64) {
    if window.label() != LABEL || !height.is_finite() {
        return;
    }
    place(&app, &window, height.clamp(HEIGHTS.0, HEIGHTS.1));
    let offering = !app.state::<Catcher>().lock().is_empty();
    if offering && !window.is_visible().unwrap_or(true) {
        let _ = window.show();
    }
}

/// Download (once added), Ignore, or the window's time is up.
#[tauri::command]
pub fn caught_done(app: AppHandle, links: Vec<String>) {
    let_go(&app, &links);
}

/// More…: the main window's list, with these links.
#[tauri::command]
pub fn caught_more(app: AppHandle, links: Vec<String>) {
    let_go(&app, &links);
    more(&app, &links);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Matched;

    fn link(url: &str, extension: Option<&str>, known: bool) -> Found {
        Found::Link {
            url: url.to_owned(),
            matched: Matched {
                extension: extension.map(str::to_owned),
            },
            known,
        }
    }

    #[test]
    fn only_new_links_to_listed_file_types_are_caught() {
        let listed = HashSet::from(["zip".to_owned(), "mp4".to_owned()]);
        let found = vec![
            link("https://x.org/a.zip", Some("zip"), false),
            link("https://x.org/page.html", Some("html"), false),
            link("https://x.org/", None, false),
            link("https://x.org/old.mp4", Some("mp4"), true),
            link("https://x.org/B.MP4", Some("MP4"), false),
            link(
                "http://127.0.0.1:24068/api/v1/stream/t/v.mp4",
                Some("mp4"),
                false,
            ),
            Found::Other,
        ];
        assert_eq!(
            wanted(found, &listed, "http://127.0.0.1:24068"),
            ["https://x.org/a.zip", "https://x.org/B.MP4"]
        );
    }

    #[test]
    fn only_text_with_a_link_is_worth_asking_about() {
        assert!(may_hold_link("see https://x.org/a.zip"));
        assert!(!may_hold_link("a password"));
        assert!(!may_hold_link(&"https://".repeat(MAX_TEXT)));
    }

    #[test]
    fn the_window_goes_in_the_corner_nearest_the_tray() {
        let area = (0.0, 0.0, 1920.0, 1040.0);
        let size = (360.0, 150.0);
        // Bottom right, also without an icon to go by.
        assert_eq!(
            corner(Some((1800.0, 1060.0)), size, area, 12.0),
            (1548.0, 878.0)
        );
        assert_eq!(corner(None, size, area, 12.0), (1548.0, 878.0));
        // A panel at the top left (Linux), a taskbar on the left (right to left).
        assert_eq!(corner(Some((30.0, 10.0)), size, area, 12.0), (12.0, 12.0));
        assert_eq!(
            corner(Some((100.0, 1060.0)), size, area, 12.0),
            (12.0, 878.0)
        );
    }
}
