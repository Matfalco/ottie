//! The job manager. One task (the "actor") owns every job; everything else
//! sends it commands through a [`Manager`] handle and listens to [`JobEvent`]s.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::Url;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::backend::Backend;
use crate::category::{self, Category, CategoryId, CategorySettings};
use crate::download::{Download, Event, Outcome, Placement};
use crate::error::Error;
use crate::filename;
use crate::id::{IdGenerator, JobId};
use crate::job::{JobEvent, JobSnapshot, JobState, NewJob, Priority};
use crate::store::{JobRecord, Settings, Store};
use crate::throttle::Limiter;

/// Running jobs report progress to the manager at most this often.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// Progress of a running job is saved at most this often (state changes are
/// saved immediately).
const SAVE_INTERVAL: Duration = Duration::from_secs(2);
const COMMAND_BUFFER: usize = 64;
/// Subscribers that fall further behind than this miss events (and should
/// call [`Manager::list`] to catch up).
const EVENT_BUFFER: usize = 1024;

/// Handle to the job manager. Cheap to clone; every clone talks to the same jobs.
#[derive(Clone)]
pub struct Manager {
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<JobEvent>,
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
        for category in loaded.categories {
            limiter.set_category_limit(category.id, category.settings.speed_limit);
            categories.insert(category.id, category.settings);
        }
        let (commands, command_rx) = mpsc::channel(COMMAND_BUFFER);
        let (internal, internal_rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let actor = Actor {
            backend: Arc::new(backend),
            store,
            limiter,
            settings,
            ids: IdGenerator::after(jobs.keys().next_back().copied()),
            jobs,
            categories,
            events: events.clone(),
            internal,
            shutdown: None,
        };
        tokio::spawn(actor.run(command_rx, internal_rx));
        Ok(Manager { commands, events })
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

    /// Every job, oldest first.
    pub async fn list(&self) -> Result<Vec<JobSnapshot>, Error> {
        self.request(|reply| Command::List { reply }).await
    }

    pub async fn settings(&self) -> Result<Settings, Error> {
        self.request(|reply| Command::Settings { reply }).await
    }

    /// Takes effect immediately and is saved. Lowering it doesn't stop
    /// running jobs; it only delays new ones. At least 1.
    pub async fn set_max_concurrent(&self, max: usize) -> Result<(), Error> {
        self.request(|reply| Command::SetMaxConcurrent { max, reply })
            .await
    }

    /// Bytes per second for all downloads together; 0 = unlimited. Takes
    /// effect immediately and is saved.
    pub async fn set_global_limit(&self, bytes_per_sec: u64) -> Result<(), Error> {
        self.request(|reply| Command::SetGlobalLimit {
            bytes_per_sec,
            reply,
        })
        .await
    }

    /// One job's own limit in bytes per second; 0 = none (the global limit
    /// still applies). Takes effect immediately, even while it runs.
    pub async fn set_speed_limit(&self, id: JobId, bytes_per_sec: u64) -> Result<(), Error> {
        self.request(|reply| Command::SetSpeedLimit {
            id,
            bytes_per_sec,
            reply,
        })
        .await
    }

    /// Changes the job's share of the global limit and its place in the queue.
    pub async fn set_priority(&self, id: JobId, priority: Priority) -> Result<(), Error> {
        self.request(|reply| Command::SetPriority {
            id,
            priority,
            reply,
        })
        .await
    }

    /// Every category, by ID. General (ID 1) is always first.
    pub async fn categories(&self) -> Result<Vec<Category>, Error> {
        self.request(|reply| Command::Categories { reply }).await
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

    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.events.subscribe()
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
    Settings {
        reply: Reply<Settings>,
    },
    SetMaxConcurrent {
        max: usize,
        reply: Reply<()>,
    },
    SetGlobalLimit {
        bytes_per_sec: u64,
        reply: Reply<()>,
    },
    SetSpeedLimit {
        id: JobId,
        bytes_per_sec: u64,
        reply: Reply<()>,
    },
    SetPriority {
        id: JobId,
        priority: Priority,
        reply: Reply<()>,
    },
    Categories {
        reply: Reply<Vec<Category>>,
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
}

/// Why a running job is being stopped. Later variants win: a user's pause or
/// cancel overrides the shutdown, and cancel overrides pause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Stop {
    /// Leave the state as it is, so the next start continues the job.
    Shutdown,
    Pause,
    Cancel,
}

impl Stop {
    fn target(self) -> Option<JobState> {
        match self {
            Stop::Shutdown => None,
            Stop::Pause => Some(JobState::Paused),
            Stop::Cancel => Some(JobState::Cancelled),
        }
    }
}

struct Job {
    state: JobState,
    download: Download,
    error: Option<String>,
    priority: Priority,
    /// Bytes per second; 0 = no limit of its own.
    speed_limit: u64,
    category: CategoryId,
    /// The user picked the category, rather than it being matched by extension.
    category_chosen: bool,
    /// Present while a task is downloading this job.
    task: Option<Task>,
    last_saved: Option<Instant>,
}

impl Job {
    async fn restore(record: JobRecord) -> Job {
        let mut state = record.state;
        let mut download = record.download;
        // Running when Ottie stopped, whether it quit or crashed: continue it.
        if state.is_running() {
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
            error: record.error,
            priority: record.priority,
            speed_limit: record.speed_limit,
            category: record.category,
            category_chosen: record.category_chosen,
            task: None,
            last_saved: None,
        }
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
            error: self.error.clone(),
            priority: self.priority,
            speed_limit: self.speed_limit,
            category: self.category,
        }
    }
}

struct Task {
    cancel: CancellationToken,
    stop: Option<Stop>,
    /// Pause/cancel requests to answer once the task has really stopped.
    waiters: Vec<(JobState, Reply<()>)>,
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
    /// Always holds General. Saved whenever one changes.
    categories: BTreeMap<CategoryId, CategorySettings>,
    ids: IdGenerator,
    events: broadcast::Sender<JobEvent>,
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
            Command::Settings { reply } => {
                let _ = reply.send(Ok(self.settings.clone()));
            }
            Command::SetMaxConcurrent { max, reply } => {
                self.settings.max_concurrent = max.max(1);
                self.store.save_settings(self.settings.clone());
                self.schedule();
                let _ = reply.send(Ok(()));
            }
            Command::SetGlobalLimit {
                bytes_per_sec,
                reply,
            } => {
                self.settings.global_limit = bytes_per_sec;
                self.store.save_settings(self.settings.clone());
                self.limiter.set_global(bytes_per_sec);
                let _ = reply.send(Ok(()));
            }
            Command::SetSpeedLimit {
                id,
                bytes_per_sec,
                reply,
            } => {
                let _ = reply.send(self.change(id, |job| job.speed_limit = bytes_per_sec));
                self.limiter.set_limit(id, bytes_per_sec);
            }
            Command::SetPriority {
                id,
                priority,
                reply,
            } => {
                let _ = reply.send(self.change(id, |job| job.priority = priority));
                self.limiter.set_priority(id, priority);
            }
            Command::Categories { reply } => {
                let _ = reply.send(Ok(self.categories()));
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
            } => {
                let _ = reply.send(self.set_category(id, category));
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
                // may belong to another category than the URL suggested.
                if job.download.file_name.is_none()
                    && !job.category_chosen
                    && let Some(ext) = filename::extension(&file_name)
                {
                    let category = owner_of(&self.categories, &ext).unwrap_or(CategoryId::GENERAL);
                    if category != job.category {
                        job.category = category;
                        self.limiter.set_category(id, category);
                    }
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
            Event::Progress { downloaded, total } => {
                job.download.downloaded = downloaded;
                job.download.total = total;
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
        let category = match new.category {
            Some(category) => category,
            None => self.match_category(&new.url),
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
        let download = Download::new(new.url, dest_dir)?;
        let id = self.ids.next(unix_now());
        self.jobs.insert(
            id,
            Job {
                state: JobState::Queued,
                download,
                error: None,
                priority: Priority::default(),
                speed_limit: 0,
                category,
                category_chosen: new.category.is_some(),
                task: None,
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

    fn update_category(&mut self, id: CategoryId, settings: CategorySettings) -> Result<(), Error> {
        self.category(id)?;
        let settings = self.check_category(settings, Some(id))?;
        self.limiter.set_category_limit(id, settings.speed_limit);
        self.categories.insert(id, settings);
        self.save_category(id);
        // A higher cap on running jobs may let queued ones start.
        self.schedule();
        Ok(())
    }

    fn remove_category(&mut self, id: CategoryId) -> Result<(), Error> {
        if id == CategoryId::GENERAL {
            return Err(Error::CannotRemoveGeneral);
        }
        self.category(id)?;
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
        self.categories.remove(&id);
        self.limiter.set_category_limit(id, 0);
        self.store.remove_category(id);
        // Fails only when nobody is subscribed.
        let _ = self.events.send(JobEvent::CategoryRemoved(id));
        self.schedule();
        Ok(())
    }

    fn set_category(&mut self, id: JobId, category: CategoryId) -> Result<(), Error> {
        let folder = self.category(category)?.folder.clone();
        self.change(id, |job| {
            job.category = category;
            job.category_chosen = true;
            // Nothing on disk yet, and no task holding its own copy of the
            // download: it can still be saved somewhere else.
            if let Some(folder) = folder
                && job.download.file_name.is_none()
                && job.task.is_none()
            {
                job.download.dest_dir = folder;
            }
        })?;
        self.limiter.set_category(id, category);
        self.schedule();
        Ok(())
    }

    fn save_category(&self, id: CategoryId) {
        let Some(settings) = self.categories.get(&id) else {
            return;
        };
        let category = Category {
            id,
            settings: settings.clone(),
        };
        self.store.save_category(category.clone());
        // Fails only when nobody is subscribed.
        let _ = self.events.send(JobEvent::CategoryChanged(category));
    }

    async fn stop(&mut self, id: JobId, stop: Stop, reply: Reply<()>) {
        let Some(job) = self.jobs.get_mut(&id) else {
            let _ = reply.send(Err(Error::JobNotFound(id)));
            return;
        };
        let Some(target) = stop.target() else {
            let _ = reply.send(Ok(()));
            return;
        };
        if let Some(task) = &mut job.task {
            // Running: ask it to stop, and answer once it has (see `on_ended`).
            task.stop = task.stop.max(Some(stop));
            task.cancel.cancel();
            task.waiters.push((target, reply));
            return;
        }
        let result = self.transition(id, target, None);
        if result.is_ok() && stop == Stop::Cancel {
            self.delete_part(id).await;
        }
        let _ = reply.send(result);
    }

    fn resume(&mut self, id: JobId) -> Result<(), Error> {
        if self.shutdown.is_some() {
            return Err(Error::ManagerStopped);
        }
        self.transition(id, JobState::Queued, None)?;
        self.schedule();
        Ok(())
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
    fn schedule(&mut self) {
        if self.shutdown.is_some() {
            return;
        }
        let mut running: HashMap<CategoryId, usize> = HashMap::new();
        for job in self.jobs.values().filter(|j| j.task.is_some()) {
            *running.entry(job.category).or_default() += 1;
        }
        let mut total: usize = running.values().sum();
        while total < self.settings.max_concurrent {
            let has_room = |category: CategoryId| {
                let cap = self
                    .categories
                    .get(&category)
                    .map_or(0, |c| c.max_concurrent);
                cap == 0 || running.get(&category).copied().unwrap_or(0) < cap
            };
            let next = self
                .jobs
                .iter()
                .filter(|(_, job)| job.state == JobState::Queued && has_room(job.category))
                // `Reverse(id)`: among equal priorities, the smallest (oldest) ID wins.
                .max_by_key(|(id, job)| (job.priority, Reverse(**id)))
                .map(|(id, job)| (*id, job.category));
            let Some((id, category)) = next else { break };
            if !self.start_job(id) {
                break;
            }
            *running.entry(category).or_default() += 1;
            total += 1;
        }
    }

    fn start_job(&mut self, id: JobId) -> bool {
        if !self.transition_or_log(id, JobState::Connecting, None) {
            return false;
        }
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
                .run(&mut download, &throttle, &cancel, &mut on_event)
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
            (Err(err), None) => {
                tracing::warn!(%id, error = %err, "download failed");
                Some((JobState::Failed, Some(err.to_string())))
            }
        };
        match next {
            Some((state, error)) => {
                self.transition_or_log(id, state, error);
                if state == JobState::Cancelled {
                    self.delete_part(id).await;
                }
            }
            // Shutting down: keep the running state so the next start continues it.
            None => self.save(id),
        }

        let Some(final_state) = self.jobs.get(&id).map(|job| job.state) else {
            return;
        };
        for (wanted, reply) in task.map(|t| t.waiters).unwrap_or_default() {
            let _ = reply.send(if wanted == final_state {
                Ok(())
            } else {
                Err(Error::InvalidTransition {
                    from: final_state,
                    to: wanted,
                })
            });
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

    fn any_task_running(&self) -> bool {
        self.jobs.values().any(|job| job.task.is_some())
    }

    /// The only place a job changes state. Refuses moves the state machine
    /// doesn't allow; saves and tells subscribers about the others.
    fn transition(
        &mut self,
        id: JobId,
        next: JobState,
        error: Option<String>,
    ) -> Result<(), Error> {
        let job = self.jobs.get_mut(&id).ok_or(Error::JobNotFound(id))?;
        if !job.state.can_become(next) {
            return Err(Error::InvalidTransition {
                from: job.state,
                to: next,
            });
        }
        job.state = next;
        job.error = error;
        self.save(id);
        self.emit_changed(id);
        Ok(())
    }

    /// For transitions the manager itself makes, which should always be valid.
    fn transition_or_log(&mut self, id: JobId, next: JobState, error: Option<String>) -> bool {
        match self.transition(id, next, error) {
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
            error: job.error.clone(),
            priority: job.priority,
            speed_limit: job.speed_limit,
            category: job.category,
            category_chosen: job.category_chosen,
        });
    }

    fn emit_changed(&self, id: JobId) {
        if let Some(job) = self.jobs.get(&id) {
            let snapshot = job.snapshot(id);
            // Fails only when nobody is subscribed.
            let _ = self.events.send(JobEvent::Changed(snapshot));
        }
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
