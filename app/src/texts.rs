//! The tray's and the notifications' texts, in the page's language: one file
//! per language in `app/messages/`, like the page's own (docs/decisions/0016).

use serde::Deserialize;

use crate::client::Problem;
use crate::status::{Progress, Summary};

/// Each language's texts, by its tag, as the page names them
/// (`ui/project.inlang/settings.json`). The first is the fallback.
const LANGUAGES: [(&str, &str); 44] = [
    ("en", include_str!("../messages/en.json")),
    ("fr", include_str!("../messages/fr.json")),
    ("de", include_str!("../messages/de.json")),
    ("es-ES", include_str!("../messages/es-ES.json")),
    ("es-419", include_str!("../messages/es-419.json")),
    ("it", include_str!("../messages/it.json")),
    ("nl", include_str!("../messages/nl.json")),
    ("pl", include_str!("../messages/pl.json")),
    ("pt-PT", include_str!("../messages/pt-PT.json")),
    ("pt-BR", include_str!("../messages/pt-BR.json")),
    ("sv", include_str!("../messages/sv.json")),
    ("ro", include_str!("../messages/ro.json")),
    ("cs", include_str!("../messages/cs.json")),
    ("hu", include_str!("../messages/hu.json")),
    ("da", include_str!("../messages/da.json")),
    ("fi", include_str!("../messages/fi.json")),
    ("sk", include_str!("../messages/sk.json")),
    ("hr", include_str!("../messages/hr.json")),
    ("sl", include_str!("../messages/sl.json")),
    ("lt", include_str!("../messages/lt.json")),
    ("lv", include_str!("../messages/lv.json")),
    ("et", include_str!("../messages/et.json")),
    ("ga", include_str!("../messages/ga.json")),
    ("mt", include_str!("../messages/mt.json")),
    ("el", include_str!("../messages/el.json")),
    ("bg", include_str!("../messages/bg.json")),
    ("uk", include_str!("../messages/uk.json")),
    ("zh-Hans", include_str!("../messages/zh-Hans.json")),
    ("zh-Hant", include_str!("../messages/zh-Hant.json")),
    ("ja", include_str!("../messages/ja.json")),
    ("ko", include_str!("../messages/ko.json")),
    ("id", include_str!("../messages/id.json")),
    ("hi", include_str!("../messages/hi.json")),
    ("vi", include_str!("../messages/vi.json")),
    ("tr", include_str!("../messages/tr.json")),
    ("sw", include_str!("../messages/sw.json")),
    ("am", include_str!("../messages/am.json")),
    ("ha", include_str!("../messages/ha.json")),
    ("yo", include_str!("../messages/yo.json")),
    ("zu", include_str!("../messages/zu.json")),
    ("ar", include_str!("../messages/ar.json")),
    ("fa", include_str!("../messages/fa.json")),
    ("he", include_str!("../messages/he.json")),
    ("ur", include_str!("../messages/ur.json")),
];

/// The languages written right to left (`direction` in
/// `ui/src/lib/languages.ts`).
const RIGHT_TO_LEFT: [&str; 4] = ["ar", "fa", "he", "ur"];

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
    /// `{number}` percent.
    percent: String,
    /// `{time}` left before the running downloads are done.
    left: String,
    /// The decimal separator.
    decimal: String,
    /// Between groups of three digits.
    group: String,
    /// The language's own digits, 0 to 9, where the page's `Intl` writes
    /// them (Persian: "۰۱۲۳۴۵۶۷۸۹"); else 0 to 9.
    #[serde(default)]
    digits: Option<String>,
    /// `{number}` and `{unit}`.
    speed: String,
    /// Bytes, kB… per second.
    units: [String; 5],
    /// `{count}` seconds, minutes, hours.
    durations: [String; 3],
    /// A notification's title: one download finished…
    pub finished: String,
    /// …or several, once nothing runs or waits any more.
    pub all_finished: String,
    /// `{count}` downloads finished, at least 2.
    files: String,
    pub failed: String,
    problems: Problems,
    /// Written right to left: every text comes embedded as such.
    #[serde(skip)]
    right_to_left: bool,
}

/// Why a download failed, as the page says it (`problem_…` in
/// `ui/messages/`), less the sizes and times.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Problems {
    /// `{detail}`, as for `redirect` and `disk`.
    network: String,
    redirect: String,
    http403: String,
    http404: String,
    http410: String,
    http429: String,
    /// `{status}`, as for `http`.
    http5xx: String,
    http: String,
    stalled: String,
    incomplete: String,
    remote_changing: String,
    disk: String,
}

impl Texts {
    /// The texts for a language tag: the page's own (`fr`, `pt-BR`), else
    /// its bare language (`fr-CA`), else English. The files are checked by a
    /// test, so the empty fallback never shows.
    pub fn for_language(tag: Option<&str>) -> Texts {
        let tag = tag.unwrap_or_default();
        let primary = tag.split(['-', '_']).next().unwrap_or_default();
        let [english, ..] = LANGUAGES;
        let (language, file) = [tag, primary]
            .iter()
            .find_map(|wanted| {
                LANGUAGES
                    .iter()
                    .find(|(language, _)| language.eq_ignore_ascii_case(wanted))
            })
            .copied()
            .unwrap_or(english);
        let Ok(mut texts) = serde_json::from_str::<Texts>(file) else {
            return serde_json::from_str(english.1).unwrap_or_default();
        };
        if RIGHT_TO_LEFT.contains(&language) {
            texts.right_to_left = true;
            for text in [
                &mut texts.open,
                &mut texts.pause_all,
                &mut texts.resume_all,
                &mut texts.change_limit,
                &mut texts.other_limit,
                &mut texts.quit,
                &mut texts.finished,
                &mut texts.all_finished,
                &mut texts.failed,
            ] {
                *text = embedded(true, std::mem::take(text));
            }
        }
        texts
    }

    /// A text as shown on its own: embedded right to left for those
    /// languages (U+202B … U+202C). Windows draws the tray's tooltip and
    /// menus left to right, which would put "Ottie" or a speed first
    /// (docs/decisions/0042).
    fn shown(&self, text: String) -> String {
        embedded(self.right_to_left, text)
    }

    /// "2 downloading, 5 waiting · 1.5 MB/s", "Nothing downloading".
    pub fn status(&self, summary: &Summary) -> String {
        if !summary.connected {
            return self.shown(self.connecting.clone());
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
            return self.shown(self.idle.clone());
        }
        let mut status = parts.join(", ");
        if summary.running > 0 {
            status.push_str(" · ");
            status.push_str(&self.speed(summary.speed));
        }
        self.shown(status)
    }

    /// "45% · 8 min left": how far the running downloads of known size have
    /// come, and when they'll be done at their speed.
    pub fn progress(&self, progress: &Progress) -> String {
        let done = progress.downloaded.min(progress.total) as f64 / progress.total.max(1) as f64;
        // Whole percents, rounded down so 99.9% isn't "100%", as on the page.
        let percent = (done * 100.0).floor() as u64;
        let mut line = self.percent.replace("{number}", &self.grouped(percent));
        if progress.speed > 0 {
            let left = progress.total.saturating_sub(progress.downloaded);
            // Whole seconds, never above u64 (bytes / bytes per second).
            let seconds = (left as f64 / progress.speed as f64).round() as u64;
            line.push_str(" · ");
            line.push_str(&self.left.replace("{time}", &self.duration(seconds)));
        }
        self.shown(line)
    }

    /// "45 sec", "3 min 20 sec", "2 hr 5 min": the two largest units, as on
    /// the page (`formatDuration`).
    fn duration(&self, seconds: u64) -> String {
        let [in_seconds, in_minutes, in_hours] = &self.durations;
        let part = |text: &str, count: u64| text.replace("{count}", &self.grouped(count));
        let both = |big: String, rest: u64, small: &str| {
            if rest == 0 {
                big
            } else {
                format!("{big} {}", part(small, rest))
            }
        };
        let minutes = seconds / 60;
        let hours = minutes / 60;
        if minutes == 0 {
            part(in_seconds, seconds)
        } else if hours == 0 {
            both(part(in_minutes, minutes), seconds % 60, in_seconds)
        } else {
            both(part(in_hours, hours), minutes % 60, in_minutes)
        }
    }

    /// "Limit speed to 10 MB/s".
    pub fn limit(&self, bytes_per_second: u64) -> String {
        let limit = self
            .limit
            .replace("{speed}", &self.format(bytes_per_second, true));
        self.shown(limit)
    }

    /// "2.5 MB/s", "234 kB/s": one decimal below 10, as on the page (inside
    /// the status line).
    fn speed(&self, bytes_per_second: u64) -> String {
        self.format(bytes_per_second, false)
    }

    /// A limit as people pick it: "1 MB/s", not "1.0 MB/s" (`formatLimit`
    /// on the page).
    pub fn rate(&self, bytes_per_second: u64) -> String {
        self.shown(self.format(bytes_per_second, true))
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
            self.digits(&format!("{value:.1}"))
                .replace('.', &self.decimal)
        } else {
            // Whole: rounded, never above u64 (bytes per second, then /1024s).
            self.grouped(value.round() as u64)
        };
        self.speed
            .replace("{number}", &number)
            .replace("{unit}", &self.units[unit])
    }

    /// "3 files downloaded".
    pub fn files(&self, count: usize) -> String {
        self.shown(self.files.replace("{count}", &self.count(count)))
    }

    /// Why a download failed; the daemon's English for codes this app
    /// doesn't know.
    pub fn reason(&self, problem: Option<&Problem>, english: Option<&str>) -> String {
        let reason = self.reason_text(problem, english);
        self.shown(reason)
    }

    fn reason_text(&self, problem: Option<&Problem>, english: Option<&str>) -> String {
        let texts = &self.problems;
        let detail = |text: &str, detail: &str| text.replace("{detail}", detail);
        let status = |text: &str, status: u16| text.replace("{status}", &status.to_string());
        match problem {
            Some(Problem::Network { detail: d }) => detail(&texts.network, d),
            Some(Problem::Redirect { detail: d }) => detail(&texts.redirect, d),
            Some(Problem::HttpStatus { status: s }) => match s {
                403 => texts.http403.clone(),
                404 => texts.http404.clone(),
                410 => texts.http410.clone(),
                429 => texts.http429.clone(),
                500.. => status(&texts.http5xx, *s),
                _ => status(&texts.http, *s),
            },
            Some(Problem::Stalled) => texts.stalled.clone(),
            Some(Problem::Incomplete) => texts.incomplete.clone(),
            Some(Problem::RemoteChanging) => texts.remote_changing.clone(),
            Some(Problem::Disk { detail: d }) => detail(&texts.disk, d),
            Some(Problem::Other { detail }) => detail.clone(),
            Some(Problem::Unknown) | None => english.unwrap_or_default().to_owned(),
        }
    }

    fn count(&self, count: usize) -> String {
        self.grouped(count as u64)
    }

    /// "12,345" (French: "12 345", Persian: "۱۲٬۳۴۵").
    fn grouped(&self, number: u64) -> String {
        let digits = number.to_string();
        let mut grouped = String::new();
        for (i, digit) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                grouped.push_str(&self.group);
            }
            grouped.push(digit);
        }
        self.digits(&grouped)
    }

    /// `text` with its digits 0 to 9 in the language's own.
    fn digits(&self, text: &str) -> String {
        let Some(own) = &self.digits else {
            return text.to_owned();
        };
        let own: Vec<char> = own.chars().collect();
        text.chars()
            .map(|c| {
                c.to_digit(10)
                    .and_then(|digit| own.get(digit as usize))
                    .copied()
                    .unwrap_or(c)
            })
            .collect()
    }
}

/// `text` embedded right to left if `right_to_left`; nothing stays nothing.
fn embedded(right_to_left: bool, text: String) -> String {
    if right_to_left && !text.is_empty() {
        format!("\u{202b}{text}\u{202c}")
    } else {
        text
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
    fn every_language_has_the_same_texts_and_placeholders() {
        let [(_, english), ..] = LANGUAGES;
        let english: serde_json::Value = serde_json::from_str(english).unwrap();
        for (language, file) in LANGUAGES {
            let texts: serde_json::Value = serde_json::from_str(file).unwrap();
            same_shape(&english, &texts, language);
        }
    }

    /// The same keys (but `digits`, which only some have), lists of the same
    /// length, and texts with the same `{placeholders}`, all the way down.
    fn same_shape(english: &serde_json::Value, other: &serde_json::Value, path: &str) {
        use serde_json::Value;
        match (english, other) {
            (Value::Object(english), Value::Object(other)) => {
                let keys = |object: &serde_json::Map<_, _>| {
                    object
                        .keys()
                        .filter(|key| *key != "digits")
                        .cloned()
                        .collect::<Vec<String>>()
                };
                assert_eq!(keys(english), keys(other), "{path}");
                for (key, value) in english {
                    same_shape(value, &other[key], &format!("{path}.{key}"));
                }
            }
            (Value::Array(english), Value::Array(other)) => {
                assert_eq!(english.len(), other.len(), "{path}");
                for (i, (english, other)) in english.iter().zip(other).enumerate() {
                    same_shape(english, other, &format!("{path}[{i}]"));
                }
            }
            (Value::String(english), Value::String(other)) => {
                assert_eq!(placeholders(english), placeholders(other), "{path}");
            }
            _ => panic!("{path}: not the same kind of value"),
        }
    }

    fn placeholders(text: &str) -> Vec<&str> {
        let mut found: Vec<&str> = text
            .split('{')
            .skip(1)
            .filter_map(|after| after.split_once('}').map(|(name, _)| name))
            .collect();
        found.sort_unstable();
        found
    }

    #[test]
    fn right_to_left_texts_come_embedded_as_such() {
        let ar = Texts::for_language(Some("ar"));
        assert!(ar.quit.starts_with('\u{202b}') && ar.quit.ends_with('\u{202c}'));
        let busy = Summary {
            connected: true,
            running: 2,
            speed: 3 * 1024 * 1024,
            ..Summary::default()
        };
        let status = ar.status(&busy);
        assert_eq!(status.matches('\u{202b}').count(), 1, "{status}");
        assert!(status.starts_with('\u{202b}') && status.ends_with('\u{202c}'));
        assert_eq!(ar.reason(None, None), "");
        let en = Texts::for_language(Some("en"));
        assert_eq!(en.quit, "Quit Ottie");
    }

    #[test]
    fn persian_counts_in_its_own_digits() {
        let fa = Texts::for_language(Some("fa"));
        let inner = |text: String| text.trim_matches(['\u{202b}', '\u{202c}']).to_owned();
        assert_eq!(
            inner(fa.files(1234)),
            inner(fa.files.replace("{count}", "۱٬۲۳۴"))
        );
        assert!(inner(fa.speed(1536 * 1024)).starts_with("۱٫۵"));
    }

    #[test]
    fn a_language_falls_back_to_english() {
        assert_eq!(Texts::for_language(Some("fr-CA")).quit, "Quitter Ottie");
        assert_eq!(Texts::for_language(Some("ru")).quit, "Quit Ottie");
        assert_eq!(Texts::for_language(None).quit, "Quit Ottie");
    }

    #[test]
    fn a_language_with_countries_is_found_by_its_full_tag() {
        assert_eq!(Texts::for_language(Some("pt-BR")).quit, "Sair do Ottie");
        assert_eq!(Texts::for_language(Some("es-419")).quit, "Salir de Ottie");
        assert_eq!(Texts::for_language(Some("zh-Hans")).quit, "退出 Ottie");
        assert_eq!(Texts::for_language(Some("zh-Hant")).quit, "結束 Ottie");
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
        // The unit first, as the page's `Intl` writes Swahili; Zulu as South
        // African English (`intlLocale` in ui/src/lib/languages.ts).
        let sw = Texts::for_language(Some("sw"));
        assert_eq!(sw.speed(1536 * 1024), "MB 1.5 kwa kila sekunde");
        assert_eq!(sw.duration(200), "dakika 3 sekunde 20");
        let zu = Texts::for_language(Some("zu"));
        assert_eq!(zu.speed(1536 * 1024), "1,5 MB/s");
        assert_eq!(zu.speed(1023 * 1024 + 900), "1\u{a0}024 kB/s");
    }

    #[test]
    fn reasons_read_as_on_the_page() {
        let en = Texts::for_language(Some("en"));
        let fr = Texts::for_language(Some("fr"));
        let status = |status| Some(Problem::HttpStatus { status });
        assert_eq!(
            en.reason(status(404).as_ref(), None),
            "Not found on the server (404)"
        );
        assert_eq!(
            fr.reason(status(503).as_ref(), None),
            "Le serveur a un problème (503)"
        );
        assert_eq!(
            en.reason(status(401).as_ref(), None),
            "The server answered 401"
        );
        let disk = Problem::Disk {
            detail: "disk full".to_owned(),
        };
        assert_eq!(
            en.reason(Some(&disk), None),
            "Couldn’t write the file (disk full)"
        );
        assert_eq!(
            en.reason(Some(&Problem::Unknown), Some("Something new")),
            "Something new"
        );
        assert_eq!(fr.files(1234), "1\u{202f}234 fichiers téléchargés");
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

    #[test]
    fn the_progress_says_how_far_and_how_long() {
        let en = Texts::for_language(Some("en"));
        let fr = Texts::for_language(Some("fr"));
        let progress = |downloaded, speed| Progress {
            downloaded,
            total: 10_000,
            speed,
        };
        assert_eq!(en.progress(&progress(4599, 0)), "45%");
        assert_eq!(en.progress(&progress(9999, 0)), "99%");
        assert_eq!(en.progress(&progress(5000, 10)), "50% · 8 min 20 sec left");
        assert_eq!(
            fr.progress(&progress(5000, 10)),
            "50\u{a0}% · encore 8\u{202f}min 20\u{202f}s"
        );
        assert_eq!(en.progress(&progress(5000, 1)), "50% · 1 hr 23 min left");
        assert_eq!(en.progress(&progress(9000, 100)), "90% · 10 sec left");
        assert_eq!(en.progress(&progress(4000, 50)), "40% · 2 min left");
        // Turkish writes the sign first, as Intl does ("%45"); Chinese and
        // Korean put no space before the unit.
        let tr = Texts::for_language(Some("tr"));
        assert_eq!(tr.progress(&progress(4599, 0)), "%45");
        let zh = Texts::for_language(Some("zh-Hans"));
        assert_eq!(zh.progress(&progress(5000, 10)), "50% · 剩余 8分钟 20秒");
        let ko = Texts::for_language(Some("ko"));
        assert_eq!(ko.speed(1536 * 1024), "1.5MB/s");
    }
}
