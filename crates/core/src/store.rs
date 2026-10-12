//! SQLite persistence. SQLite calls block, so the connection lives on its own
//! thread and the manager talks to it through a channel. See docs/decisions/0008.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;

use reqwest::Url;
use rusqlite::{Connection, Row, params};
use tokio::sync::oneshot;

use crate::category::{Category, CategoryId, CategorySettings};
use crate::download::{Download, Segment, Validator};
use crate::error::Error;
use crate::id::JobId;
use crate::job::{JobState, Priority};

/// Each entry upgrades the schema by one version (`PRAGMA user_version`).
/// Never edit an entry once released; append a new one.
const MIGRATIONS: &[&str] = &[
    // 1: jobs
    "CREATE TABLE jobs (
        id_secs         INTEGER NOT NULL,
        id_seq          INTEGER NOT NULL,
        url             TEXT    NOT NULL,
        dest_dir        TEXT    NOT NULL,
        file_name       TEXT,
        state           TEXT    NOT NULL CHECK (state IN ('queued', 'connecting',
                            'downloading', 'paused', 'completed', 'failed', 'cancelled')),
        total           INTEGER,
        downloaded      INTEGER NOT NULL DEFAULT 0,
        validator_kind  TEXT    CHECK (validator_kind IN ('etag', 'last-modified')),
        validator_value TEXT,
        resumable       INTEGER NOT NULL DEFAULT 0 CHECK (resumable IN (0, 1)),
        error           TEXT,
        PRIMARY KEY (id_secs, id_seq)
    ) STRICT;",
    // 2: ranges of multi-connection downloads
    "CREATE TABLE segments (
        job_secs    INTEGER NOT NULL,
        job_seq     INTEGER NOT NULL,
        range_start INTEGER NOT NULL,
        range_end   INTEGER NOT NULL,
        done        INTEGER NOT NULL,
        PRIMARY KEY (job_secs, job_seq, range_start),
        FOREIGN KEY (job_secs, job_seq) REFERENCES jobs (id_secs, id_seq) ON DELETE CASCADE,
        CHECK (range_start < range_end AND done >= 0 AND done <= range_end - range_start)
    ) STRICT;",
    // 3: priorities, speed limits and settings
    "ALTER TABLE jobs ADD COLUMN priority TEXT NOT NULL DEFAULT 'normal'
        CHECK (priority IN ('low', 'normal', 'high', 'urgent'));
     ALTER TABLE jobs ADD COLUMN speed_limit INTEGER NOT NULL DEFAULT 0
        CHECK (speed_limit >= 0);
     CREATE TABLE settings (
        id             INTEGER PRIMARY KEY CHECK (id = 1),
        global_limit   INTEGER NOT NULL DEFAULT 0 CHECK (global_limit >= 0),
        max_concurrent INTEGER NOT NULL DEFAULT 3 CHECK (max_concurrent >= 1)
     ) STRICT;
     INSERT INTO settings (id) VALUES (1);",
    // 4: categories. Runs with foreign keys off: SQLite only adds a
    // REFERENCES column with a non-NULL default while they are.
    "CREATE TABLE categories (
        id             INTEGER PRIMARY KEY CHECK (id >= 1),
        name           TEXT    NOT NULL COLLATE NOCASE UNIQUE CHECK (name <> ''),
        folder         TEXT,
        speed_limit    INTEGER NOT NULL DEFAULT 0 CHECK (speed_limit >= 0),
        max_concurrent INTEGER NOT NULL DEFAULT 0 CHECK (max_concurrent >= 0)
     ) STRICT;
     CREATE TABLE category_extensions (
        extension   TEXT    PRIMARY KEY CHECK (extension <> ''),
        category_id INTEGER NOT NULL REFERENCES categories (id) ON DELETE CASCADE
     ) STRICT;
     INSERT INTO categories (id, name) VALUES (1, 'General');
     ALTER TABLE jobs ADD COLUMN category_id INTEGER NOT NULL DEFAULT 1
        REFERENCES categories (id);
     ALTER TABLE jobs ADD COLUMN category_chosen INTEGER NOT NULL DEFAULT 0
        CHECK (category_chosen IN (0, 1));",
    // 5: the `retrying` state and the retry policy. SQLite can't change a
    // CHECK constraint, so `jobs` is rebuilt (with foreign keys off, see `prepare`).
    "CREATE TABLE jobs_new (
        id_secs         INTEGER NOT NULL,
        id_seq          INTEGER NOT NULL,
        url             TEXT    NOT NULL,
        dest_dir        TEXT    NOT NULL,
        file_name       TEXT,
        state           TEXT    NOT NULL CHECK (state IN ('queued', 'connecting',
                            'downloading', 'paused', 'completed', 'failed', 'cancelled',
                            'retrying')),
        total           INTEGER,
        downloaded      INTEGER NOT NULL DEFAULT 0,
        validator_kind  TEXT    CHECK (validator_kind IN ('etag', 'last-modified')),
        validator_value TEXT,
        resumable       INTEGER NOT NULL DEFAULT 0 CHECK (resumable IN (0, 1)),
        error           TEXT,
        priority        TEXT    NOT NULL DEFAULT 'normal'
                            CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
        speed_limit     INTEGER NOT NULL DEFAULT 0 CHECK (speed_limit >= 0),
        category_id     INTEGER NOT NULL DEFAULT 1 REFERENCES categories (id),
        category_chosen INTEGER NOT NULL DEFAULT 0 CHECK (category_chosen IN (0, 1)),
        PRIMARY KEY (id_secs, id_seq)
     ) STRICT;
     INSERT INTO jobs_new (id_secs, id_seq, url, dest_dir, file_name, state, total,
                           downloaded, validator_kind, validator_value, resumable, error,
                           priority, speed_limit, category_id, category_chosen)
        SELECT id_secs, id_seq, url, dest_dir, file_name, state, total,
               downloaded, validator_kind, validator_value, resumable, error,
               priority, speed_limit, category_id, category_chosen
        FROM jobs;
     DROP TABLE jobs;
     ALTER TABLE jobs_new RENAME TO jobs;
     ALTER TABLE settings ADD COLUMN max_retries INTEGER NOT NULL DEFAULT 10
        CHECK (max_retries >= 0);
     ALTER TABLE settings ADD COLUMN retries_before_yield INTEGER NOT NULL DEFAULT 3
        CHECK (retries_before_yield >= 0);
     ALTER TABLE settings ADD COLUMN retry_wait_ms INTEGER NOT NULL DEFAULT 2000
        CHECK (retry_wait_ms >= 0);",
];

/// Everything saved about one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JobRecord {
    pub id: JobId,
    pub state: JobState,
    pub download: Download,
    pub error: Option<String>,
    pub priority: Priority,
    pub speed_limit: u64,
    pub category: CategoryId,
    /// The user picked the category, rather than it being matched by extension.
    pub category_chosen: bool,
}

/// Everything the database holds, as read at start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Loaded {
    /// Oldest first.
    pub jobs: Vec<JobRecord>,
    pub settings: Settings,
    /// By ID; General is always there.
    pub categories: Vec<Category>,
}

/// Settings that apply to the whole manager. Stored as the single row of the
/// `settings` table; the defaults here match the table's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Bytes per second for all downloads together; 0 = unlimited.
    pub global_limit: u64,
    /// How many jobs may download at once. At least 1.
    pub max_concurrent: usize,
    pub retry: RetryPolicy,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            global_limit: 0,
            max_concurrent: 3,
            retry: RetryPolicy::default(),
        }
    }
}

/// What happens when a download fails with an error that may go away (a
/// dropped connection, a busy server). See docs/decisions/0013.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Retries after the attempts since bytes last arrived; then the job
    /// fails. 0 = never retry.
    pub max_retries: u32,
    /// Retries a job makes in its download slot before giving way to a job
    /// that is waiting for one.
    pub retries_before_yield: u32,
    /// Wait before the first retry. Each next one waits twice as long, up to
    /// a minute (or this wait, if longer); a server's `Retry-After` can ask for more.
    pub first_wait: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 10,
            retries_before_yield: 3,
            first_wait: Duration::from_secs(2),
        }
    }
}

/// Handle to the database thread. The thread stops when this is dropped.
pub struct Store {
    ops: std_mpsc::Sender<Op>,
}

enum Op {
    Load(oneshot::Sender<Result<Loaded, Error>>),
    Save(Box<JobRecord>),
    SaveSettings(Settings),
    SaveCategory(Category),
    RemoveCategory(CategoryId),
    Flush(oneshot::Sender<()>),
}

impl Store {
    /// Opens (or creates) the database file and brings its schema up to date.
    pub async fn open(path: impl Into<PathBuf>) -> Result<Store, Error> {
        let path = path.into();
        Self::spawn(move || Connection::open(path)).await
    }

    /// A database that only lives in memory, for tests and experiments.
    pub async fn open_in_memory() -> Result<Store, Error> {
        Self::spawn(Connection::open_in_memory).await
    }

    async fn spawn(
        open: impl FnOnce() -> rusqlite::Result<Connection> + Send + 'static,
    ) -> Result<Store, Error> {
        let (ops, requests) = std_mpsc::channel();
        let (ready, opened) = oneshot::channel();
        thread::Builder::new()
            .name("ottie-store".to_owned())
            .spawn(move || {
                let conn = open().map_err(Error::from).and_then(|mut conn| {
                    prepare(&mut conn)?;
                    Ok(conn)
                });
                match conn {
                    Ok(conn) => {
                        let _ = ready.send(Ok(()));
                        serve(conn, requests);
                    }
                    Err(err) => {
                        let _ = ready.send(Err(err));
                    }
                }
            })?;
        opened.await.map_err(|_| Error::StoreStopped)??;
        Ok(Store { ops })
    }

    pub(crate) async fn load(&self) -> Result<Loaded, Error> {
        let (reply, answer) = oneshot::channel();
        self.ops
            .send(Op::Load(reply))
            .map_err(|_| Error::StoreStopped)?;
        answer.await.map_err(|_| Error::StoreStopped)?
    }

    /// Queues an insert-or-update. Writes happen in order. Failures are logged
    /// rather than returned: a failed save must not stop a download.
    pub(crate) fn save(&self, record: JobRecord) {
        if self.ops.send(Op::Save(Box::new(record))).is_err() {
            tracing::error!("database thread has stopped; job not saved");
        }
    }

    /// Queued like `save`.
    pub(crate) fn save_settings(&self, settings: Settings) {
        if self.ops.send(Op::SaveSettings(settings)).is_err() {
            tracing::error!("database thread has stopped; settings not saved");
        }
    }

    /// Queued like `save`. Replaces the category's extensions too.
    pub(crate) fn save_category(&self, category: Category) {
        if self.ops.send(Op::SaveCategory(category)).is_err() {
            tracing::error!("database thread has stopped; category not saved");
        }
    }

    /// Queued like `save`. Its jobs must have been saved in another category first.
    pub(crate) fn remove_category(&self, id: CategoryId) {
        if self.ops.send(Op::RemoveCategory(id)).is_err() {
            tracing::error!("database thread has stopped; category not removed");
        }
    }

    /// Waits until every save queued before this call has been written.
    pub(crate) async fn flush(&self) {
        let (reply, done) = oneshot::channel();
        if self.ops.send(Op::Flush(reply)).is_ok() {
            let _ = done.await;
        }
    }
}

/// Runs on the database thread until the `Store` is dropped.
fn serve(mut conn: Connection, requests: std_mpsc::Receiver<Op>) {
    while let Ok(op) = requests.recv() {
        match op {
            Op::Load(reply) => {
                let _ = reply.send(load_all(&conn));
            }
            Op::Save(record) => {
                if let Err(err) = save(&mut conn, &record) {
                    tracing::error!(id = %record.id, error = %err, "could not save job");
                }
            }
            Op::SaveSettings(settings) => {
                if let Err(err) = save_settings(&conn, &settings) {
                    tracing::error!(error = %err, "could not save settings");
                }
            }
            Op::SaveCategory(category) => {
                if let Err(err) = save_category(&mut conn, &category) {
                    tracing::error!(id = %category.id, error = %err, "could not save category");
                }
            }
            Op::RemoveCategory(id) => {
                if let Err(err) = conn.execute("DELETE FROM categories WHERE id = ?1", [id.0]) {
                    tracing::error!(%id, error = %err, "could not remove category");
                }
            }
            Op::Flush(reply) => {
                let _ = reply.send(());
            }
        }
    }
}

fn prepare(conn: &mut Connection) -> Result<(), Error> {
    // WAL: a crash can't corrupt the file, and writes don't block reads.
    // NORMAL: a power cut may lose the last few saves, which only means
    // re-downloading a little; it never makes us trust bytes we don't have.
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // Some schema changes need foreign keys off, so they're only turned on
    // after migrating; `migrate` checks them itself.
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    migrate(conn)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

fn migrate(conn: &mut Connection) -> Result<(), Error> {
    let found: usize = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if found > MIGRATIONS.len() {
        return Err(Error::DatabaseTooNew {
            found,
            supported: MIGRATIONS.len(),
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(found) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        if tx.prepare("PRAGMA foreign_key_check")?.exists([])? {
            // Returning without committing undoes this migration.
            return Err(Error::MigrationBroken(index + 1));
        }
        tx.pragma_update(None, "user_version", index + 1)?;
        tx.commit()?;
    }
    Ok(())
}

/// Upserts the job and replaces its segments, in one transaction so a crash
/// can't leave the two out of step.
fn save(conn: &mut Connection, record: &JobRecord) -> Result<(), Error> {
    let dl = &record.download;
    let dest_dir = dl
        .dest_dir
        .to_str()
        .ok_or_else(|| Error::NonUtf8Path(dl.dest_dir.clone()))?;
    let (validator_kind, validator_value) = match &dl.validator {
        Some(Validator::ETag(v)) => (Some("etag"), Some(v.as_str())),
        Some(Validator::LastModified(v)) => (Some("last-modified"), Some(v.as_str())),
        None => (None, None),
    };
    let (secs, seq) = (record.id.secs(), record.id.seq());
    let tx = conn.transaction()?;
    tx.prepare_cached(
        "INSERT INTO jobs (id_secs, id_seq, url, dest_dir, file_name, state, total,
                           downloaded, validator_kind, validator_value, resumable, error,
                           priority, speed_limit, category_id, category_chosen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT (id_secs, id_seq) DO UPDATE SET
             url = excluded.url, dest_dir = excluded.dest_dir,
             file_name = excluded.file_name, state = excluded.state,
             total = excluded.total, downloaded = excluded.downloaded,
             validator_kind = excluded.validator_kind,
             validator_value = excluded.validator_value,
             resumable = excluded.resumable, error = excluded.error,
             priority = excluded.priority, speed_limit = excluded.speed_limit,
             category_id = excluded.category_id, category_chosen = excluded.category_chosen",
    )?
    .execute(params![
        secs,
        seq,
        dl.url.as_str(),
        dest_dir,
        dl.file_name,
        state_name(record.state),
        dl.total,
        dl.downloaded,
        validator_kind,
        validator_value,
        dl.resumable,
        record.error,
        priority_name(record.priority),
        record.speed_limit,
        record.category.0,
        record.category_chosen,
    ])?;
    tx.prepare_cached("DELETE FROM segments WHERE job_secs = ?1 AND job_seq = ?2")?
        .execute(params![secs, seq])?;
    {
        let mut insert = tx.prepare_cached(
            "INSERT INTO segments (job_secs, job_seq, range_start, range_end, done)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for s in &dl.segments {
            insert.execute(params![secs, seq, s.start, s.end, s.done])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn load_all(conn: &Connection) -> Result<Loaded, Error> {
    Ok(Loaded {
        jobs: load_jobs(conn)?,
        settings: load_settings(conn)?,
        categories: load_categories(conn)?,
    })
}

fn load_jobs(conn: &Connection) -> Result<Vec<JobRecord>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id_secs, id_seq, url, dest_dir, file_name, state, total, downloaded,
                validator_kind, validator_value, resumable, error, priority, speed_limit,
                category_id, category_chosen
         FROM jobs ORDER BY id_secs, id_seq",
    )?;
    let mut rows = stmt.query([])?;
    let mut records = Vec::new();
    while let Some(row) = rows.next()? {
        // A row we can't understand is skipped (and logged) rather than
        // stopping every other download from loading.
        match record_from_row(row)? {
            Ok(record) => records.push(record),
            Err(problem) => tracing::error!(%problem, "skipping unreadable job in database"),
        }
    }

    let mut by_job: BTreeMap<JobId, Vec<Segment>> = BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT job_secs, job_seq, range_start, range_end, done
         FROM segments ORDER BY job_secs, job_seq, range_start",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id = JobId::from_parts(row.get(0)?, row.get(1)?);
        by_job.entry(id).or_default().push(Segment {
            start: row.get(2)?,
            end: row.get(3)?,
            done: row.get(4)?,
        });
    }
    for record in &mut records {
        record.download.segments = by_job.remove(&record.id).unwrap_or_default();
    }
    Ok(records)
}

/// The outer `Result` is a database error, the inner one a row with bad data.
fn record_from_row(row: &Row) -> rusqlite::Result<Result<JobRecord, String>> {
    let id = JobId::from_parts(row.get(0)?, row.get(1)?);
    let url: String = row.get(2)?;
    let state: String = row.get(5)?;
    let validator_kind: Option<String> = row.get(8)?;
    let validator_value: Option<String> = row.get(9)?;

    let Ok(url) = Url::parse(&url) else {
        return Ok(Err(format!("job {id}: invalid URL")));
    };
    let Some(state) = parse_state(&state) else {
        return Ok(Err(format!("job {id}: unknown state `{state}`")));
    };
    let priority: String = row.get(12)?;
    let Some(priority) = parse_priority(&priority) else {
        return Ok(Err(format!("job {id}: unknown priority `{priority}`")));
    };
    let validator = match (validator_kind.as_deref(), validator_value) {
        (Some("etag"), Some(v)) => Some(Validator::ETag(v)),
        (Some("last-modified"), Some(v)) => Some(Validator::LastModified(v)),
        _ => None,
    };
    let download = Download {
        url,
        dest_dir: PathBuf::from(row.get::<_, String>(3)?),
        file_name: row.get(4)?,
        total: row.get(6)?,
        downloaded: row.get(7)?,
        validator,
        resumable: row.get(10)?,
        segments: Vec::new(), // filled in by `load`
        placement: None,
    };
    Ok(Ok(JobRecord {
        id,
        state,
        download,
        error: row.get(11)?,
        priority,
        speed_limit: row.get(13)?,
        category: CategoryId(row.get(14)?),
        category_chosen: row.get(15)?,
    }))
}

fn load_settings(conn: &Connection) -> Result<Settings, Error> {
    Ok(conn.query_row(
        "SELECT global_limit, max_concurrent, max_retries, retries_before_yield, retry_wait_ms
         FROM settings WHERE id = 1",
        [],
        |row| {
            Ok(Settings {
                global_limit: row.get(0)?,
                max_concurrent: row.get(1)?,
                retry: RetryPolicy {
                    max_retries: row.get(2)?,
                    retries_before_yield: row.get(3)?,
                    first_wait: Duration::from_millis(row.get(4)?),
                },
            })
        },
    )?)
}

fn save_settings(conn: &Connection, settings: &Settings) -> Result<(), Error> {
    let retry = &settings.retry;
    let wait_ms = u64::try_from(retry.first_wait.as_millis()).unwrap_or(u64::MAX);
    conn.execute(
        "UPDATE settings SET global_limit = ?1, max_concurrent = ?2, max_retries = ?3,
                retries_before_yield = ?4, retry_wait_ms = ?5
         WHERE id = 1",
        params![
            settings.global_limit,
            settings.max_concurrent,
            retry.max_retries,
            retry.retries_before_yield,
            wait_ms,
        ],
    )?;
    Ok(())
}

fn load_categories(conn: &Connection) -> Result<Vec<Category>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id, name, folder, speed_limit, max_concurrent FROM categories ORDER BY id",
    )?;
    let mut categories: BTreeMap<CategoryId, Category> = stmt
        .query_map([], |row| {
            let id = CategoryId(row.get(0)?);
            let settings = CategorySettings {
                name: row.get(1)?,
                folder: row.get::<_, Option<String>>(2)?.map(PathBuf::from),
                extensions: Vec::new(), // filled in below
                speed_limit: row.get(3)?,
                max_concurrent: row.get(4)?,
            };
            Ok((id, Category { id, settings }))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt =
        conn.prepare("SELECT category_id, extension FROM category_extensions ORDER BY extension")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if let Some(category) = categories.get_mut(&CategoryId(row.get(0)?)) {
            category.settings.extensions.push(row.get(1)?);
        }
    }
    Ok(categories.into_values().collect())
}

/// Upserts the category and replaces its extensions, in one transaction.
fn save_category(conn: &mut Connection, category: &Category) -> Result<(), Error> {
    let settings = &category.settings;
    let folder = match &settings.folder {
        Some(folder) => Some(
            folder
                .to_str()
                .ok_or_else(|| Error::NonUtf8Path(folder.clone()))?,
        ),
        None => None,
    };
    let id = category.id.0;
    let tx = conn.transaction()?;
    tx.prepare_cached(
        "INSERT INTO categories (id, name, folder, speed_limit, max_concurrent)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (id) DO UPDATE SET
             name = excluded.name, folder = excluded.folder,
             speed_limit = excluded.speed_limit, max_concurrent = excluded.max_concurrent",
    )?
    .execute(params![
        id,
        settings.name,
        folder,
        settings.speed_limit,
        settings.max_concurrent,
    ])?;
    tx.prepare_cached("DELETE FROM category_extensions WHERE category_id = ?1")?
        .execute([id])?;
    {
        let mut insert = tx.prepare_cached(
            "INSERT INTO category_extensions (extension, category_id) VALUES (?1, ?2)",
        )?;
        for ext in &settings.extensions {
            insert.execute(params![ext, id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn priority_name(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => "low",
        Priority::Normal => "normal",
        Priority::High => "high",
        Priority::Urgent => "urgent",
    }
}

fn parse_priority(name: &str) -> Option<Priority> {
    Some(match name {
        "low" => Priority::Low,
        "normal" => Priority::Normal,
        "high" => Priority::High,
        "urgent" => Priority::Urgent,
        _ => return None,
    })
}

/// How states are written in the database. Independent of the Rust names, so
/// renaming an enum variant can't silently change stored data.
fn state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Connecting => "connecting",
        JobState::Downloading => "downloading",
        JobState::Paused => "paused",
        JobState::Completed => "completed",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::Retrying => "retrying",
    }
}

fn parse_state(name: &str) -> Option<JobState> {
    Some(match name {
        "queued" => JobState::Queued,
        "connecting" => JobState::Connecting,
        "downloading" => JobState::Downloading,
        "paused" => JobState::Paused,
        "completed" => JobState::Completed,
        "failed" => JobState::Failed,
        "cancelled" => JobState::Cancelled,
        "retrying" => JobState::Retrying,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(seq: u32) -> JobRecord {
        let url = Url::parse("https://example.com/file.zip?x=1").unwrap();
        let mut download = Download::new(url, "/downloads").unwrap();
        download.file_name = Some("file.zip".to_owned());
        download.total = Some(u64::from(u32::MAX) * 4);
        download.downloaded = 1234;
        download.validator = Some(Validator::ETag("\"abc\"".to_owned()));
        download.resumable = true;
        JobRecord {
            id: JobId::from_parts(1_760_279_405, seq),
            state: JobState::Paused,
            download,
            error: None,
            priority: Priority::High,
            speed_limit: 512 * 1024,
            category: CategoryId::GENERAL,
            category_chosen: false,
        }
    }

    #[tokio::test]
    async fn saves_and_loads_every_field() {
        let store = Store::open_in_memory().await.unwrap();
        let mut failed = record(2);
        failed.state = JobState::Failed;
        failed.error = Some("server answered 404".to_owned());
        failed.download.validator = Some(Validator::LastModified("yesterday".to_owned()));
        let mut fresh = record(3);
        fresh.download = Download::new(fresh.download.url.clone(), "/x").unwrap();

        for r in [record(1), failed.clone(), fresh.clone()] {
            store.save(r);
        }
        assert_eq!(store.load().await.unwrap().jobs, [record(1), failed, fresh]);
    }

    #[tokio::test]
    async fn saving_again_updates_the_row() {
        let store = Store::open_in_memory().await.unwrap();
        store.save(record(1));
        let mut done = record(1);
        done.state = JobState::Completed;
        done.download.downloaded = done.download.total.unwrap();
        store.save(done.clone());
        assert_eq!(store.load().await.unwrap().jobs, [done]);
    }

    #[tokio::test]
    async fn segments_are_saved_and_replaced() {
        let store = Store::open_in_memory().await.unwrap();
        let mut job = record(1);
        let seg = |start, end, done| Segment { start, end, done };
        job.download.segments = vec![seg(0, 50, 50), seg(50, 100, 10)];
        store.save(job.clone());
        assert_eq!(store.load().await.unwrap().jobs, [job.clone()]);

        // After a split there are more ranges; after completion none.
        job.download.segments = vec![seg(0, 50, 50), seg(50, 75, 25), seg(75, 100, 3)];
        store.save(job.clone());
        assert_eq!(store.load().await.unwrap().jobs, [job.clone()]);
        job.download.segments.clear();
        store.save(job.clone());
        assert_eq!(store.load().await.unwrap().jobs, [job]);
    }

    #[tokio::test]
    async fn settings_start_at_their_defaults_and_can_change() {
        let store = Store::open_in_memory().await.unwrap();
        assert_eq!(store.load().await.unwrap().settings, Settings::default());
        let changed = Settings {
            global_limit: 1_000_000,
            max_concurrent: 5,
            retry: RetryPolicy {
                max_retries: 4,
                retries_before_yield: 0,
                first_wait: Duration::from_millis(250),
            },
        };
        store.save_settings(changed.clone());
        assert_eq!(store.load().await.unwrap().settings, changed);
    }

    fn videos() -> Category {
        Category {
            id: CategoryId(2),
            settings: CategorySettings {
                folder: Some(PathBuf::from("/downloads/videos")),
                extensions: vec!["mkv".into(), "mp4".into()],
                speed_limit: 1_000_000,
                max_concurrent: 2,
                ..CategorySettings::named("Videos")
            },
        }
    }

    #[tokio::test]
    async fn general_exists_from_the_start() {
        let store = Store::open_in_memory().await.unwrap();
        let general = Category {
            id: CategoryId::GENERAL,
            settings: CategorySettings::named("General"),
        };
        assert_eq!(store.load().await.unwrap().categories, [general]);
    }

    #[tokio::test]
    async fn categories_are_saved_replaced_and_removed() {
        let store = Store::open_in_memory().await.unwrap();
        let mut videos = videos();
        store.save_category(videos.clone());
        assert_eq!(store.load().await.unwrap().categories[1], videos);

        videos.settings.extensions = vec!["webm".into()];
        videos.settings.folder = None;
        store.save_category(videos.clone());
        assert_eq!(store.load().await.unwrap().categories[1], videos);

        store.remove_category(videos.id);
        assert_eq!(store.load().await.unwrap().categories.len(), 1);
        // Its extension went with it, so another category can take it.
        let mut films = videos;
        films.settings.name = "Films".into();
        store.save_category(films.clone());
        assert_eq!(store.load().await.unwrap().categories[1], films);
    }

    #[tokio::test]
    async fn jobs_keep_their_category() {
        let store = Store::open_in_memory().await.unwrap();
        store.save_category(videos());
        let mut job = record(1);
        job.category = videos().id;
        job.category_chosen = true;
        store.save(job.clone());
        assert_eq!(store.load().await.unwrap().jobs, [job]);
    }

    #[test]
    fn jobs_from_before_categories_land_in_general() {
        let mut conn = Connection::open_in_memory().unwrap();
        for (i, sql) in MIGRATIONS[..3].iter().enumerate() {
            conn.execute_batch(sql).unwrap();
            conn.pragma_update(None, "user_version", i + 1).unwrap();
        }
        conn.execute(
            "INSERT INTO jobs (id_secs, id_seq, url, dest_dir, state)
             VALUES (1, 1, 'https://example.com/a.mkv', '/d', 'paused')",
            [],
        )
        .unwrap();
        prepare(&mut conn).unwrap();
        let jobs = load_jobs(&conn).unwrap();
        assert_eq!(jobs[0].category, CategoryId::GENERAL);
        assert!(!jobs[0].category_chosen);
    }

    #[test]
    fn rebuilding_jobs_keeps_every_row_and_segment() {
        let mut conn = Connection::open_in_memory().unwrap();
        for (i, sql) in MIGRATIONS[..4].iter().enumerate() {
            conn.execute_batch(sql).unwrap();
            conn.pragma_update(None, "user_version", i + 1).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO jobs (id_secs, id_seq, url, dest_dir, file_name, state, total,
                               downloaded, priority, speed_limit)
             VALUES (1, 1, 'https://example.com/a.bin', '/d', 'a.bin', 'paused', 100,
                     40, 'high', 7);
             INSERT INTO segments VALUES (1, 1, 0, 50, 30), (1, 1, 50, 100, 10);",
        )
        .unwrap();
        prepare(&mut conn).unwrap();

        let loaded = load_all(&conn).unwrap();
        let job = &loaded.jobs[0];
        assert_eq!((job.priority, job.speed_limit), (Priority::High, 7));
        assert_eq!(job.download.segments.len(), 2);
        assert_eq!(loaded.settings.retry, RetryPolicy::default());
        // The segments still belong to the rebuilt table.
        conn.execute("DELETE FROM jobs", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM segments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0, "deleting a job deletes its segments");
    }

    #[test]
    fn priority_names_round_trip() {
        use Priority::*;
        for priority in [Low, Normal, High, Urgent] {
            assert_eq!(parse_priority(priority_name(priority)), Some(priority));
        }
    }

    #[test]
    fn state_names_round_trip() {
        use JobState::*;
        for state in [
            Queued,
            Connecting,
            Downloading,
            Paused,
            Completed,
            Failed,
            Cancelled,
            Retrying,
        ] {
            assert_eq!(parse_state(state_name(state)), Some(state));
        }
    }

    #[test]
    fn migrations_set_the_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        prepare(&mut conn).unwrap();
        let version: usize = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len());
        // Running again is a no-op.
        prepare(&mut conn).unwrap();
    }
}
