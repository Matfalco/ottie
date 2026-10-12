#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Categories: matching by extension, folders, removal and per-category caps.

mod support;

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use ottie_core::{
    CategoryId, CategorySettings, Error, HttpBackend, JobEvent, JobId, JobSnapshot, JobState,
    Manager, NewJob, Store,
};
use reqwest::Url;
use support::{TestServer, test_data};
use tempfile::TempDir;
use tokio::sync::broadcast::Receiver;

const SIZE: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

struct Setup {
    server: TestServer,
    dir: TempDir,
    manager: Manager,
    events: Receiver<JobEvent>,
}

/// `chunk_delay_ms` slows the server down so jobs stay running long enough to poke at.
async fn setup(chunk_delay_ms: u64) -> Setup {
    let server = TestServer::start(test_data(SIZE, 4)).await;
    server.configure(|b| b.chunk_delay = Some(Duration::from_millis(chunk_delay_ms)));
    let store = Store::open_in_memory().await.unwrap();
    let manager = Manager::start(HttpBackend::new().unwrap(), store)
        .await
        .unwrap();
    // Subscribe before adding anything so no event is missed.
    let events = manager.subscribe();
    Setup {
        server,
        dir: tempfile::tempdir().unwrap(),
        manager,
        events,
    }
}

impl Setup {
    fn url(&self, name: &str) -> Url {
        self.server.url(&format!("/files/{name}"))
    }

    fn folder(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Adds a "Videos" category for `.mkv` files, saving into `<dir>/videos`.
    async fn videos(&self, max_concurrent: usize) -> CategoryId {
        let settings = CategorySettings {
            folder: Some(self.folder("videos")),
            extensions: vec!["mkv".into()],
            max_concurrent,
            ..CategorySettings::named("Videos")
        };
        self.manager.add_category(settings).await.unwrap()
    }

    async fn job(&self, id: JobId) -> JobSnapshot {
        let jobs = self.manager.list().await.unwrap();
        jobs.into_iter().find(|j| j.id == id).unwrap()
    }

    /// Waits until every job in `ids` has reached `state`, in any order.
    async fn wait_for_all(&mut self, ids: &[JobId], state: JobState) {
        let events = &mut self.events;
        let mut reached = HashSet::new();
        tokio::time::timeout(TIMEOUT, async {
            while reached.len() < ids.len() {
                if let JobEvent::Changed(job) = events.recv().await.unwrap()
                    && ids.contains(&job.id)
                    && job.state == state
                {
                    reached.insert(job.id);
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("not all of {ids:?} became {state:?}"));
    }
}

#[tokio::test]
async fn general_is_always_there() {
    let s = setup(0).await;
    let categories = s.manager.categories().await.unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].id, CategoryId::GENERAL);
    assert_eq!(categories[0].settings.name, "General");
}

#[tokio::test]
async fn a_matching_extension_picks_the_category_and_its_folder() {
    let mut s = setup(0).await;
    let videos = s.videos(0).await;
    let id = s.manager.add(NewJob::new(s.url("film.MKV"))).await.unwrap();
    s.wait_for_all(&[id], JobState::Completed).await;

    let job = s.job(id).await;
    assert_eq!(job.category, videos);
    // The folder didn't exist; the download created it.
    assert!(s.folder("videos").join("film.MKV").is_file());
}

#[tokio::test]
async fn unmatched_jobs_go_to_general_which_needs_a_folder() {
    let mut s = setup(0).await;
    s.videos(0).await;
    let add = || s.manager.add(NewJob::new(s.url("notes.txt")));
    assert!(matches!(add().await, Err(Error::NoFolder(_))));

    let general = CategorySettings {
        folder: Some(s.folder("general")),
        ..CategorySettings::named("General")
    };
    s.manager
        .update_category(CategoryId::GENERAL, general)
        .await
        .unwrap();
    let id = add().await.unwrap();
    let job = s.job(id).await;
    assert_eq!(job.category, CategoryId::GENERAL);
    assert_eq!(job.dest_dir, s.folder("general"));
    s.wait_for_all(&[id], JobState::Completed).await;
}

#[tokio::test]
async fn a_chosen_category_or_folder_overrides_the_match() {
    let mut s = setup(0).await;
    let videos = s.videos(0).await;
    let elsewhere = s.folder("elsewhere");
    let film = NewJob::new(s.url("film.mkv"))
        .in_category(CategoryId::GENERAL)
        .in_dir(&elsewhere);
    let film = s.manager.add(film).await.unwrap();
    let other = NewJob::new(s.url("data.bin")).in_category(videos);
    let other = s.manager.add(other).await.unwrap();

    let film = s.job(film).await;
    assert_eq!(film.category, CategoryId::GENERAL);
    assert_eq!(film.dest_dir, elsewhere);
    let other = s.job(other).await;
    assert_eq!(other.category, videos);
    assert_eq!(other.dest_dir, s.folder("videos"));
    s.wait_for_all(&[film.id, other.id], JobState::Completed)
        .await;
}

#[tokio::test]
async fn names_and_extensions_belong_to_one_category() {
    let s = setup(0).await;
    let videos = s.videos(0).await;
    let add = |settings| s.manager.add_category(settings);

    let same_name = add(CategorySettings::named(" VIDEOS ")).await;
    assert!(matches!(same_name, Err(Error::CategoryNameTaken(_))));
    let films = CategorySettings {
        extensions: vec![".MKV".into()],
        ..CategorySettings::named("Films")
    };
    let same_ext = add(films.clone()).await;
    assert!(matches!(same_ext, Err(Error::ExtensionTaken { .. })));

    // Editing a category may keep its own name and extensions.
    let mut edited = s.manager.categories().await.unwrap()[1].settings.clone();
    edited.speed_limit = 1000;
    s.manager.update_category(videos, edited).await.unwrap();

    let removed = s.manager.remove_category(CategoryId::GENERAL).await;
    assert!(matches!(removed, Err(Error::CannotRemoveGeneral)));
    let missing = s.manager.update_category(CategoryId(99), films).await;
    assert!(matches!(missing, Err(Error::CategoryNotFound(_))));
}

#[tokio::test]
async fn removing_a_category_moves_its_jobs_to_general() {
    let mut s = setup(2).await;
    let videos = s.videos(0).await;
    let id = s.manager.add(NewJob::new(s.url("film.mkv"))).await.unwrap();
    s.wait_for_all(&[id], JobState::Downloading).await;

    s.manager.remove_category(videos).await.unwrap();
    let job = s.job(id).await;
    assert_eq!(job.category, CategoryId::GENERAL);
    assert_eq!(job.dest_dir, s.folder("videos"), "files don't move");
    assert_eq!(s.manager.categories().await.unwrap().len(), 1);
    // The running job carries on.
    s.wait_for_all(&[id], JobState::Completed).await;
}

#[tokio::test]
async fn a_job_that_has_not_started_follows_its_new_category_folder() {
    let mut s = setup(2).await;
    let videos = s.videos(0).await;
    s.manager.set_max_concurrent(1).await.unwrap();
    let first = NewJob::new(s.url("a.bin")).in_dir(s.folder("misc"));
    let first = s.manager.add(first).await.unwrap();
    let waiting = NewJob::new(s.url("b.bin")).in_dir(s.folder("misc"));
    let waiting = s.manager.add(waiting).await.unwrap();
    assert_eq!(s.job(waiting).await.state, JobState::Queued);

    s.manager.set_category(waiting, videos).await.unwrap();
    let job = s.job(waiting).await;
    assert_eq!(job.category, videos);
    assert_eq!(job.dest_dir, s.folder("videos"));
    s.wait_for_all(&[first, waiting], JobState::Completed).await;
    assert!(s.folder("videos").join("b.bin").is_file());
}

#[tokio::test]
async fn a_category_cap_limits_its_running_jobs() {
    let mut s = setup(1).await;
    s.videos(1).await;
    let mut films = Vec::new();
    for name in ["a.mkv", "b.mkv", "c.mkv"] {
        films.push(s.manager.add(NewJob::new(s.url(name))).await.unwrap());
    }
    let other = NewJob::new(s.url("d.bin")).in_dir(s.folder("misc"));
    let other = s.manager.add(other).await.unwrap();

    let mut running = HashSet::new();
    let mut most_films_at_once = 0;
    let mut other_ran_beside_a_film = false;
    let mut completed = 0;
    tokio::time::timeout(TIMEOUT, async {
        while completed < 4 {
            let JobEvent::Changed(job) = s.events.recv().await.unwrap() else {
                continue;
            };
            match job.state {
                JobState::Connecting => {
                    running.insert(job.id);
                }
                JobState::Completed => {
                    running.remove(&job.id);
                    completed += 1;
                }
                _ => continue,
            }
            let films_running = films.iter().filter(|id| running.contains(id)).count();
            most_films_at_once = most_films_at_once.max(films_running);
            other_ran_beside_a_film |= running.contains(&other) && films_running > 0;
        }
    })
    .await
    .expect("jobs didn't finish");

    assert_eq!(most_films_at_once, 1);
    assert!(other_ran_beside_a_film, "the global cap of 3 had room");
}
