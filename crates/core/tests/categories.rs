#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Categories: matching by extension, folders, removal and per-category caps.

mod support;

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use ottie_core::{
    BuiltIn, CategoryId, CategorySettings, Error, HttpBackend, JobEvent, JobId, JobSnapshot,
    JobState, Manager, NewJob, Store,
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
async fn general_and_other_are_always_there() {
    let s = setup(0).await;
    let categories = s.manager.categories().await.unwrap();
    assert_eq!(categories.len(), 2);
    assert_eq!(categories[0].id, CategoryId::GENERAL);
    assert_eq!(categories[0].settings.name, "General");
    assert_eq!(categories[0].built_in, Some(BuiltIn::General));
    assert_eq!(categories[1].settings.name, "Other");
    assert_eq!(categories[1].built_in, Some(BuiltIn::Other));
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
    let categories = s.manager.categories().await.unwrap();
    let mut edited = categories
        .into_iter()
        .find(|c| c.id == videos)
        .unwrap()
        .settings;
    edited.speed_limit = 1000;
    s.manager.update_category(videos, edited).await.unwrap();

    for built_in in [CategoryId::GENERAL, other_category(&s).await] {
        let removed = s.manager.remove_category(built_in).await;
        assert!(matches!(removed, Err(Error::CannotRemoveBuiltIn(_))));
    }
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
    assert_eq!(s.manager.categories().await.unwrap().len(), 2);
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

/// Serves `name` through a URL that doesn't show it, as download pages do.
fn named_by_server(s: &Setup, name: &str) -> Url {
    let header = format!("attachment; filename=\"{name}\"");
    s.server
        .configure(|b| b.content_disposition = Some(header.into_bytes()));
    s.url("get.php")
}

#[tokio::test]
async fn the_real_name_rematches_an_auto_picked_category() {
    let mut s = setup(0).await;
    let videos = s.videos(0).await;
    let general = CategorySettings {
        folder: Some(s.folder("general")),
        ..CategorySettings::named("General")
    };
    s.manager
        .update_category(CategoryId::GENERAL, general)
        .await
        .unwrap();
    let url = named_by_server(&s, "film.mkv");
    let id = s.manager.add(NewJob::new(url)).await.unwrap();
    assert_eq!(s.job(id).await.category, CategoryId::GENERAL, "from .php");
    s.wait_for_all(&[id], JobState::Completed).await;

    let job = s.job(id).await;
    assert_eq!(job.category, videos);
    assert_eq!(job.dest_dir, s.folder("videos"));
    assert!(s.folder("videos").join("film.mkv").is_file());
}

#[tokio::test]
async fn a_chosen_folder_decides_the_category() {
    let mut s = setup(0).await;
    let videos = s.videos(0).await;
    let other = other_category(&s).await;
    // A folder of no category: Other, even once the real name says .mkv.
    let url = named_by_server(&s, "film.mkv");
    let mine = s.manager.add(NewJob::new(url).in_dir(s.folder("mine")));
    let mine = mine.await.unwrap();
    // Videos' folder: Videos, whatever the file type.
    let data = NewJob::new(s.url("data.bin")).in_dir(s.folder("videos"));
    let data = s.manager.add(data).await.unwrap();
    s.wait_for_all(&[mine, data], JobState::Completed).await;

    let mine = s.job(mine).await;
    assert_eq!(mine.category, other);
    assert!(s.folder("mine").join("film.mkv").is_file());
    assert_eq!(s.job(data).await.category, videos);
}

/// The built-in category for folders of no category.
async fn other_category(s: &Setup) -> CategoryId {
    let categories = s.manager.categories().await.unwrap();
    let other = categories
        .iter()
        .find(|c| c.built_in == Some(BuiltIn::Other));
    other.unwrap().id
}

#[tokio::test]
async fn a_chosen_category_is_not_rematched() {
    let mut s = setup(0).await;
    s.videos(0).await;
    let url = named_by_server(&s, "film.mkv");
    let job = NewJob::new(url)
        .in_category(CategoryId::GENERAL)
        .in_dir(s.folder("mine"));
    let id = s.manager.add(job).await.unwrap();
    s.wait_for_all(&[id], JobState::Completed).await;

    assert_eq!(s.job(id).await.category, CategoryId::GENERAL);
    assert!(s.folder("mine").join("film.mkv").is_file());
}

#[tokio::test]
async fn a_link_can_be_matched_without_adding_it() {
    let s = setup(0).await;
    let videos = s.videos(0).await;
    let matched = s.manager.match_category(s.url("Film.MKV"), None).await;
    let matched = matched.unwrap();
    assert_eq!(matched.category, videos);
    assert_eq!(matched.extension.as_deref(), Some("mkv"));
    assert!(!matched.by_folder);

    let general = s.manager.match_category(s.url("notes.txt"), None).await;
    let general = general.unwrap();
    assert_eq!(general.category, CategoryId::GENERAL);
    assert_eq!(general.extension.as_deref(), Some("txt"));
    let bare = s.manager.match_category(s.url("README"), None).await;
    assert_eq!(bare.unwrap().extension, None);

    let elsewhere = Some(s.folder("elsewhere"));
    let other = s.manager.match_category(s.url("Film.MKV"), elsewhere).await;
    let other = other.unwrap();
    assert_eq!(other.category, other_category(&s).await);
    assert!(other.by_folder);
    let in_videos = Some(s.folder("videos").join(""));
    let in_videos = s
        .manager
        .match_category(s.url("notes.txt"), in_videos)
        .await;
    assert_eq!(in_videos.unwrap().category, videos);
    assert!(
        s.manager.list().await.unwrap().is_empty(),
        "nothing was added"
    );
}

/// Waits until some bytes of `id` have arrived.
async fn wait_for_bytes(s: &mut Setup, id: JobId) {
    let events = &mut s.events;
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let JobEvent::Progress {
                id: got,
                downloaded,
                ..
            } = events.recv().await.unwrap()
                && got == id
                && downloaded > 0
            {
                return;
            }
        }
    })
    .await
    .expect("no bytes arrived");
}

#[tokio::test]
async fn moving_a_paused_job_moves_its_part_file() {
    let mut s = setup(30).await;
    let videos = s.videos(0).await;
    let id = NewJob::new(s.url("data.bin")).in_dir(s.folder("misc"));
    let id = s.manager.add(id).await.unwrap();
    wait_for_bytes(&mut s, id).await;
    s.manager.pause(id).await.unwrap();
    assert!(s.folder("misc").join("data.bin.part.ottie").is_file());

    s.manager.set_category(id, videos).await.unwrap();
    let job = s.job(id).await;
    assert_eq!(
        (job.category, job.dest_dir.clone()),
        (videos, s.folder("videos"))
    );
    assert!(!job.moving);
    assert!(!s.folder("misc").join("data.bin.part.ottie").exists());
    let part = s.folder("videos").join("data.bin.part.ottie");
    assert_eq!(std::fs::metadata(&part).unwrap().len(), job.downloaded);

    s.manager.resume(id).await.unwrap();
    s.wait_for_all(&[id], JobState::Completed).await;
    let file = std::fs::read(s.folder("videos").join("data.bin")).unwrap();
    assert_eq!(file, test_data(SIZE, 4));
}

#[tokio::test]
async fn a_running_job_moves_and_carries_on_in_its_slot() {
    let mut s = setup(30).await;
    let videos = s.videos(0).await;
    s.manager.set_max_concurrent(1).await.unwrap();
    let first = NewJob::new(s.url("a.bin")).in_dir(s.folder("misc"));
    let first = s.manager.add(first).await.unwrap();
    let waiting = NewJob::new(s.url("b.bin")).in_dir(s.folder("misc"));
    let waiting = s.manager.add(waiting).await.unwrap();
    wait_for_bytes(&mut s, first).await;

    s.manager.set_category(first, videos).await.unwrap();
    let job = s.job(first).await;
    assert_eq!(job.dest_dir, s.folder("videos"));
    assert!(job.state.is_running(), "{:?}", job.state);
    // It kept its slot.
    assert_eq!(s.job(waiting).await.state, JobState::Queued);
    assert!(!s.folder("misc").join("a.bin.part.ottie").exists());

    s.wait_for_all(&[first, waiting], JobState::Completed).await;
    let file = std::fs::read(s.folder("videos").join("a.bin")).unwrap();
    assert_eq!(file, test_data(SIZE, 4));
    assert!(s.folder("misc").join("b.bin").is_file());
}

#[tokio::test]
async fn a_finished_file_moves_and_takes_a_free_name() {
    let mut s = setup(0).await;
    let videos = s.videos(0).await;
    let id = NewJob::new(s.url("c.bin")).in_dir(s.folder("misc"));
    let id = s.manager.add(id).await.unwrap();
    s.wait_for_all(&[id], JobState::Completed).await;
    std::fs::create_dir_all(s.folder("videos")).unwrap();
    std::fs::write(s.folder("videos").join("c.bin"), b"someone else's").unwrap();

    s.manager.set_category(id, videos).await.unwrap();
    let job = s.job(id).await;
    assert_eq!(job.file_name.as_deref(), Some("c (1).bin"));
    assert_eq!(job.state, JobState::Completed);
    let file = std::fs::read(s.folder("videos").join("c (1).bin")).unwrap();
    assert_eq!(file, test_data(SIZE, 4));
    assert!(!s.folder("misc").join("c.bin").exists());
}

#[tokio::test]
async fn a_category_without_a_folder_leaves_the_file_where_it_is() {
    let mut s = setup(0).await;
    let docs = s
        .manager
        .add_category(CategorySettings::named("Docs"))
        .await
        .unwrap();
    let id = NewJob::new(s.url("d.bin")).in_dir(s.folder("misc"));
    let id = s.manager.add(id).await.unwrap();
    s.wait_for_all(&[id], JobState::Completed).await;

    s.manager.set_category(id, docs).await.unwrap();
    let job = s.job(id).await;
    assert_eq!((job.category, job.dest_dir), (docs, s.folder("misc")));
    assert!(s.folder("misc").join("d.bin").is_file());
}

#[tokio::test]
async fn a_download_that_cannot_resume_moves_only_once_stopped() {
    let mut s = setup(30).await;
    s.server.configure(|b| b.ranges = false);
    let videos = s.videos(0).await;
    let id = NewJob::new(s.url("e.bin")).in_dir(s.folder("misc"));
    let id = s.manager.add(id).await.unwrap();
    wait_for_bytes(&mut s, id).await;

    let refused = s.manager.set_category(id, videos).await;
    assert!(matches!(refused, Err(Error::CannotMoveWhileRunning(_))));
    let job = s.job(id).await;
    assert_eq!(job.dest_dir, s.folder("misc"));
    assert_ne!(job.category, videos);

    s.wait_for_all(&[id], JobState::Completed).await;
    s.manager.set_category(id, videos).await.unwrap();
    assert!(s.folder("videos").join("e.bin").is_file());
}
