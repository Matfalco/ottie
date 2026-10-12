//! SQLite persistence. SQLite calls block, so the connection lives on its own
//! thread and the manager talks to it through a channel. See docs/decisions/0008.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::thread;

use reqwest::Url;
use rusqlite::{Connection, Row, params};
use tokio::sync::oneshot;

use crate::download::{Download, Segment, Validator};
use crate::error::Error;
use crate::id::JobId;
use crate::job::JobState;

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
];

/// Everything saved about one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JobRecord {
    pub id: JobId,
    pub state: JobState,
    pub download: Download,
    pub error: Option<String>,
}

/// Handle to the database thread. The thread stops when this is dropped.
pub struct Store {
    ops: std_mpsc::Sender<Op>,
}

enum Op {
    Load(oneshot::Sender<Result<Vec<JobRecord>, Error>>),
    Save(Box<JobRecord>),
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

    pub(crate) async fn load(&self) -> Result<Vec<JobRecord>, Error> {
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
                let _ = reply.send(load(&conn));
            }
            Op::Save(record) => {
                if let Err(err) = save(&mut conn, &record) {
                    tracing::error!(id = %record.id, error = %err, "could not save job");
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
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(conn)
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
                           downloaded, validator_kind, validator_value, resumable, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT (id_secs, id_seq) DO UPDATE SET
             url = excluded.url, dest_dir = excluded.dest_dir,
             file_name = excluded.file_name, state = excluded.state,
             total = excluded.total, downloaded = excluded.downloaded,
             validator_kind = excluded.validator_kind,
             validator_value = excluded.validator_value,
             resumable = excluded.resumable, error = excluded.error",
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

fn load(conn: &Connection) -> Result<Vec<JobRecord>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id_secs, id_seq, url, dest_dir, file_name, state, total, downloaded,
                validator_kind, validator_value, resumable, error
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
    };
    Ok(Ok(JobRecord {
        id,
        state,
        download,
        error: row.get(11)?,
    }))
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
        assert_eq!(store.load().await.unwrap(), [record(1), failed, fresh]);
    }

    #[tokio::test]
    async fn saving_again_updates_the_row() {
        let store = Store::open_in_memory().await.unwrap();
        store.save(record(1));
        let mut done = record(1);
        done.state = JobState::Completed;
        done.download.downloaded = done.download.total.unwrap();
        store.save(done.clone());
        assert_eq!(store.load().await.unwrap(), [done]);
    }

    #[tokio::test]
    async fn segments_are_saved_and_replaced() {
        let store = Store::open_in_memory().await.unwrap();
        let mut job = record(1);
        let seg = |start, end, done| Segment { start, end, done };
        job.download.segments = vec![seg(0, 50, 50), seg(50, 100, 10)];
        store.save(job.clone());
        assert_eq!(store.load().await.unwrap(), [job.clone()]);

        // After a split there are more ranges; after completion none.
        job.download.segments = vec![seg(0, 50, 50), seg(50, 75, 25), seg(75, 100, 3)];
        store.save(job.clone());
        assert_eq!(store.load().await.unwrap(), [job.clone()]);
        job.download.segments.clear();
        store.save(job.clone());
        assert_eq!(store.load().await.unwrap(), [job]);
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
