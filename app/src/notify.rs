//! Notifications (docs/decisions/0035): downloads finished, one by one or all
//! together, and downloads that failed. The app sees the changes as it follows
//! the daemon for the tray (`status.rs`).

use tauri::{AppHandle, Url};
use tauri_plugin_notification::NotificationExt;

use crate::client::{Job, Problem};
use crate::prefs::{Notify, Prefs};
use crate::texts::Texts;

/// What happened to a download, seen as its state changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum News {
    Finished {
        name: String,
    },
    /// After its last retry, or at once when retrying can't help.
    Failed {
        name: String,
        problem: Option<Problem>,
        error: Option<String>,
    },
}

impl News {
    /// What a download's change from the state `before` tells, if anything.
    pub fn of(before: &str, job: &Job) -> Option<News> {
        if before == job.state {
            return None;
        }
        match job.state.as_str() {
            "completed" => Some(News::Finished { name: name(job) }),
            "failed" => Some(News::Failed {
                name: name(job),
                problem: job.problem.clone(),
                error: job.error.clone(),
            }),
            _ => None,
        }
    }
}

/// A download's name: its file's, else the end of its address (a download
/// that failed before the server answered has no file yet).
fn name(job: &Job) -> String {
    if let Some(name) = &job.file_name {
        return name.clone();
    }
    let Ok(url) = Url::parse(&job.url) else {
        return job.url.clone();
    };
    // Never the query: it can hold a signature or a token.
    let last = url
        .path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()));
    last.or(url.host_str()).unwrap_or(&job.url).to_owned()
}

/// A notification to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub title: String,
    pub body: String,
}

/// Decides what to notify, from what happened and the settings.
#[derive(Debug, Default)]
pub struct Notifier {
    /// "Once everything has finished": the downloads finished since the
    /// last notification, while others still run or wait.
    finished: Vec<String>,
}

impl Notifier {
    /// Whether finished downloads wait for the others before being notified.
    pub fn waiting(&self) -> bool {
        !self.finished.is_empty()
    }

    /// The notifications for `news`; `busy`: downloads still run or wait.
    pub fn notes(
        &mut self,
        news: Vec<News>,
        busy: bool,
        prefs: &Prefs,
        texts: &Texts,
    ) -> Vec<Note> {
        let mut notes = Vec::new();
        for item in news {
            match item {
                News::Finished { name } => match prefs.notify {
                    Notify::Each => notes.push(Note {
                        title: texts.finished.clone(),
                        body: name,
                    }),
                    Notify::All => self.finished.push(name),
                    Notify::Never => {}
                },
                News::Failed {
                    name,
                    problem,
                    error,
                } if prefs.notify_failed => {
                    let reason = texts.reason(problem.as_ref(), error.as_deref());
                    let body = if reason.is_empty() {
                        name
                    } else {
                        format!("{name}\n{reason}")
                    };
                    notes.push(Note {
                        title: texts.failed.clone(),
                        body,
                    });
                }
                News::Failed { .. } => {}
            }
        }
        // The setting changed while some waited: they're dropped.
        if prefs.notify != Notify::All {
            self.finished.clear();
        }
        if !busy {
            match self.finished.as_slice() {
                [] => {}
                [name] => notes.push(Note {
                    title: texts.finished.clone(),
                    body: name.clone(),
                }),
                names => notes.push(Note {
                    title: texts.all_finished.clone(),
                    body: texts.files(names.len()),
                }),
            }
            self.finished.clear();
        }
        notes
    }
}

/// Shows the notifications, as the system does.
pub fn show(app: &AppHandle, notes: Vec<Note>) {
    for note in notes {
        // Best effort: the download list says the same.
        let _ = app
            .notification()
            .builder()
            .title(note.title)
            .body(note.body)
            .show();
    }
}

/// A click on a notification opens the window.
pub fn listen(app: &AppHandle) {
    let handle = app.clone();
    // Never fails on desktop.
    let _ = app.notification().on_action(move |_| crate::show(&handle));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(state: &str, file_name: Option<&str>, url: &str) -> Job {
        Job {
            id: "a".to_owned(),
            url: url.to_owned(),
            file_name: file_name.map(str::to_owned),
            state: state.to_owned(),
            downloaded: 0,
            moving: false,
            problem: None,
            error: None,
        }
    }

    fn finished(name: &str) -> News {
        News::Finished {
            name: name.to_owned(),
        }
    }

    fn prefs(notify: Notify, notify_failed: bool) -> Prefs {
        Prefs {
            notify,
            notify_failed,
            ..Prefs::default()
        }
    }

    #[test]
    fn only_finishing_or_failing_is_news() {
        let done = job("completed", Some("a.zip"), "https://x.org/a");
        assert_eq!(News::of("downloading", &done), Some(finished("a.zip")));
        assert_eq!(News::of("completed", &done), None);
        let paused = job("paused", Some("a.zip"), "https://x.org/a");
        assert_eq!(News::of("downloading", &paused), None);
        let failed = job("failed", None, "https://x.org/files/b.iso?sig=secret");
        assert!(matches!(
            News::of("retrying", &failed),
            Some(News::Failed { name, .. }) if name == "b.iso"
        ));
    }

    #[test]
    fn a_name_comes_from_the_address_without_its_query() {
        assert_eq!(name(&job("failed", None, "https://x.org/dir/")), "dir");
        assert_eq!(name(&job("failed", None, "https://x.org/?k=v")), "x.org");
    }

    #[test]
    fn each_download_is_notified_as_it_finishes() {
        let texts = Texts::for_language(Some("en"));
        let mut notifier = Notifier::default();
        let each = prefs(Notify::Each, true);
        let notes = notifier.notes(vec![finished("a.zip")], true, &each, &texts);
        assert_eq!(
            notes,
            [Note {
                title: "Download finished".to_owned(),
                body: "a.zip".to_owned()
            }]
        );
        assert!(!notifier.waiting());
    }

    #[test]
    fn all_together_waits_until_nothing_runs() {
        let texts = Texts::for_language(Some("en"));
        let mut notifier = Notifier::default();
        let all = prefs(Notify::All, true);
        assert!(
            notifier
                .notes(vec![finished("a.zip")], true, &all, &texts)
                .is_empty()
        );
        assert!(notifier.waiting());
        let notes = notifier.notes(vec![finished("b.iso")], false, &all, &texts);
        assert_eq!(
            notes,
            [Note {
                title: "All downloads finished".to_owned(),
                body: "2 files downloaded".to_owned()
            }]
        );
        // Alone, it's named.
        let notes = notifier.notes(vec![finished("c.mp4")], false, &all, &texts);
        assert_eq!(notes[0].body, "c.mp4");
        // The last one stopped some other way (paused, cancelled).
        notifier.notes(vec![finished("d.mp4")], true, &all, &texts);
        assert_eq!(notifier.notes(Vec::new(), false, &all, &texts).len(), 1);
        assert!(!notifier.waiting());
    }

    #[test]
    fn failures_have_their_own_switch() {
        let texts = Texts::for_language(Some("fr"));
        let mut notifier = Notifier::default();
        let failed = News::Failed {
            name: "a.zip".to_owned(),
            problem: Some(Problem::HttpStatus { status: 404 }),
            error: None,
        };
        let notes = notifier.notes(
            vec![failed.clone()],
            false,
            &prefs(Notify::Never, true),
            &texts,
        );
        assert_eq!(
            notes,
            [Note {
                title: "Échec du téléchargement".to_owned(),
                body: "a.zip\nIntrouvable sur le serveur (404)".to_owned()
            }]
        );
        let off = prefs(Notify::All, false);
        assert!(notifier.notes(vec![failed], false, &off, &texts).is_empty());
    }

    #[test]
    fn never_notifies_no_finished_download() {
        let texts = Texts::for_language(Some("en"));
        let mut notifier = Notifier::default();
        let all = prefs(Notify::All, true);
        notifier.notes(vec![finished("a.zip")], true, &all, &texts);
        let never = prefs(Notify::Never, true);
        assert!(
            notifier
                .notes(vec![finished("b.zip")], false, &never, &texts)
                .is_empty()
        );
    }
}
