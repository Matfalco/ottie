//! Ottie's own tray menu (docs/decisions/0034): a small borderless window on
//! the daemon's `/tray` page, opened at the click. The system's menus only
//! hold text items, and this one has the speed limit's field. It's created
//! at the first click, then hidden and shown again, so it opens at once; it
//! hides when it loses focus, like a menu.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, State, Url, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

pub const LABEL: &str = "tray";
/// Its width in CSS pixels; the page says how tall it is.
const WIDTH: f64 = 300.0;
const FIRST_HEIGHT: f64 = 360.0;
/// What the page may ask for: a menu, not a window.
const HEIGHTS: (f64, f64) = (80.0, 720.0);
/// A click on the icon while the menu is open first takes the focus from
/// it, which hides it: that click mustn't open it again.
const JUST_HIDDEN: Duration = Duration::from_millis(300);

/// The menu's state, as managed state.
pub struct Popup {
    /// The `/tray` page, signed in.
    page: Url,
    state: Mutex<Placing>,
}

struct Placing {
    /// Where it was asked for: the click, in physical pixels.
    anchor: PhysicalPosition<f64>,
    /// The page's height, in CSS pixels.
    height: f64,
    /// Created, and shown once the page says how tall it is.
    waiting: bool,
    hidden_at: Option<Instant>,
}

impl Popup {
    /// From the main window's signed-in link (`http://…/#token=…`).
    pub fn new(link: &Url) -> Popup {
        let mut page = link.clone();
        page.set_path("/tray");
        Popup {
            page,
            state: Mutex::new(Placing {
                anchor: PhysicalPosition::new(0.0, 0.0),
                height: FIRST_HEIGHT,
                waiting: false,
                hidden_at: None,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Placing> {
        // Nothing can panic while it's held, so a poisoned lock is still consistent.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Opens the menu at `at`, or closes it if it's open.
pub fn toggle(app: &AppHandle, at: PhysicalPosition<f64>) {
    let popup = app.state::<Popup>();
    let window = app.get_webview_window(LABEL);
    if let Some(window) = &window
        && window.is_visible().unwrap_or(false)
    {
        let _ = window.hide();
        return;
    }
    let mut placing = popup.lock();
    if placing
        .hidden_at
        .is_some_and(|at| at.elapsed() < JUST_HIDDEN)
    {
        return;
    }
    placing.anchor = at;
    match window {
        Some(window) => {
            place(app, &window, &placing);
            // Best effort: the icon can be clicked again.
            let _ = window.show();
            let _ = window.set_focus();
        }
        None => {
            placing.waiting = true;
            drop(placing);
            if create(app, &popup.page).is_err() {
                popup.lock().waiting = false;
            }
        }
    }
}

fn create(app: &AppHandle, page: &Url) -> tauri::Result<()> {
    let daemon = page.origin();
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(page.clone()))
        .title("Ottie")
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        // Shown once the page says how tall it is.
        .visible(false)
        .inner_size(WIDTH, FIRST_HEIGHT)
        .on_navigation(move |url| url.origin() == daemon)
        .build()?;
    let handle = app.clone();
    let hidden = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            let _ = hidden.hide();
            handle.state::<Popup>().lock().hidden_at = Some(Instant::now());
        }
    });
    Ok(())
}

/// Sizes it and puts it beside the click, inside the screen's work area.
fn place(app: &AppHandle, window: &WebviewWindow, placing: &Placing) {
    let monitor = app
        .monitor_from_point(placing.anchor.x, placing.anchor.y)
        .ok()
        .flatten();
    let scale = monitor
        .as_ref()
        .map_or(1.0, |monitor| monitor.scale_factor());
    let _ = window.set_size(LogicalSize::new(WIDTH, placing.height));
    let size = (WIDTH * scale, placing.height * scale);
    let area = monitor.map(|monitor| {
        let area = monitor.work_area();
        let (x, y) = (f64::from(area.position.x), f64::from(area.position.y));
        (
            x,
            y,
            x + f64::from(area.size.width),
            y + f64::from(area.size.height),
        )
    });
    let (x, y) = position((placing.anchor.x, placing.anchor.y), size, area);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// The top-left corner: left of and above the click, as from a taskbar at
/// the bottom right; right of or below it where that doesn't fit; always
/// inside `area` (left, top, right, bottom).
fn position(
    (x, y): (f64, f64),
    (width, height): (f64, f64),
    area: Option<(f64, f64, f64, f64)>,
) -> (f64, f64) {
    let Some((left, top, right, bottom)) = area else {
        return ((x - width).max(0.0), (y - height).max(0.0));
    };
    let x = if x - width >= left { x - width } else { x };
    let y = if y - height >= top { y - height } else { y };
    (
        x.min(right - width).max(left),
        y.min(bottom - height).max(top),
    )
}

/// The page is drawn and says how tall it is (again when that changes).
#[tauri::command]
pub fn menu_ready(app: AppHandle, window: WebviewWindow, popup: State<'_, Popup>, height: f64) {
    if window.label() != LABEL || !height.is_finite() {
        return;
    }
    let mut placing = popup.lock();
    placing.height = height.clamp(HEIGHTS.0, HEIGHTS.1);
    let waiting = std::mem::take(&mut placing.waiting);
    if waiting || window.is_visible().unwrap_or(false) {
        place(&app, &window, &placing);
    }
    if waiting {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// "Open Ottie" in the menu.
#[tauri::command]
pub fn show_main(app: AppHandle) {
    crate::show(&app);
}

/// "Quit Ottie" in the menu.
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Option<(f64, f64, f64, f64)> = Some((0.0, 0.0, 1920.0, 1032.0));

    #[test]
    fn it_opens_above_and_left_of_a_click_on_the_taskbar() {
        assert_eq!(
            position((1800.0, 1060.0), (300.0, 400.0), SCREEN),
            (1500.0, 632.0)
        );
    }

    #[test]
    fn it_stays_on_the_screen() {
        // A taskbar at the top left: below and right of the click.
        assert_eq!(position((20.0, 10.0), (300.0, 400.0), SCREEN), (20.0, 10.0));
        // Near the right edge, but too low to go up: moved in.
        let second = Some((1920.0, 0.0, 3840.0, 1080.0));
        assert_eq!(
            position((3830.0, 1070.0), (300.0, 400.0), second),
            (3530.0, 670.0)
        );
        assert_eq!(position((100.0, 100.0), (300.0, 400.0), None), (0.0, 0.0));
    }
}
