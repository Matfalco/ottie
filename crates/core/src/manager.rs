//! The job manager. One task (the "actor") owns every job; everything else
//! sends it commands through a [`Manager`] handle and listens to [`JobEvent`]s.

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::Url;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::backend::Backend;
use crate::download::{Download, Event, Outcome};
use crate::error::Error;
use crate::id::{IdGenerator, JobId};
use crate::job::{JobEvent, JobSnapshot, JobState};
use crate::store::{JobRecord, Store};

/// Running jobs report progress to the manager at most this often.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// Progress of a running job is saved at most this often (state changes are
/// saved immediately).
const SAVE_INTERVAL: Duration = Duration::from_secs(2);
const COMMAND_BUFFER: usize = 64;
/// Subscribers that fall further behind than this miss events (and should
/// call [`Manager::list`] to catch up).
const EVENT_BUFFER: usize = 1024;

#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// How many jobs may download at once. At least 1.
    pub max_concurrent: usize,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self { max_concurrent: 3 }
    }
}

/// Handle to the job manager. Cheap to clone; every clone talks to the same jobs.
#[derive(Clone)]
pub struct Manager {
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<JobEvent>,
}

impl Manager {
    /// Loads saved jobs from `store` and starts the manager task. Jobs that
    /// were running when Ottie last stopped (quit or crash) are queued again.
    /// Must be called from inside a tokio runtime. When every handle is
    /// dropped, the manager shuts down as if [`Manager::shutdown`] was called.
    pub async fn start(
        backend: impl Backend,
        store: Store,
        config: ManagerConfig,
    ) -> Result<Manager, Error> {
        let mut jobs = BTreeMap::new();
        for record in store.load().await? {
            jobs.insert(record.id, Job::restore(record).await);
        }
        let (commands, command_rx) = mpsc::channel(COMMAND_BUFFER);
        let (internal, internal_rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let actor = Actor {
            backend: Arc::new(backend),
            store,
            max_concurrent: config.max_concurrent.max(1),
            ids: IdGenerator::after(jobs.keys().next_back().copied()),
            jobs,
            events: events.clone(),
            internal,
            shutdown: None,
        };
        tokio::spawn(actor.run(command_rx, internal_rx));
        Ok(Manager { commands, events })
    }

    /// Queues a download into `dest_dir`. It starts as soon as a slot is free.
    pub async fn add(&self, url: Url, dest_dir: impl Into<PathBuf>) -> Result<JobId, Error> {
        let dest_dir = dest_dir.into();
        self.request(|reply| Command::Add {
            url,
            dest_dir,
            reply,
        })
        .await
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

    /// Takes effect immediately. Lowering it doesn't stop running jobs; it
    /// only delays new ones.
    pub async fn set_max_concurrent(&self, max: usize) -> Result<(), Error> {
        self.request(|reply| Command::SetMaxConcurrent { max, reply })
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
        url: Url,
        dest_dir: PathBuf,
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
    SetMaxConcurrent {
        max: usize,
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
            task: None,
            last_saved: None,
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
    max_concurrent: usize,
    /// Sorted by ID, which means by creation time.
    jobs: BTreeMap<JobId, Job>,
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
            Command::Add {
                url,
                dest_dir,
                reply,
            } => {
                let _ = reply.send(self.add(url, dest_dir));
            }
            Command::Stop { id, stop, reply } => self.stop(id, stop, reply).await,
            Command::Resume { id, reply } => {
                let _ = reply.send(self.resume(id));
            }
            Command::List { reply } => {
                let _ = reply.send(Ok(self.list()));
            }
            Command::SetMaxConcurrent { max, reply } => {
                self.max_concurrent = max.max(1);
                self.schedule();
                let _ = reply.send(Ok(()));
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
                total,
                validator,
                resumable,
                downloaded,
                segments,
            } => {
                let dl = &mut job.download;
                dl.file_name = Some(file_name);
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

    fn add(&mut self, url: Url, dest_dir: PathBuf) -> Result<JobId, Error> {
        if self.shutdown.is_some() {
            return Err(Error::ManagerStopped);
        }
        if dest_dir.to_str().is_none() {
            return Err(Error::NonUtf8Path(dest_dir));
        }
        let download = Download::new(url, dest_dir)?;
        let id = self.ids.next(unix_now());
        self.jobs.insert(
            id,
            Job {
                state: JobState::Queued,
                download,
                error: None,
                task: None,
                last_saved: None,
            },
        );
        self.save(id);
        self.emit_changed(id);
        self.schedule();
        Ok(id)
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
            .map(|(id, job)| JobSnapshot::new(*id, job.state, &job.download, job.error.clone()))
            .collect()
    }

    /// Starts the oldest queued jobs until `max_concurrent` are running.
    fn schedule(&mut self) {
        if self.shutdown.is_some() {
            return;
        }
        let mut running = self.jobs.values().filter(|j| j.task.is_some()).count();
        while running < self.max_concurrent {
            let next = self
                .jobs
                .iter()
                .find(|(_, job)| job.state == JobState::Queued)
                .map(|(id, _)| *id);
            let Some(id) = next else { break };
            if !self.start_job(id) {
                break;
            }
            running += 1;
        }
    }

    fn start_job(&mut self, id: JobId) -> bool {
        if !self.transition_or_log(id, JobState::Connecting, None) {
            return false;
        }
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
        let backend = Arc::clone(&self.backend);
        let internal = self.internal.clone();

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
            let result = backend.run(&mut download, &cancel, &mut on_event).await;
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
        });
    }

    fn emit_changed(&self, id: JobId) {
        if let Some(job) = self.jobs.get(&id) {
            let snapshot = JobSnapshot::new(id, job.state, &job.download, job.error.clone());
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

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
