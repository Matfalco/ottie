//! The tray's texts, in the page's language: one file per language in
//! `app/messages/`, like the page's own (docs/decisions/0016).

use serde::Deserialize;

use crate::status::Summary;

/// Each language's texts, by its tag. The first is the fallback.
const LANGUAGES: [(&str, &str); 2] = [
    ("en", include_str!("../messages/en.json")),
    ("fr", include_str!("../messages/fr.json")),
];

/// Units count in 1024s, as on the page (`ui/src/lib/format.ts`).
const STEP: f64 = 1024.0;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Texts {
    pub open: String,
    pub pause_all: String,
    pub resume_all: String,
    /// `{speed}`: the limit.
    limit: String,
    /// The submenu of limits to choose from.
    pub change_limit: String,
    /// The last of them: any other, set on the page.
    pub other_limit: String,
    pub quit: String,
    connecting: String,
    idle: String,
    /// `{count}` downloads running.
    running: String,
    /// `{count}` downloads waiting.
    waiting: String,
    /// The decimal separator.
    decimal: String,
    /// Between groups of three digits.
    group: String,
    /// `{number}` and `{unit}`.
    speed: String,
    /// Bytes, kB… per second.
    units: [String; 5],
}

impl Texts {
    /// The texts for a language tag (`fr`, `fr-CA`), else English. The files
    /// are checked by a test, so the empty fallback never shows.
    pub fn for_language(tag: Option<&str>) -> Texts {
        let primary = tag
            .and_then(|tag| tag.split(['-', '_']).next())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let [(_, english), ..] = LANGUAGES;
        let file = LANGUAGES
            .iter()
            .find(|(language, _)| *language == primary)
            .map_or(english, |(_, file)| file);
        serde_json::from_str(file)
            .or_else(|_| serde_json::from_str(english))
            .unwrap_or_default()
    }

    /// "2 downloading, 5 waiting · 1.5 MB/s", "Nothing downloading".
    pub fn status(&self, summary: &Summary) -> String {
        if !summary.connected {
            return self.connecting.clone();
        }
        let mut parts = Vec::new();
        if summary.running > 0 {
            parts.push(
                self.running
                    .replace("{count}", &self.count(summary.running)),
            );
        }
        if summary.waiting > 0 {
            parts.push(
                self.waiting
                    .replace("{count}", &self.count(summary.waiting)),
            );
        }
        if parts.is_empty() {
            return self.idle.clone();
        }
        let mut status = parts.join(", ");
        if summary.running > 0 {
            status.push_str(" · ");
            status.push_str(&self.speed(summary.speed));
        }
        status
    }

    /// "Limit speed to 10 MB/s".
    pub fn limit(&self, bytes_per_second: u64) -> String {
        self.limit.replace("{speed}", &self.rate(bytes_per_second))
    }

    /// "2.5 MB/s", "234 kB/s": one decimal below 10, as on the page.
    pub fn speed(&self, bytes_per_second: u64) -> String {
        self.format(bytes_per_second, false)
    }

    /// A limit as people pick it: "1 MB/s", not "1.0 MB/s" (`formatLimit`
    /// on the page).
    pub fn rate(&self, bytes_per_second: u64) -> String {
        self.format(bytes_per_second, true)
    }

    fn format(&self, bytes_per_second: u64, trim: bool) -> String {
        let mut value = bytes_per_second as f64;
        let mut unit = 0;
        while value >= STEP && unit < self.units.len() - 1 {
            value /= STEP;
            unit += 1;
        }
        let whole = (value * 10.0).round() % 10.0 == 0.0;
        let number = if unit > 0 && value < 10.0 && !(trim && whole) {
            format!("{value:.1}").replace('.', &self.decimal)
        } else {
            // Whole: rounded, never above u64 (bytes per second, then /1024s).
            self.grouped(value.round() as u64)
        };
        self.speed
            .replace("{number}", &number)
            .replace("{unit}", &self.units[unit])
    }

    fn count(&self, count: usize) -> String {
        self.grouped(count as u64)
    }

    /// "12,345" (French: "12 345").
    fn grouped(&self, number: u64) -> String {
        let digits = number.to_string();
        let mut grouped = String::new();
        for (i, digit) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                grouped.push_str(&self.group);
            }
            grouped.push(digit);
        }
        grouped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_every_text() {
        for (language, file) in LANGUAGES {
            let texts: Result<Texts, _> = serde_json::from_str(file);
            assert!(texts.is_ok(), "{language}: {texts:?}");
        }
    }

    #[test]
    fn a_language_falls_back_to_english() {
        assert_eq!(Texts::for_language(Some("fr-CA")).quit, "Quitter Ottie");
        assert_eq!(Texts::for_language(Some("de")).quit, "Quit Ottie");
        assert_eq!(Texts::for_language(None).quit, "Quit Ottie");
    }

    #[test]
    fn speeds_read_as_on_the_page() {
        let en = Texts::for_language(Some("en"));
        let fr = Texts::for_language(Some("fr"));
        assert_eq!(en.speed(512), "512 B/s");
        assert_eq!(en.speed(1536 * 1024), "1.5 MB/s");
        assert_eq!(fr.speed(1536 * 1024), "1,5\u{a0}Mo/s");
        assert_eq!(en.speed(234 * 1024), "234 kB/s");
        assert_eq!(en.speed(1024 * 1024), "1.0 MB/s");
        assert_eq!(en.rate(1024 * 1024), "1 MB/s");
        assert_eq!(en.rate(1536 * 1024), "1.5 MB/s");
        assert_eq!(en.speed(1023 * 1024 + 900), "1,024 kB/s");
        assert_eq!(
            fr.limit(10 * 1024 * 1024),
            "Limiter la vitesse à 10\u{a0}Mo/s"
        );
    }

    #[test]
    fn the_status_says_what_runs() {
        let en = Texts::for_language(Some("en"));
        let fr = Texts::for_language(Some("fr"));
        assert_eq!(en.status(&Summary::default()), "Connecting to Ottie…");
        let idle = Summary {
            connected: true,
            paused: 3,
            ..Summary::default()
        };
        assert_eq!(en.status(&idle), "Nothing downloading");
        let busy = Summary {
            running: 2,
            waiting: 1234,
            speed: 3 * 1024 * 1024,
            ..idle
        };
        assert_eq!(en.status(&busy), "2 downloading, 1,234 waiting · 3.0 MB/s");
        assert_eq!(
            fr.status(&busy),
            "2 en cours, 1\u{202f}234 en attente · 3,0\u{a0}Mo/s"
        );
        let queued = Summary { running: 0, ..busy };
        assert_eq!(en.status(&queued), "1,234 waiting");
    }
}
