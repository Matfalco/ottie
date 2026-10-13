//! The job manager. One task (the "actor") owns every job; everything else
//! sends it commands through a [`Manager`] handle and listens to [`JobEvent`]s.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::Url;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::backend::Backend;
use crate::category::{self, BuiltIn, Category, CategoryId, CategorySettings, Matched};
use crate::download::{Download, Event, Outcome, Placement};
use crate::error::Error;
use crate::filename;
use crate::id::{IdGenerator, JobId};
use crate::job::{JobChange, JobEvent, JobSnapshot, JobState, NewJob, Priority};
use crate::playback::{self, Head, Playbacks};
use crate::problem::Problem;
use crate::settings::{RetryPolicy, Settings, SettingsChange};
use crate::store::{JobRecord, Store};
use crate::throttle::Limiter;
use crate::view::{View, ViewId, ViewSettings};

/// Running jobs report progress to the manager at most this often.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// Progress of a running job is saved at most this often (state changes are
/// saved immediately).
const SAVE_INTERVAL: Duration = Duration::from_secs(2);
const COMMAND_BUFFER: usize = 64;
/// Retries wait twice as long each time, up to this (or the policy's first
/// wait, if that's longer)...
const MAX_RETRY_WAIT: Duration = Duration::from_secs(60);
/// ...unless the server's `Retry-After` asks for longer, up to this.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(600);
/// No wait is longer, whatever the policy says (a huge one would overflow).
const LONGEST_RETRY_WAIT: Duration = Duration::from_secs(24 * 3600);
/// Subscribers that fall further behind than this miss events (and should
/// call [`Manager::list`] to catch up).
const EVENT_BUFFER: usize = 1024;

/// Handle to the job manager. Cheap to clone; every clone talks to the same jobs.
#[derive(Clone)]
pub struct Manager {
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<JobEvent>,
    playbacks: Playbacks,
}

impl Manager {
    /// Loads saved jobs and settings from `store` and starts the manager
    /// task. Jobs that were running when Ottie last stopped (quit or crash)
    /// are queued again. Must be called from inside a tokio runtime. When
    /// every handle is dropped, the manager shuts down as if
    /// [`Manager::shutdown`] was called.
    pub async fn start(backend: impl Backend, store: Store) -> Result<Manager, Error> {
        let loaded = store.load().await?;
        let mut jobs = BTreeMap::new();
        for record in loaded.jobs {
            jobs.insert(record.id, Job::restore(record).await);
        }
        let settings = loaded.settings;
        let limiter = Limiter::new(settings.global_limit);
        let mut categories = BTreeMap::new();
        // The database always has it (migration 8); General would do otherwise.
        let other = loaded
            .categories
            .iter()
            .find(|c| c.built_in == Some(BuiltIn::Other))
            .map_or(CategoryId::GENERAL, |c| c.id);
        for category in loaded.categories {
            limiter.set_category_limit(category.id, category.settings.speed_limit);
            categories.insert(category.id, category.settings);
        }
        let views = loaded
            .views
            .into_iter()
            .map(|view| (view.id, view.settings))
            .collect();
        let (commands, command_rx) = mpsc::channel(COMMAND_BUFFER);
        let (internal, internal_rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let playbacks = Playbacks::default();
        let actor = Actor {
            backend: Arc::new(backend),
            store,
            limiter,
            settings,
            // A removed job's ID may be higher than any left (`None` < `Some`).
            ids: IdGenerator::after(jobs.keys().next_back().copied().max(loaded.last_id)),
            jobs,
            categories,
            other,
            views,
            yields: 0,
            events: events.clone(),
            playbacks: playbacks.clone(),
            internal,
            shutdown: None,
        };
        tokio::spawn(actor.run(command_rx, internal_rx));
        Ok(Manager {
            commands,
            events,
            playbacks,
        })
    }

    /// Queues a download. It starts as soon as a slot is free (in total and
    /// in its category).
    pub async fn add(&self, job: NewJob) -> Result<JobId, Error> {
        self.request(|reply| Command::Add { job, reply }).await
    }

    /// Returns once the job has actually stopped and its `.part.ottie` file is flushed.
    pub async fn pause(&self, id: JobId) -> Result<(), Error> {
        self.request(|reply| Command::Stop {
            id,
            stop: Stop::Pause,
            reply,
        })
        .await
    }

    /// Puts a paused or failed job back in the queue.
    pub async fn resume(&self, id: JobId) -> Result<(), Error> {
        self.request(|reply| Command::Resume { id, reply }).await
    }

    /// Stops the job for good and deletes its `.part.ottie` file.
    pub async fn cancel(&self, id: JobId) -> Result<(), Error> {
        self.request(|reply| Command::Stop {
            id,
            stop: Stop::Cancel,
            reply,
        })
        .await
    }

    /// Takes the job off the list for good. A running job is stopped first
    /// (this answers once it has), and an unfinished one's `.part.ottie` file
    /// is deleted, as with `cancel`. A completed file stays on disk.
    pub async fn remove(&self, id: JobId) -> Result<(), Error> {
        self.request(|reply| Command::Stop {
            id,
            stop: Stop::Remove,
            reply,
        })
        .await
    }

    /// Every job, oldest first.
    pub async fn list(&self) -> Result<Vec<JobSnapshot>, Error> {
        self.request(|reply| Command::List { reply }).await
    }

    /// One job as it is now.
    pub async fn job(&self, id: JobId) -> Result<JobSnapshot, Error> {
        self.request(|reply| Command::Job { id, reply }).await
    }

    pub async fn settings(&self) -> Result<Settings, Error> {
        self.request(|reply| Command::Settings { reply }).await
    }

    /// Changes the settings given, all at once, and answers with all of them.
    /// Each takes effect immediately and is saved. Lowering `max_concurrent`
    /// doesn't stop running jobs; it only delays new ones. A new retry policy
    /// applies from the next failure.
    pub async fn change_settings(&self, change: SettingsChange) -> Result<Settings, Error> {
        self.request(|reply| Command::ChangeSettings { change, reply })
            .await
    }

    /// How many jobs may download at once (at least 1).
    pub async fn set_max_concurrent(&self, max: usize) -> Result<(), Error> {
        self.change(SettingsChange {
            max_concurrent: Some(max),
            ..SettingsChange::default()
        })
        .await
    }

    /// Bytes per second for all downloads together; 0 = unlimited.
    pub async fn set_global_limit(&self, bytes_per_sec: u64) -> Result<(), Error> {
        self.change(SettingsChange {
            global_limit: Some(bytes_per_sec),
            ..SettingsChange::default()
        })
        .await
    }

    pub async fn set_retry_policy(&self, policy: RetryPolicy) -> Result<(), Error> {
        self.change(SettingsChange {
            max_retries: Some(policy.max_retries),
            retries_before_yield: Some(policy.retries_before_yield),
            first_retry_wait: Some(policy.first_wait),
            ..SettingsChange::default()
        })
        .await
    }

    /// Records whether the first-start setup was done (or skipped).
    pub async fn set_setup_done(&self, done: bool) -> Result<(), Error> {
        self.change(SettingsChange {
            setup_done: Some(done),
            ..SettingsChange::default()
        })
        .await
    }

    async fn change(&self, change: SettingsChange) -> Result<(), Error> {
        self.change_settings(change).await.map(drop)
    }

    /// Changes some of a job's own settings at once, in any state; they take
    /// effect immediately, even while it runs. If anything changed, the job
    /// is saved once and one `Changed` event goes out; otherwise neither.
    /// - `speed_limit`: bytes per second; 0 = none (the global and category
    ///   limits still apply).
    /// - `priority`: its share of the global limit and its place in the queue.
    /// - `streaming`: streaming mode; once set, the file type no longer decides it.
    pub async fn change_job(&self, id: JobId, change: JobChange) -> Result<(), Error> {
        self.request(|reply| Command::ChangeJob { id, change, reply })
            .await
    }

    /// One job's own limit; see [`Manager::change_job`].
    pub async fn set_speed_limit(&self, id: JobId, bytes_per_sec: u64) -> Result<(), Error> {
        let change = JobChange {
            speed_limit: Some(bytes_per_sec),
            ..JobChange::default()
        };
        self.change_job(id, change).await
    }

    /// One job's priority; see [`Manager::change_job`].
    pub async fn set_priority(&self, id: JobId, priority: Priority) -> Result<(), Error> {
        let change = JobChange {
            priority: Some(priority),
            ..JobChange::default()
        };
        self.change_job(id, change).await
    }

    /// Every category, by ID. General (ID 1) is always first; Other is
    /// always there too.
    pub async fn categories(&self) -> Result<Vec<Category>, Error> {
        self.request(|reply| Command::Categories { reply }).await
    }

    /// The category a job for `url` would get if added without one: by the
    /// folder, if one is given (see [`NewJob::in_dir`]), else by the extension
    /// in the URL. The real file name, once the server answers, may still
    /// move a job matched by extension.
    pub async fn match_category(
        &self,
        url: Url,
        dest_dir: Option<PathBuf>,
    ) -> Result<Matched, Error> {
        self.request(|reply| Command::Match {
            url,
            dest_dir,
            reply,
        })
        .await
    }

    pub async fn add_category(&self, settings: CategorySettings) -> Result<CategoryId, Error> {
        self.request(|reply| Command::AddCategory { settings, reply })
            .await
    }

    /// Replaces the category's settings. Limits take effect immediately; a
    /// new folder applies to jobs added from now on.
    pub async fn update_category(
        &self,
        id: CategoryId,
        settings: CategorySettings,
    ) -> Result<(), Error> {
        self.request(|reply| Command::UpdateCategory {
            id,
            settings,
            reply,
        })
        .await
    }

    /// Moves its jobs to General (they stay where they are on disk), then
    /// removes it. General itself can't be removed.
    pub async fn remove_category(&self, id: CategoryId) -> Result<(), Error> {
        self.request(|reply| Command::RemoveCategory { id, reply })
            .await
    }

    /// Moves a job to another category, as the user's choice. If nothing has
    /// been downloaded yet, it will also be saved in that category's folder.
    pub async fn set_category(&self, id: JobId, category: CategoryId) -> Result<(), Error> {
        self.request(|reply| Command::SetCategory {
            id,
            category,
            reply,
        })
        .await
    }

    /// Every saved view, by ID.
    pub async fn views(&self) -> Result<Vec<View>, Error> {
        self.request(|reply| Command::Views { reply }).await
    }

    pub async fn add_view(&self, settings: ViewSettings) -> Result<ViewId, Error> {
        self.request(|reply| Command::AddView { settings, reply })
            .await
    }

    /// Replaces the view's settings (its name too).
    pub async fn update_view(&self, id: ViewId, settings: ViewSettings) -> Result<(), Error> {
        self.request(|reply| Command::UpdateView {
            id,
            settings,
            reply,
        })
        .await
    }

    pub async fn remove_view(&self, id: ViewId) -> Result<(), Error> {
        self.request(|reply| Command::RemoveView { id, reply })
            .await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.events.subscribe()
    }

    /// A player reading the job's file: while it's running, the download
    /// goes where the player reads (docs/decisions/0029).
    pub(crate) fn head(&self, id: JobId) -> Head {
        self.playbacks.head(id)
    }

    /// Stops every running job, saves everything, then stops the manager.
    /// Running jobs keep their state in the database, so the next start
    /// continues them; jobs you paused stay paused.
    pub async fn shutdown(&self) -> Result<(), Error> {
        self.request(|reply| Command::Shutdown { reply }).await
    }

    /// Sends a command with a one-shot reply channel and waits for the answer.
    async fn request<T>(&self, command: impl FnOnce(Reply<T>) -> Command) -> Result<T, Error> {
        let (reply, answer) = oneshot::channel();
        self.commands
            .send(command(reply))
            .await
            .map_err(|_| Error::ManagerStopped)?;
        answer.await.map_err(|_| Error::ManagerStopped)?
    }
}

type Reply<T> = oneshot::Sender<Result<T, Error>>;

enum Command {
    Add {
        job: NewJob,
        reply: Reply<JobId>,
    },
    Stop {
        id: JobId,
        stop: Stop,
        reply: Reply<()>,
    },
    Resume {
        id: JobId,
        reply: Reply<()>,
    },
    List {
        reply: Reply<Vec<JobSnapshot>>,
    },
    Job {
        id: JobId,
        reply: Reply<JobSnapshot>,
    },
    Settings {
        reply: Reply<Settings>,
    },
    ChangeSettings {
        change: SettingsChange,
        reply: Reply<Settings>,
    },
    ChangeJob {
        id: JobId,
        change: JobChange,
        reply: Reply<()>,
    },
    Categories {
        reply: Reply<Vec<Category>>,
    },
    Match {
        url: Url,
        dest_dir: Option<PathBuf>,
        reply: Reply<Matched>,
    },
    AddCategory {
        settings: CategorySettings,
        reply: Reply<CategoryId>,
    },
    UpdateCategory {
        id: CategoryId,
        settings: CategorySettings,
        reply: Reply<()>,
    },
    RemoveCategory {
        id: CategoryId,
        reply: Reply<()>,
    },
    SetCategory {
        id: JobId,
        category: CategoryId,
        reply: Reply<()>,
    },
    Views {
        reply: Reply<Vec<View>>,
    },
    AddView {
        settings: ViewSettings,
        reply: Reply<ViewId>,
    },
    UpdateView {
        id: ViewId,
        settings: ViewSettings,
        reply: Reply<()>,
    },
    RemoveView {
        id: ViewId,
        reply: Reply<()>,
    },
    Shutdown {
        reply: Reply<()>,
    },
}

/// Messages from running job tasks back to the manager.
enum Internal {
    Event {
        id: JobId,
        event: Event,
    },
    Ended {
        id: JobId,
        download: Box<Download>,
        result: Result<Outcome, Error>,
    },
    /// A job's file has moved (or couldn't): its new folder and name.
    Moved {
        id: JobId,
        result: io::Result<(PathBuf, String)>,
    },
    /// A `Retrying` job's wait is over. `due` tells this wait from a later one.
    RetryDue {
        id: JobId,
        due: Instant,
    },
}

/// Why a running job is being stopped. Later variants win: a user's pause or
/// cancel overrides the shutdown, cancel overrides pause, remove overrides all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Stop {
    /// Stop to move the file to another folder, then carry on (see
    /// `set_category`). The state stays as it is meanwhile.
    Move,
    /// Leave the state as it is, so the next start continues the job.
    Shutdown,
    Pause,
    Cancel,
    /// Cancel (unless it completed), then forget the job.
    Remove,
}

impl Stop {
    fn target(self) -> Option<JobState> {
        match self {
            Stop::Move | Stop::Shutdown => None,
            Stop::Pause => Some(JobState::Paused),
            Stop::Cancel | Stop::Remove => Some(JobState::Cancelled),
        }
    }
}

struct Job {
    state: JobState,
    download: Download,
    /// Why it failed, or the last failure while retrying.
    problem: Option<Problem>,
    priority: Priority,
    /// Bytes per second; 0 = no limit of its own.
    speed_limit: u64,
    category: CategoryId,
    /// The user picked the category (or a folder, which decided it), rather
    /// than it being matched by extension; it is never re-matched.
    category_chosen: bool,
    /// Streaming mode (docs/decisions/0029).
    streaming: bool,
    /// The user turned streaming mode on or off, rather than the file type
    /// deciding; it is never decided again.
    streaming_chosen: bool,
    /// Failed attempts since bytes last arrived (not saved).
    retries: u32,
    /// While `Retrying`: when it tries again.
    retry_at: Option<(Instant, SystemTime)>,
    /// 0, or the order in which it gave its slot away to a waiting job: such
    /// jobs queue behind every job that didn't (not saved).
    gave_way: u64,
    /// Present while a task is downloading this job.
    task: Option<Task>,
    /// Present while its category is changing and its file moving there.
    moving: Option<Moving>,
    last_saved: Option<Instant>,
}

/// A move to another category, under way (docs/decisions/0028).
struct Moving {
    category: CategoryId,
    /// The `set_category` call to answer once the file has moved.
    reply: Reply<()>,
}

/// Where a job's file is: its folder, and its name if it has one yet.
type Place = (PathBuf, Option<String>);

impl Job {
    async fn restore(record: JobRecord) -> Job {
        let mut state = record.state;
        let mut download = record.download;
        // Running (or about to retry) when Ottie stopped, whether it quit or
        // crashed: continue it.
        if state.is_running() || state == JobState::Retrying {
            state = JobState::Queued;
        }
        // What's on disk is the truth for how much we have; the saved byte
        // count can lag behind it. With several connections the file has its
        // full size from the start, so the saved ranges tell instead.
        if !download.segments.is_empty() {
            download.downloaded = download.segments.iter().map(|s| s.done).sum();
        } else if !matches!(state, JobState::Completed | JobState::Cancelled)
            && let Some(part) = download.part_path()
        {
            download.downloaded = tokio::fs::metadata(&part).await.map_or(0, |m| m.len());
        }
        Job {
            state,
            download,
            problem: record.problem,
            priority: record.priority,
            speed_limit: record.speed_limit,
            category: record.category,
            category_chosen: record.category_chosen,
            streaming: record.streaming,
            streaming_chosen: record.streaming_chosen,
            retries: 0,
            retry_at: None,
            gave_way: 0,
            task: None,
            moving: None,
            last_saved: None,
        }
    }

    /// Running, or waiting to retry in the slot it ran in, or stopped for a
    /// moment while its file moves.
    fn holds_slot(&self) -> bool {
        self.task.is_some()
            || self.state == JobState::Retrying
            || (self.moving.is_some() && self.state.is_running())
    }

    fn snapshot(&self, id: JobId) -> JobSnapshot {
        let dl = &self.download;
        JobSnapshot {
            id,
            url: dl.url.clone(),
            dest_dir: dl.dest_dir.clone(),
            file_name: dl.file_name.clone(),
            state: self.state,
            downloaded: dl.downloaded,
            total: dl.total,
            resumable: dl.resumable,
            problem: self.problem.clone(),
            priority: self.priority,
            speed_limit: self.speed_limit,
            streaming: self.streaming,
            category: self.category,
            retries: self.retries,
            retry_at: self.retry_at.map(|(_, at)| at),
            segments: dl.segments.clone(),
            moving: self.moving.is_some(),
        }
    }
}

struct Task {
    cancel: CancellationToken,
    stop: Option<Stop>,
    /// Pause/cancel/remove requests to answer once the task has really stopped.
    waiters: Vec<(Stop, Reply<()>)>,
}

/// The task that owns all job state. Only it reads or changes `jobs`.
struct Actor<B> {
    backend: Arc<B>,
    store: Store,
    /// Saved whenever it changes.
    settings: Settings,
    limiter: Limiter,
    /// Sorted by ID, which means by creation time.
    jobs: BTreeMap<JobId, Job>,
    /// Always holds General and Other. Saved whenever one changes.
    categories: BTreeMap<CategoryId, CategorySettings>,
    /// The ID of the built-in Other category.
    other: CategoryId,
    /// Saved views. Saved whenever one changes.
    views: BTreeMap<ViewId, ViewSettings>,
    /// How many times a retrying job has given way (see `Job::gave_way`).
    yields: u64,
    ids: IdGenerator,
    events: broadcast::Sender<JobEvent>,
    /// Where players read; each running job gets its own.
    playbacks: Playbacks,
    /// Cloned into each job task so it can report back.
    internal: mpsc::UnboundedSender<Internal>,
    /// `Some` once shutting down; holds the replies to send when everything has stopped.
    shutdown: Option<Vec<Reply<()>>>,
}

impl<B: Backend> Actor<B> {
    async fn run(
        mut self,
        mut commands: mpsc::Receiver<Command>,
        mut internal: mpsc::UnboundedReceiver<Internal>,
    ) {
        self.schedule();
        let mut handles_alive = true;
        loop {
            tokio::select! {
                // The actor holds a sender itself, so this channel never closes.
                Some(message) = internal.recv() => self.on_internal(message).await,
                command = commands.recv(), if handles_alive => match command {
                    Some(command) => self.on_command(command).await,
                    None => {
                        handles_alive = false;
                        self.begin_shutdown(None);
                    }
                },
            }
            if self.shutdown.is_some() && !self.any_task_running() {
                self.store.flush().await;
                for reply in self.shutdown.take().unwrap_or_default() {
                    let _ = reply.send(Ok(()));
                }
                return;
            }
        }
    }

    // Replies are sent with `let _ =`: sending only fails if the caller
    // stopped waiting, and then there's nobody to tell.
    async fn on_command(&mut self, command: Command) {
        match command {
            Command::Add { job, reply } => {
                let _ = reply.send(self.add(job));
            }
            Command::Stop { id, stop, reply } => self.stop(id, stop, reply).await,
            Command::Resume { id, reply } => {
                let _ = reply.send(self.resume(id));
            }
            Command::List { reply } => {
                let _ = reply.send(Ok(self.list()));
            }
            Command::Job { id, reply } => {
                let snapshot = self.jobs.get(&id).map(|job| job.snapshot(id));
                let _ = reply.send(snapshot.ok_or(Error::JobNotFound(id)));
            }
            Command::Settings { reply } => {
                let _ = reply.send(Ok(self.settings.clone()));
            }
            Command::ChangeSettings { change, reply } => {
                let _ = reply.send(Ok(self.change_settings(change)));
            }
            Command::ChangeJob { id, change, reply } => {
                let _ = reply.send(self.change_job(id, change));
            }
            Command::Categories { reply } => {
                let _ = reply.send(Ok(self.categories()));
            }
            Command::Match {
                url,
                dest_dir,
                reply,
            } => {
                let (category, by_folder) = match &dest_dir {
                    Some(dir) => (self.category_of_folder(&url, dir), true),
                    None => (self.match_category(&url), false),
                };
                let _ = reply.send(Ok(Matched {
                    category,
                    extension: category::url_extension(&url),
                    by_folder,
                }));
            }
            Command::AddCategory { settings, reply } => {
                let _ = reply.send(self.add_category(settings));
            }
            Command::UpdateCategory {
                id,
                settings,
                reply,
            } => {
                let _ = reply.send(self.update_category(id, settings));
            }
            Command::RemoveCategory { id, reply } => {
                let _ = reply.send(self.remove_category(id));
            }
            Command::SetCategory {
                id,
                category,
                reply,
            } => self.set_category(id, category, reply),
            Command::Views { reply } => {
                let _ = reply.send(Ok(self.views()));
            }
            Command::AddView { settings, reply } => {
                let _ = reply.send(self.add_view(settings));
            }
            Command::UpdateView {
                id,
                settings,
                reply,
            } => {
                let _ = reply.send(self.update_view(id, settings));
            }
            Command::RemoveView { id, reply } => {
                let _ = reply.send(self.remove_view(id));
            }
            Command::Shutdown { reply } => self.begin_shutdown(Some(reply)),
        }
    }

    async fn on_internal(&mut self, message: Internal) {
        match message {
            Internal::Event { id, event } => self.on_event(id, event),
            Internal::Ended {
                id,
                download,
                result,
            } => self.on_ended(id, download, result).await,
            Internal::Moved { id, result } => {
                self.finish_move(id, result.map(|(dir, name)| Some((dir, Some(name)))))
            }
            Internal::RetryDue { id, due } => {
                let waiting = self.jobs.get(&id).is_some_and(|job| {
                    job.state == JobState::Retrying
                        && job.retry_at.map(|(at, _)| at) == Some(due)
                        && job.moving.is_none()
                });
                // Not if it was paused, cancelled or gave way meanwhile.
                if waiting && self.shutdown.is_none() {
                    self.start_job(id);
                }
            }
        }
    }

    fn on_event(&mut self, id: JobId, event: Event) {
        let Some(job) = self.jobs.get_mut(&id) else {
            return;
        };
        match event {
            Event::Started {
                file_name,
                dest_dir,
                total,
                validator,
                resumable,
                downloaded,
                segments,
            } => {
                // Named for the first time: the real name (from the server)
                // may belong to another category than the URL suggested, and
                // be a video when the URL didn't say.
                let extension = job
                    .download
                    .file_name
                    .is_none()
                    .then(|| filename::extension(&file_name))
                    .flatten();
                if let Some(ext) = &extension
                    && !job.category_chosen
                {
                    let category = owner_of(&self.categories, ext).unwrap_or(CategoryId::GENERAL);
                    if category != job.category {
                        job.category = category;
                        self.limiter.set_category(id, category);
                    }
                }
                if let Some(ext) = &extension
                    && !job.streaming_chosen
                {
                    job.streaming = self.settings.stream_media && playback::is_media(ext);
                    self.playbacks.of(id).set_streaming(job.streaming);
                }
                let dl = &mut job.download;
                dl.file_name = Some(file_name);
                dl.dest_dir = dest_dir;
                dl.total = total;
                dl.validator = validator;
                dl.resumable = resumable;
                dl.downloaded = downloaded;
                dl.segments = segments;
                // Save right away: from now on a crash must not lose the
                // file name or the validator needed to resume.
                self.save(id);
                self.emit_changed(id);
            }
            Event::Progress {
                downloaded,
                total,
                segments,
            } => {
                job.download.downloaded = downloaded;
                job.download.total = total;
                // It works again: a later failure gets every retry anew.
                job.retries = 0;
                job.gave_way = 0;
                let save_due = job
                    .last_saved
                    .is_none_or(|at| at.elapsed() >= SAVE_INTERVAL);
                if job.state == JobState::Connecting {
                    self.transition_or_log(id, JobState::Downloading, None);
                } else if save_due {
                    self.save(id);
                }
                let _ = self.events.send(JobEvent::Progress {
                    id,
                    downloaded,
                    total,
                    segments,
                });
            }
            // Saved with the next progress save; only flushed ranges are reported.
            Event::Checkpoint { segments } => job.download.segments = segments,
            Event::Restarted(reason) => {
                let _ = self.events.send(JobEvent::Restarted { id, reason });
            }
        }
    }

    fn add(&mut self, new: NewJob) -> Result<JobId, Error> {
        if self.shutdown.is_some() {
            return Err(Error::ManagerStopped);
        }
        // Fixed (never re-matched) when the user picked the category, or a
        // folder, which then decides it.
        let (category, fixed) = match (new.category, &new.dest_dir) {
            (Some(category), _) => (category, true),
            (None, Some(dir)) => (self.category_of_folder(&new.url, dir), true),
            (None, None) => (self.match_category(&new.url), false),
        };
        let settings = self.category(category)?;
        let dest_dir = match new.dest_dir {
            Some(dir) => dir,
            None => settings
                .folder
                .clone()
                .ok_or_else(|| Error::NoFolder(settings.name.clone()))?,
        };
        if dest_dir.to_str().is_none() {
            return Err(Error::NonUtf8Path(dest_dir));
        }
        let streaming = new.streaming.unwrap_or_else(|| {
            self.settings.stream_media
                && category::url_extension(&new.url).is_some_and(|ext| playback::is_media(&ext))
        });
        let download = Download::new(new.url, dest_dir)?;
        let id = self.ids.next(unix_now());
        self.jobs.insert(
            id,
            Job {
                state: JobState::Queued,
                download,
                problem: None,
                priority: Priority::default(),
                speed_limit: 0,
                category,
                category_chosen: fixed,
                streaming,
                streaming_chosen: new.streaming.is_some(),
                retries: 0,
                retry_at: None,
                gave_way: 0,
                task: None,
                moving: None,
                last_saved: None,
            },
        );
        self.save(id);
        self.emit_changed(id);
        self.schedule();
        Ok(id)
    }

    /// Changes a setting of one job (any state), saves it and tells
    /// subscribers. A queued job's new priority counts at the next `schedule`.
    /// Applies what differs from the job's settings, then tells the limiter.
    fn change_job(&mut self, id: JobId, change: JobChange) -> Result<(), Error> {
        let job = self.jobs.get(&id).ok_or(Error::JobNotFound(id))?;
        let JobChange {
            speed_limit,
            priority,
            streaming,
        } = change;
        let speed_limit = speed_limit.filter(|&limit| limit != job.speed_limit);
        let priority = priority.filter(|&priority| priority != job.priority);
        // Chosen now, even if it stays as it was: the file type no longer decides.
        let streaming = streaming.filter(|&on| on != job.streaming || !job.streaming_chosen);
        if speed_limit.is_none() && priority.is_none() && streaming.is_none() {
            return Ok(());
        }
        self.change(id, |job| {
            if let Some(limit) = speed_limit {
                job.speed_limit = limit;
            }
            if let Some(priority) = priority {
                job.priority = priority;
            }
            if let Some(on) = streaming {
                job.streaming = on;
                job.streaming_chosen = true;
            }
        })?;
        if let Some(on) = streaming {
            // A running download changes its order at once.
            self.playbacks.of(id).set_streaming(on);
        }
        if let Some(limit) = speed_limit {
            self.limiter.set_limit(id, limit);
        }
        if let Some(priority) = priority {
            self.limiter.set_priority(id, priority);
        }
        Ok(())
    }

    fn change(&mut self, id: JobId, edit: impl FnOnce(&mut Job)) -> Result<(), Error> {
        let job = self.jobs.get_mut(&id).ok_or(Error::JobNotFound(id))?;
        edit(job);
        self.save(id);
        self.emit_changed(id);
        Ok(())
    }

    /// The category whose extensions include the one in the URL, else General.
    fn match_category(&self, url: &Url) -> CategoryId {
        category::url_extension(url)
            .and_then(|ext| owner_of(&self.categories, &ext))
            .unwrap_or(CategoryId::GENERAL)
    }

    /// The category whose folder `dir` is: the one the extension matches if
    /// its folder is `dir`, else the first with that folder, else Other.
    fn category_of_folder(&self, url: &Url, dir: &Path) -> CategoryId {
        let in_dir = |id: &CategoryId| {
            self.categories
                .get(id)
                .and_then(|c| c.folder.as_deref())
                .is_some_and(|folder| category::same_folder(folder, dir))
        };
        let by_extension = self.match_category(url);
        if in_dir(&by_extension) {
            return by_extension;
        }
        self.categories
            .keys()
            .find(|id| in_dir(id))
            .copied()
            .unwrap_or(self.other)
    }

    fn built_in(&self, id: CategoryId) -> Option<BuiltIn> {
        if id == CategoryId::GENERAL {
            Some(BuiltIn::General)
        } else if id == self.other {
            Some(BuiltIn::Other)
        } else {
            None
        }
    }

    /// For a job's first run, if its category was matched automatically and
    /// it's saved in that category's folder: where to save it instead once
    /// its real name is known. Mirrors the re-match in `on_event`.
    fn placement(&self, job: &Job) -> Option<Placement> {
        let here = &job.download.dest_dir;
        let category_folder = self.categories.get(&job.category)?.folder.as_ref();
        if job.download.file_name.is_some() || job.category_chosen || category_folder != Some(here)
        {
            return None;
        }
        // A category without a folder of its own leaves the job where it is.
        let folder_of = |c: &CategorySettings| c.folder.clone().unwrap_or_else(|| here.clone());
        let by_extension = self
            .categories
            .values()
            .flat_map(|c| {
                c.extensions
                    .iter()
                    .map(move |ext| (ext.clone(), folder_of(c)))
            })
            .collect();
        let otherwise = self
            .categories
            .get(&CategoryId::GENERAL)
            .map_or_else(|| here.clone(), folder_of);
        Some(Placement {
            by_extension,
            otherwise,
        })
    }

    fn category(&self, id: CategoryId) -> Result<&CategorySettings, Error> {
        self.categories.get(&id).ok_or(Error::CategoryNotFound(id))
    }

    fn categories(&self) -> Vec<Category> {
        self.categories
            .iter()
            .map(|(&id, settings)| Category {
                id,
                settings: settings.clone(),
                built_in: self.built_in(id),
            })
            .collect()
    }

    /// Normalizes `settings` and checks that its name and extensions aren't
    /// used by another category (`own` is the one being edited, if any).
    fn check_category(
        &self,
        settings: CategorySettings,
        own: Option<CategoryId>,
    ) -> Result<CategorySettings, Error> {
        let settings = settings.normalized()?;
        let name = settings.name.to_lowercase();
        for (&id, other) in &self.categories {
            if Some(id) == own {
                continue;
            }
            if other.name.to_lowercase() == name {
                return Err(Error::CategoryNameTaken(settings.name));
            }
            if let Some(ext) = settings
                .extensions
                .iter()
                .find(|ext| other.extensions.contains(ext))
            {
                return Err(Error::ExtensionTaken {
                    extension: ext.clone(),
                    category: other.name.clone(),
                });
            }
        }
        Ok(settings)
    }

    fn add_category(&mut self, settings: CategorySettings) -> Result<CategoryId, Error> {
        let settings = self.check_category(settings, None)?;
        // General always exists, so there is a last ID.
        let last = self.categories.keys().next_back().map_or(0, |id| id.0);
        let id = CategoryId(last.checked_add(1).ok_or(Error::TooManyCategories)?);
        self.limiter.set_category_limit(id, settings.speed_limit);
        self.categories.insert(id, settings);
        self.save_category(id);
        Ok(id)
    }

    fn update_category(
        &mut self,
        id: CategoryId,
        mut settings: CategorySettings,
    ) -> Result<(), Error> {
        self.category(id)?;
        if id == self.other {
            // Its jobs are the ones saved outside every category's folder.
            settings.folder = None;
            settings.extensions.clear();
        }
        let settings = self.check_category(settings, Some(id))?;
        self.limiter.set_category_limit(id, settings.speed_limit);
        self.categories.insert(id, settings);
        self.save_category(id);
        // A higher cap on running jobs may let queued ones start.
        self.schedule();
        Ok(())
    }

    fn remove_category(&mut self, id: CategoryId) -> Result<(), Error> {
        let settings = self.category(id)?;
        if self.built_in(id).is_some() {
            return Err(Error::CannotRemoveBuiltIn(settings.name.clone()));
        }
        let members: Vec<JobId> = self
            .jobs
            .iter()
            .filter(|(_, job)| job.category == id)
            .map(|(job_id, _)| *job_id)
            .collect();
        // Moved and saved before the category row is deleted, which the
        // database would refuse while jobs still point at it.
        for job_id in members {
            self.change(job_id, |job| {
                job.category = CategoryId::GENERAL;
                job.category_chosen = false;
            })?;
            self.limiter.set_category(job_id, CategoryId::GENERAL);
        }
        // Views that showed it show General instead, where its jobs went.
        // Saved first too: the database would drop it from them otherwise.
        let showing: Vec<ViewId> = self
            .views
            .iter()
            .filter(|(_, view)| {
                view.categories
                    .as_ref()
                    .is_some_and(|ids| ids.contains(&id))
            })
            .map(|(view_id, _)| *view_id)
            .collect();
        for view_id in showing {
            if let Some(ids) = self
                .views
                .get_mut(&view_id)
                .and_then(|view| view.categories.as_mut())
            {
                for each in ids.iter_mut().filter(|each| **each == id) {
                    *each = CategoryId::GENERAL;
                }
                ids.sort();
                ids.dedup();
            }
            self.save_view(view_id);
        }
        self.categories.remove(&id);
        self.limiter.set_category_limit(id, 0);
        self.store.remove_category(id);
        // Fails only when nobody is subscribed.
        let _ = self.events.send(JobEvent::CategoryRemoved(id));
        self.schedule();
        Ok(())
    }

    /// Moves a job to another category, as the user's choice, and its file
    /// to that category's folder (docs/decisions/0028). A running job stops,
    /// its file moves, and it carries on in the same slot. Answers once the
    /// file has moved; if it can't, the job stays in its old category.
    fn set_category(&mut self, id: JobId, category: CategoryId, reply: Reply<()>) {
        let refusal = if self.shutdown.is_some() {
            Some(Error::ManagerStopped)
        } else {
            self.category(category).err()
        };
        if let Some(err) = refusal {
            let _ = reply.send(Err(err));
            return;
        }
        let folder = self
            .categories
            .get(&category)
            .and_then(|c| c.folder.clone());
        let Some(job) = self.jobs.get_mut(&id) else {
            let _ = reply.send(Err(Error::JobNotFound(id)));
            return;
        };
        if job.moving.is_some() {
            let _ = reply.send(Err(Error::JobMoving(id)));
            return;
        }
        let dl = &job.download;
        let has_file = dl.file_name.is_some() && job.state != JobState::Cancelled;
        // What it has would be lost if it stopped: it would start over.
        let would_restart = has_file && !dl.resumable && dl.downloaded > 0;
        // No folder of its own (it keeps the one it has), or already there.
        let Some(folder) = folder.filter(|folder| !category::same_folder(folder, &dl.dest_dir))
        else {
            let _ = reply.send(self.assign_category(id, category, None));
            self.schedule();
            return;
        };
        if let Some(task) = &mut job.task {
            if would_restart {
                let _ = reply.send(Err(Error::CannotMoveWhileRunning(id)));
                return;
            }
            // Running: stop it first; `on_ended` moves the file.
            task.stop = task.stop.max(Some(Stop::Move));
            task.cancel.cancel();
            job.moving = Some(Moving { category, reply });
            self.emit_changed(id);
            return;
        }
        if !has_file {
            // Nothing on disk yet: it will simply be saved there.
            let _ = reply.send(self.assign_category(id, category, Some((folder, None))));
            self.schedule();
            return;
        }
        job.moving = Some(Moving { category, reply });
        self.emit_changed(id);
        self.move_file(id);
    }

    /// Moves the file of a stopped job whose category is changing, in a task
    /// of its own (between drives it's a copy, which takes a while). With no
    /// file, or no folder to move to, the move is done at once.
    fn move_file(&mut self, id: JobId) {
        let Some(job) = self.jobs.get(&id) else {
            return;
        };
        let Some(moving) = &job.moving else {
            return;
        };
        let folder = self
            .categories
            .get(&moving.category)
            .and_then(|settings| settings.folder.clone());
        let name = job
            .download
            .file_name
            .clone()
            .filter(|_| job.state != JobState::Cancelled);
        let (Some(to), Some(name)) = (folder.clone(), name) else {
            self.finish_move(id, Ok(folder.map(|dir| (dir, None))));
            return;
        };
        let from = job.download.dest_dir.clone();
        let finished = job.state == JobState::Completed;
        let internal = self.internal.clone();
        tokio::spawn(async move {
            let result = filename::relocate(&from, &name, finished, &to)
                .await
                .map(|name| (to, name));
            // Fails only if the manager has stopped.
            let _ = internal.send(Internal::Moved { id, result });
        });
    }

    /// Ends a move: the job takes its new category and place, or stays as it
    /// was if the file couldn't move. A job that was running carries on.
    fn finish_move(&mut self, id: JobId, result: io::Result<Option<Place>>) {
        let Some(moving) = self.jobs.get_mut(&id).and_then(|job| job.moving.take()) else {
            return;
        };
        let answer = match result {
            Ok(place) => {
                // Removed meanwhile: the file stays where it went, and the job
                // goes to General like the category's other jobs.
                let category = if self.categories.contains_key(&moving.category) {
                    moving.category
                } else {
                    CategoryId::GENERAL
                };
                self.assign_category(id, category, place)
            }
            Err(err) => {
                tracing::warn!(%id, error = %err, "could not move the file");
                self.emit_changed(id);
                Err(Error::MoveFailed(err))
            }
        };
        let _ = moving.reply.send(answer);
        let Some(state) = self.jobs.get(&id).map(|job| job.state) else {
            return;
        };
        if self.shutdown.is_none() && state.is_running() {
            self.spawn_task(id);
        } else if state == JobState::Retrying {
            // Its wait may have ended while it was moving.
            self.retry_when_due(id);
        }
        self.schedule();
    }

    /// Puts the job in `category` as the user's choice, and at `place` if
    /// given. The caller schedules (a category cap may have changed).
    fn assign_category(
        &mut self,
        id: JobId,
        category: CategoryId,
        place: Option<Place>,
    ) -> Result<(), Error> {
        self.change(id, |job| {
            job.category = category;
            job.category_chosen = true;
            if let Some((dir, name)) = place {
                job.download.dest_dir = dir;
                if name.is_some() {
                    job.download.file_name = name;
                }
            }
        })?;
        self.limiter.set_category(id, category);
        Ok(())
    }

    fn save_category(&self, id: CategoryId) {
        let Some(settings) = self.categories.get(&id) else {
            return;
        };
        let category = Category {
            id,
            settings: settings.clone(),
            built_in: self.built_in(id),
        };
        self.store.save_category(category.clone());
        // Fails only when nobody is subscribed.
        let _ = self.events.send(JobEvent::CategoryChanged(category));
    }

    fn views(&self) -> Vec<View> {
        self.views
            .iter()
            .map(|(&id, settings)| View {
                id,
                settings: settings.clone(),
            })
            .collect()
    }

    /// Normalizes `settings` and checks that its name isn't another view's
    /// (`own` is the one being edited, if any) and its categories exist.
    fn check_view(
        &self,
        settings: ViewSettings,
        own: Option<ViewId>,
    ) -> Result<ViewSettings, Error> {
        let settings = settings.normalized()?;
        let name = settings.name.to_lowercase();
        if self
            .views
            .iter()
            .any(|(&id, other)| Some(id) != own && other.name.to_lowercase() == name)
        {
            return Err(Error::ViewNameTaken(settings.name));
        }
        for &category in settings.categories.iter().flatten() {
            self.category(category)?;
        }
        Ok(settings)
    }

    fn add_view(&mut self, settings: ViewSettings) -> Result<ViewId, Error> {
        let settings = self.check_view(settings, None)?;
        let last = self.views.keys().next_back().map_or(0, |id| id.0);
        let id = ViewId(last.checked_add(1).ok_or(Error::TooManyViews)?);
        self.views.insert(id, settings);
        self.save_view(id);
        Ok(id)
    }

    fn update_view(&mut self, id: ViewId, settings: ViewSettings) -> Result<(), Error> {
        if !self.views.contains_key(&id) {
            return Err(Error::ViewNotFound(id));
        }
        let settings = self.check_view(settings, Some(id))?;
        self.views.insert(id, settings);
        self.save_view(id);
        Ok(())
    }

    fn remove_view(&mut self, id: ViewId) -> Result<(), Error> {
        self.views.remove(&id).ok_or(Error::ViewNotFound(id))?;
        self.store.remove_view(id);
        // Fails only when nobody is subscribed.
        let _ = self.events.send(JobEvent::ViewRemoved(id));
        Ok(())
    }

    fn save_view(&self, id: ViewId) {
        let Some(settings) = self.views.get(&id) else {
            return;
        };
        let view = View {
            id,
            settings: settings.clone(),
        };
        self.store.save_view(view.clone());
        // Fails only when nobody is subscribed.
        let _ = self.events.send(JobEvent::ViewChanged(view));
    }

    async fn stop(&mut self, id: JobId, stop: Stop, reply: Reply<()>) {
        let Some(job) = self.jobs.get_mut(&id) else {
            let _ = reply.send(Err(Error::JobNotFound(id)));
            return;
        };
        if job.moving.is_some() && stop != Stop::Shutdown {
            let _ = reply.send(Err(Error::JobMoving(id)));
            return;
        }
        let Some(target) = stop.target() else {
            let _ = reply.send(Ok(()));
            return;
        };
        if let Some(task) = &mut job.task {
            // Running: ask it to stop, and answer once it has (see `on_ended`).
            task.stop = task.stop.max(Some(stop));
            task.cancel.cancel();
            task.waiters.push((stop, reply));
            return;
        }
        let finished = matches!(job.state, JobState::Completed | JobState::Cancelled);
        let result = if stop == Stop::Remove && finished {
            Ok(())
        } else {
            self.transition(id, target, None)
        };
        if result.is_ok() {
            if matches!(stop, Stop::Cancel | Stop::Remove) && !finished {
                self.delete_part(id).await;
            }
            if stop == Stop::Remove {
                self.forget(id);
            }
            // A retrying job held a slot, which is now free.
            self.schedule();
        }
        let _ = reply.send(result);
    }

    fn resume(&mut self, id: JobId) -> Result<(), Error> {
        if self.shutdown.is_some() {
            return Err(Error::ManagerStopped);
        }
        if self.jobs.get(&id).is_some_and(|job| job.moving.is_some()) {
            return Err(Error::JobMoving(id));
        }
        self.transition(id, JobState::Queued, None)?;
        // Resumed by hand: every retry is available again.
        if let Some(job) = self.jobs.get_mut(&id) {
            job.retries = 0;
            job.gave_way = 0;
        }
        self.schedule();
        Ok(())
    }

    /// Applies `change`; if anything changed, saves the settings, puts them
    /// to use and tells the subscribers.
    fn change_settings(&mut self, change: SettingsChange) -> Settings {
        let before = self.settings.clone();
        self.settings.apply(change);
        if self.settings != before {
            self.store.save_settings(self.settings.clone());
            if self.settings.global_limit != before.global_limit {
                self.limiter.set_global(self.settings.global_limit);
            }
            // More slots, or retrying jobs that may now give way.
            self.schedule();
            let _ = self
                .events
                .send(JobEvent::SettingsChanged(self.settings.clone()));
        }
        self.settings.clone()
    }

    fn list(&self) -> Vec<JobSnapshot> {
        self.jobs
            .iter()
            .map(|(id, job)| job.snapshot(*id))
            .collect()
    }

    /// Starts queued jobs until `max_concurrent` are running, skipping jobs
    /// whose category is at its own cap: the highest priority first, and the
    /// oldest first within a priority.
    ///
    /// When every slot is taken, a `Retrying` job that has used its in-slot
    /// retries gives its slot to a queued job, and queues behind the others.
    fn schedule(&mut self) {
        if self.shutdown.is_some() {
            return;
        }
        loop {
            let mut busy: HashMap<CategoryId, usize> = HashMap::new();
            for job in self.jobs.values().filter(|j| j.holds_slot()) {
                *busy.entry(job.category).or_default() += 1;
            }
            if busy.values().sum::<usize>() < self.settings.max_concurrent {
                let Some(id) = self.next_queued(&busy) else {
                    break;
                };
                if !self.start_job(id) {
                    break;
                }
            } else {
                let Some(id) = self.ready_to_give_way(&busy) else {
                    break;
                };
                self.yields += 1;
                let problem = self.jobs.get_mut(&id).and_then(|job| {
                    job.gave_way = self.yields;
                    job.problem.clone()
                });
                if !self.transition_or_log(id, JobState::Queued, problem) {
                    break;
                }
            }
        }
    }

    /// The queued job to start next, among those whose category has room:
    /// jobs that gave their slot away come last (whatever their priority, or
    /// they would take it straight back); then the highest priority, the oldest.
    fn next_queued(&self, busy: &HashMap<CategoryId, usize>) -> Option<JobId> {
        self.jobs
            .iter()
            .filter(|(_, job)| {
                job.state == JobState::Queued
                    && job.moving.is_none()
                    && self.has_room(job.category, busy)
            })
            // `Reverse`: the smallest `gave_way` and ID win.
            .max_by_key(|(id, job)| {
                let never_gave_way = job.gave_way == 0;
                (
                    never_gave_way,
                    job.priority,
                    Reverse(job.gave_way),
                    Reverse(**id),
                )
            })
            .map(|(id, _)| *id)
    }

    /// A `Retrying` job past its in-slot retries whose slot a queued job could use.
    fn ready_to_give_way(&self, busy: &HashMap<CategoryId, usize>) -> Option<JobId> {
        let in_slot = self.settings.retry.retries_before_yield;
        self.jobs
            .iter()
            .filter(|(_, job)| {
                job.state == JobState::Retrying && job.retries > in_slot && job.moving.is_none()
            })
            .find(|(_, job)| {
                let mut after = busy.clone();
                if let Some(count) = after.get_mut(&job.category) {
                    *count = count.saturating_sub(1);
                }
                self.jobs.values().any(|waiting| {
                    waiting.state == JobState::Queued && self.has_room(waiting.category, &after)
                })
            })
            .map(|(id, _)| *id)
    }

    fn has_room(&self, category: CategoryId, busy: &HashMap<CategoryId, usize>) -> bool {
        let cap = self
            .categories
            .get(&category)
            .map_or(0, |c| c.max_concurrent);
        cap == 0 || busy.get(&category).copied().unwrap_or(0) < cap
    }

    fn start_job(&mut self, id: JobId) -> bool {
        if !self.transition_or_log(id, JobState::Connecting, None) {
            return false;
        }
        self.spawn_task(id)
    }

    /// Starts the task that downloads the job, in the state it's in.
    fn spawn_task(&mut self, id: JobId) -> bool {
        let placement = self.jobs.get(&id).and_then(|job| self.placement(job));
        let Some(job) = self.jobs.get_mut(&id) else {
            return false;
        };
        let cancel = CancellationToken::new();
        job.task = Some(Task {
            cancel: cancel.clone(),
            stop: None,
            waiters: Vec::new(),
        });
        // The task works on its own copy and hands it back when it ends.
        let mut download = job.download.clone();
        download.placement = placement;
        let backend = Arc::clone(&self.backend);
        let internal = self.internal.clone();
        // The job's speed budget lives as long as the task does.
        let throttle = self
            .limiter
            .register(id, job.speed_limit, job.priority, job.category);
        let playback = self.playbacks.of(id);
        playback.set_streaming(job.streaming);

        tokio::spawn(async move {
            let mut last_progress: Option<Instant> = None;
            // Sends fail only if the manager is gone, and then nobody's listening.
            let mut on_event = |event: Event| {
                if let Event::Progress { .. } = event {
                    if last_progress.is_some_and(|at| at.elapsed() < PROGRESS_INTERVAL) {
                        return;
                    }
                    last_progress = Some(Instant::now());
                }
                let _ = internal.send(Internal::Event { id, event });
            };
            let result = backend
                .run(&mut download, &throttle, &playback, &cancel, &mut on_event)
                .await;
            drop(throttle);
            let _ = internal.send(Internal::Ended {
                id,
                download: Box::new(download),
                result,
            });
        });
        true
    }

    async fn on_ended(
        &mut self,
        id: JobId,
        download: Box<Download>,
        result: Result<Outcome, Error>,
    ) {
        let Some(job) = self.jobs.get_mut(&id) else {
            return;
        };
        let task = job.task.take();
        job.download = *download;
        job.download.placement = None;
        let stop = task.as_ref().and_then(|t| t.stop);

        let next = match (result, stop) {
            // Finished before it noticed the stop request: keep the file.
            (Ok(Outcome::Completed { .. }), _) => Some((JobState::Completed, None)),
            (_, Some(stop)) => stop.target().map(|state| (state, None)),
            // Only the manager cancels the token, so this means paused.
            (Ok(Outcome::Paused), None) => Some((JobState::Paused, None)),
            (Err(err), None)
                if err.is_retryable() && job.retries < self.settings.retry.max_retries =>
            {
                job.retries += 1;
                let wait = retry_wait(&self.settings.retry, job.retries, err.retry_after());
                tracing::info!(%id, error = %err, retry = job.retries, ?wait, "download failed; retrying");
                job.retry_at = Some((Instant::now() + wait, SystemTime::now() + wait));
                Some((JobState::Retrying, Some(Problem::from(&err))))
            }
            (Err(err), None) => {
                tracing::warn!(%id, error = %err, "download failed");
                Some((JobState::Failed, Some(Problem::from(&err))))
            }
        };
        match next {
            Some((state, problem)) => {
                self.transition_or_log(id, state, problem);
                match state {
                    JobState::Cancelled => self.delete_part(id).await,
                    JobState::Retrying => self.retry_when_due(id),
                    _ => {}
                }
            }
            // Shutting down, or stopped to move its file: keep the running state
            // (the next start continues it, or `finish_move` does).
            None => self.save(id),
        }

        let Some(final_state) = self.jobs.get(&id).map(|job| job.state) else {
            return;
        };
        // Forgotten before answering, so a caller that lists next won't see it.
        let removed = stop == Some(Stop::Remove);
        if removed {
            self.forget(id);
        }
        for (asked, reply) in task.map(|t| t.waiters).unwrap_or_default() {
            let answer = match asked.target() {
                _ if removed && asked == Stop::Remove => Ok(()),
                _ if removed => Err(Error::JobNotFound(id)),
                Some(wanted) if wanted != final_state => Err(Error::InvalidTransition {
                    from: final_state,
                    to: wanted,
                }),
                _ => Ok(()),
            };
            let _ = reply.send(answer);
        }
        // Stopped to move its file (or finished first): move it now, unless
        // Ottie is shutting down.
        if self.shutdown.is_some() {
            if let Some(moving) = self.jobs.get_mut(&id).and_then(|job| job.moving.take()) {
                let _ = moving.reply.send(Err(Error::ManagerStopped));
            }
        } else {
            self.move_file(id);
        }
        self.schedule();
    }

    fn begin_shutdown(&mut self, reply: Option<Reply<()>>) {
        self.shutdown.get_or_insert_with(Vec::new).extend(reply);
        for job in self.jobs.values_mut() {
            if let Some(task) = &mut job.task {
                task.stop = task.stop.max(Some(Stop::Shutdown));
                task.cancel.cancel();
            }
        }
    }

    /// Includes files being moved, which shutdown waits for too.
    fn any_task_running(&self) -> bool {
        self.jobs
            .values()
            .any(|job| job.task.is_some() || job.moving.is_some())
    }

    /// The only place a job changes state. Refuses moves the state machine
    /// doesn't allow; saves and tells subscribers about the others.
    fn transition(
        &mut self,
        id: JobId,
        next: JobState,
        problem: Option<Problem>,
    ) -> Result<(), Error> {
        let job = self.jobs.get_mut(&id).ok_or(Error::JobNotFound(id))?;
        if !job.state.can_become(next) {
            return Err(Error::InvalidTransition {
                from: job.state,
                to: next,
            });
        }
        job.state = next;
        job.problem = problem;
        if next != JobState::Retrying {
            job.retry_at = None;
        }
        self.save(id);
        self.emit_changed(id);
        Ok(())
    }

    /// For transitions the manager itself makes, which should always be valid.
    fn transition_or_log(&mut self, id: JobId, next: JobState, problem: Option<Problem>) -> bool {
        match self.transition(id, next, problem) {
            Ok(()) => true,
            Err(err) => {
                tracing::error!(%id, error = %err, "manager made an invalid transition");
                false
            }
        }
    }

    fn save(&mut self, id: JobId) {
        let Some(job) = self.jobs.get_mut(&id) else {
            return;
        };
        job.last_saved = Some(Instant::now());
        self.store.save(JobRecord {
            id,
            state: job.state,
            download: job.download.clone(),
            problem: job.problem.clone(),
            priority: job.priority,
            speed_limit: job.speed_limit,
            category: job.category,
            category_chosen: job.category_chosen,
            streaming: job.streaming,
            streaming_chosen: job.streaming_chosen,
        });
    }

    /// Takes a stopped job off the list and out of the database.
    fn forget(&mut self, id: JobId) {
        if self.jobs.remove(&id).is_none() {
            return;
        }
        // The generator has handed out at least this job's ID.
        let last_id = self.ids.last().unwrap_or(id);
        self.store.remove_job(id, last_id);
        // Fails only when nobody is subscribed.
        let _ = self.events.send(JobEvent::Removed(id));
    }

    fn emit_changed(&self, id: JobId) {
        if let Some(job) = self.jobs.get(&id) {
            let snapshot = job.snapshot(id);
            // Fails only when nobody is subscribed.
            let _ = self.events.send(JobEvent::Changed(snapshot));
        }
    }

    /// Wakes the actor when the job's retry is due.
    fn retry_when_due(&self, id: JobId) {
        let Some((due, _)) = self.jobs.get(&id).and_then(|job| job.retry_at) else {
            return;
        };
        let internal = self.internal.clone();
        tokio::spawn(async move {
            tokio::time::sleep_until(due.into()).await;
            // Fails only if the manager has stopped.
            let _ = internal.send(Internal::RetryDue { id, due });
        });
    }

    async fn delete_part(&self, id: JobId) {
        let Some(part) = self.jobs.get(&id).and_then(|job| job.download.part_path()) else {
            return;
        };
        if let Err(err) = tokio::fs::remove_file(&part).await
            && err.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(%id, error = %err, "could not delete .part.ottie file");
        }
    }
}

/// How long to wait before retry number `retry` (1 for the first): the
/// policy's first wait, doubled each time up to a minute, or longer if the
/// server asked for that with `Retry-After`.
fn retry_wait(policy: &RetryPolicy, retry: u32, asked: Option<Duration>) -> Duration {
    let doublings = retry.saturating_sub(1).min(16);
    let cap = MAX_RETRY_WAIT.max(policy.first_wait);
    let backoff = policy.first_wait.saturating_mul(1 << doublings).min(cap);
    let wait = asked.map_or(backoff, |asked| asked.min(MAX_RETRY_AFTER).max(backoff));
    wait.min(LONGEST_RETRY_WAIT)
}

/// The category that lists `ext`, if any. Takes the map rather than `&self`
/// so it can be used while a job is borrowed.
fn owner_of(categories: &BTreeMap<CategoryId, CategorySettings>, ext: &str) -> Option<CategoryId> {
    categories
        .iter()
        .find(|(_, settings)| settings.extensions.iter().any(|e| e == ext))
        .map(|(id, _)| *id)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_waits_double_up_to_a_minute() {
        let policy = RetryPolicy::default(); // first wait 2 s
        let secs = |retry, asked| retry_wait(&policy, retry, asked).as_secs();
        assert_eq!(
            [1, 2, 3, 4, 5, 6, 7].map(|r| secs(r, None)),
            [2, 4, 8, 16, 32, 60, 60]
        );
        assert_eq!(secs(40, None), 60, "no overflow");
        // Retry-After can ask for longer, never shorter, and not forever.
        assert_eq!(secs(1, Some(Duration::from_secs(30))), 30);
        assert_eq!(secs(5, Some(Duration::from_secs(1))), 32);
        assert_eq!(secs(1, Some(Duration::from_secs(86_400))), 600);
    }

    #[test]
    fn a_long_first_wait_is_not_capped() {
        let policy = RetryPolicy {
            first_wait: Duration::from_secs(90),
            ..RetryPolicy::default()
        };
        assert_eq!(retry_wait(&policy, 3, None), Duration::from_secs(90));
        let absurd = RetryPolicy {
            first_wait: Duration::MAX,
            ..RetryPolicy::default()
        };
        assert_eq!(retry_wait(&absurd, 1, None), LONGEST_RETRY_WAIT);
    }
}
