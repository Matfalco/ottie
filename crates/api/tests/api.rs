#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The API end to end: a real server on 127.0.0.1, called the way clients
//! will, next to a small file server.

use std::path::Path;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::routing::get;
use futures_util::StreamExt;
use ottie_api::{
    AddJob, Added, AddedCategory, ApiError, Appearance, AppearanceMode, BuiltIn, Category,
    CategoryBody, EVENTS_PROTOCOL, Event, Job, JobState, Matched, Palette, Problem, Reason,
    Settings, SettingsChange, TOKEN_PROTOCOL_PREFIX, Token, UiSource,
};
use ottie_core::{HttpBackend, Manager, Store};
use reqwest::{RequestBuilder, StatusCode};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_util::sync::CancellationToken;

const TOKEN: &str = "0123456789abcdef";
const SIZE: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

struct Daemon {
    /// `127.0.0.1:port` of the API.
    addr: String,
    /// Base URL of the file server.
    files: String,
    client: reqwest::Client,
    dir: TempDir,
    _shutdown: CancellationToken,
}

/// `/file.bin` arrives at once; `/slow.bin` takes about 4 s.
async fn start() -> Daemon {
    start_with(UiSource::Embedded).await
}

async fn start_with(ui: UiSource) -> Daemon {
    let files = Router::new()
        .route("/file.bin", get(|| async { vec![7u8; SIZE] }))
        .route("/slow.bin", get(|| async { slow_body() }));
    let files = serve(files).await;

    let store = Store::open_in_memory().await.unwrap();
    let manager = Manager::start(HttpBackend::new().unwrap(), store)
        .await
        .unwrap();
    let shutdown = CancellationToken::new();
    let api = ottie_api::router(manager, Token::new(TOKEN), shutdown.clone(), ui);
    Daemon {
        addr: serve(api).await,
        files: format!("http://{files}"),
        client: reqwest::Client::new(),
        dir: tempfile::tempdir().unwrap(),
        _shutdown: shutdown,
    }
}

async fn serve(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

fn slow_body() -> Body {
    let chunks = futures_util::stream::unfold(0, |sent| async move {
        if sent == 200 {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        Some((
            Ok::<_, std::io::Error>(Bytes::from(vec![1u8; 1024])),
            sent + 1,
        ))
    });
    Body::from_stream(chunks)
}

impl Daemon {
    fn url(&self, path: &str) -> String {
        format!("http://{}/api/v1{path}", self.addr)
    }

    fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(self.url(path)).bearer_auth(TOKEN)
    }

    fn post(&self, path: &str) -> RequestBuilder {
        self.client.post(self.url(path)).bearer_auth(TOKEN)
    }

    fn new_job(&self, file: &str) -> AddJob {
        AddJob {
            url: format!("{}/{file}", self.files),
            dest_dir: Some(self.dir.path().to_string_lossy().into_owned()),
            category: None,
        }
    }

    async fn add(&self, file: &str) -> String {
        let response = self.post("/jobs").json(&self.new_job(file)).send().await;
        let response = response.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        response.json::<Added>().await.unwrap().id
    }

    async fn job(&self, id: &str) -> Job {
        let jobs: Vec<Job> = self
            .get("/jobs")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        jobs.into_iter().find(|job| job.id == id).unwrap()
    }

    async fn wait_until(&self, id: &str, state: JobState) {
        tokio::time::timeout(TIMEOUT, async {
            while self.job(id).await.state != state {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("job {id} never became {state:?}"));
    }

    /// Opens the events WebSocket, offering `protocols`.
    async fn events(
        &self,
        protocols: &str,
    ) -> Result<
        (
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            tungstenite::handshake::client::Response,
        ),
        tungstenite::Error,
    > {
        let mut request = format!("ws://{}/api/v1/events", self.addr)
            .into_client_request()
            .unwrap();
        if !protocols.is_empty() {
            let value = protocols.parse().unwrap();
            request
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", value);
        }
        tokio_tungstenite::connect_async(request).await
    }
}

async fn error_of(response: reqwest::Response) -> (StatusCode, ApiError) {
    let status = response.status();
    (status, response.json::<ApiError>().await.unwrap())
}

#[tokio::test]
async fn every_request_needs_the_token() {
    let d = start().await;
    let anonymous = d.client.get(d.url("/jobs")).send().await.unwrap();
    let (status, refused) = error_of(anonymous).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(refused.reason, Reason::Unauthorized);
    assert!(refused.error.contains("token"));

    let wrong = d.client.get(d.url("/jobs")).bearer_auth("guess");
    assert_eq!(
        wrong.send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    // Unknown paths don't reveal themselves without the token either.
    let unknown = d.client.get(d.url("/nope")).send().await.unwrap();
    assert_eq!(unknown.status(), StatusCode::UNAUTHORIZED);
    let unknown = d.get("/nope").send().await.unwrap();
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);

    let jobs: Vec<Job> = d.get("/jobs").send().await.unwrap().json().await.unwrap();
    assert!(jobs.is_empty());
}

#[tokio::test]
async fn the_event_stream_needs_the_token() {
    let d = start().await;
    for offered in [
        "",
        EVENTS_PROTOCOL,
        &format!("{EVENTS_PROTOCOL}, {TOKEN_PROTOCOL_PREFIX}guess"),
    ] {
        match d.events(offered).await {
            Err(tungstenite::Error::Http(response)) => {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            }
            other => panic!("expected a refusal for {offered:?}, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_download_from_start_to_finish() {
    let d = start().await;
    let offered = format!("{EVENTS_PROTOCOL}, {TOKEN_PROTOCOL_PREFIX}{TOKEN}");
    let (mut events, response) = d.events(&offered).await.unwrap();
    // The server picks the events protocol, never echoing the token.
    assert_eq!(
        response.headers()["sec-websocket-protocol"],
        EVENTS_PROTOCOL
    );

    let id = d.add("file.bin").await;
    let mut states = Vec::new();
    tokio::time::timeout(TIMEOUT, async {
        while states.last() != Some(&JobState::Completed) {
            let Some(Ok(Message::Text(text))) = events.next().await else {
                continue;
            };
            if let Event::Changed { job } = serde_json::from_str(&text).unwrap() {
                assert_eq!(job.id, id);
                if states.last() != Some(&job.state) {
                    states.push(job.state);
                }
            }
        }
    })
    .await
    .expect("never completed");
    assert_eq!(states.first(), Some(&JobState::Queued));

    let job = d.job(&id).await;
    assert_eq!(job.file_name.as_deref(), Some("file.bin"));
    assert_eq!(
        (job.downloaded, job.total),
        (SIZE as u64, Some(SIZE as u64))
    );
    let file = Path::new(&job.dest_dir).join("file.bin");
    assert_eq!(std::fs::read(file).unwrap(), vec![7u8; SIZE]);
}

#[tokio::test]
async fn pause_resume_and_cancel() {
    let d = start().await;
    let id = d.add("slow.bin").await;
    d.wait_until(&id, JobState::Downloading).await;

    let paused = d.post(&format!("/jobs/{id}/pause")).send().await.unwrap();
    assert_eq!(paused.status(), StatusCode::NO_CONTENT);
    assert_eq!(d.job(&id).await.state, JobState::Paused);

    let resumed = d.post(&format!("/jobs/{id}/resume")).send().await.unwrap();
    assert_eq!(resumed.status(), StatusCode::NO_CONTENT);
    d.wait_until(&id, JobState::Downloading).await;

    let cancelled = d.post(&format!("/jobs/{id}/cancel")).send().await.unwrap();
    assert_eq!(cancelled.status(), StatusCode::NO_CONTENT);
    assert_eq!(d.job(&id).await.state, JobState::Cancelled);
}

#[tokio::test]
async fn remove_takes_a_job_off_the_list() {
    let d = start().await;
    let offered = format!("{EVENTS_PROTOCOL}, {TOKEN_PROTOCOL_PREFIX}{TOKEN}");
    let (mut events, _) = d.events(&offered).await.unwrap();
    let id = d.add("slow.bin").await;
    d.wait_until(&id, JobState::Downloading).await;

    let removed = d
        .client
        .delete(d.url(&format!("/jobs/{id}")))
        .bearer_auth(TOKEN);
    assert_eq!(
        removed.send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    let jobs: Vec<Job> = d.get("/jobs").send().await.unwrap().json().await.unwrap();
    assert!(jobs.is_empty());
    tokio::time::timeout(TIMEOUT, async {
        loop {
            let Some(Ok(Message::Text(text))) = events.next().await else {
                continue;
            };
            if let Event::Removed { id: gone } = serde_json::from_str(&text).unwrap() {
                assert_eq!(gone, id);
                return;
            }
        }
    })
    .await
    .expect("no removed event");

    let again = d
        .client
        .delete(d.url(&format!("/jobs/{id}")))
        .bearer_auth(TOKEN);
    let (status, refused) = error_of(again.send().await.unwrap()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(refused.reason, Reason::JobNotFound { id });
}

#[tokio::test]
async fn bad_requests_say_what_is_wrong() {
    let d = start().await;
    let add = |job: &AddJob| d.post("/jobs").json(job).send();
    let refused = async |request: RequestBuilder| error_of(request.send().await.unwrap()).await;

    let mut job = d.new_job("file.bin");
    job.url = "not a url".to_owned();
    let (status, body) = error_of(add(&job).await.unwrap()).await;
    assert_eq!(
        (status, body.reason),
        (StatusCode::BAD_REQUEST, Reason::InvalidUrl)
    );
    assert!(body.error.contains("not a URL"), "{}", body.error);

    job.url = "ftp://example.com/file".to_owned();
    let (status, body) = error_of(add(&job).await.unwrap()).await;
    let scheme = Reason::UnsupportedScheme {
        scheme: "ftp".into(),
    };
    assert_eq!((status, body.reason), (StatusCode::BAD_REQUEST, scheme));

    let mut job = d.new_job("file.bin");
    job.dest_dir = Some("relative/folder".to_owned());
    let (status, body) = error_of(add(&job).await.unwrap()).await;
    assert_eq!(
        (status, body.reason),
        (StatusCode::BAD_REQUEST, Reason::RelativeFolder)
    );

    // General has no folder yet, so a folder is required.
    job.dest_dir = None;
    let (status, body) = error_of(add(&job).await.unwrap()).await;
    let no_folder = Reason::NoFolder {
        category: "General".into(),
    };
    assert_eq!((status, body.reason), (StatusCode::BAD_REQUEST, no_folder));

    let broken = d.post("/jobs").header("content-type", "application/json");
    let (status, body) = refused(broken.body("{")).await;
    assert_eq!(
        (status, body.reason),
        (StatusCode::BAD_REQUEST, Reason::InvalidBody)
    );
    let missing_url = d
        .post("/jobs")
        .json(&serde_json::json!({ "destDir": "/x" }));
    let (status, body) = refused(missing_url).await;
    let expected = (StatusCode::UNPROCESSABLE_ENTITY, Reason::InvalidBody);
    assert_eq!((status, body.reason), expected);

    let (status, body) = refused(d.post("/jobs/nonsense/pause")).await;
    let invalid = Reason::InvalidJobId {
        id: "nonsense".into(),
    };
    assert_eq!((status, body.reason), (StatusCode::BAD_REQUEST, invalid));
    let unknown_id = "2026.01.01-00.00.00-1";
    let (status, body) = refused(d.post(&format!("/jobs/{unknown_id}/pause"))).await;
    let not_found = Reason::JobNotFound {
        id: unknown_id.into(),
    };
    assert_eq!((status, body.reason), (StatusCode::NOT_FOUND, not_found));

    let id = d.add("file.bin").await;
    d.wait_until(&id, JobState::Completed).await;
    let (status, body) = refused(d.post(&format!("/jobs/{id}/resume"))).await;
    let conflict = Reason::InvalidTransition {
        from: JobState::Completed,
        to: JobState::Queued,
    };
    assert_eq!((status, body.reason), (StatusCode::CONFLICT, conflict));
    assert!(body.error.contains("Completed"), "{}", body.error);
}

#[tokio::test]
async fn failures_carry_a_problem_code() {
    let d = start().await;
    let id = d.add("missing.bin").await;
    d.wait_until(&id, JobState::Failed).await;
    let job = d.job(&id).await;
    assert_eq!(job.problem, Some(Problem::HttpStatus { status: 404 }));
    assert_eq!(job.error.as_deref(), Some("server answered 404 Not Found"));
}

#[tokio::test]
async fn the_ui_is_served_without_the_token() {
    let ui = tempfile::tempdir().unwrap();
    std::fs::write(ui.path().join("index.html"), "<!doctype html>app").unwrap();
    std::fs::create_dir_all(ui.path().join("_app/immutable")).unwrap();
    std::fs::write(ui.path().join("_app/immutable/start.abc.js"), "code").unwrap();
    std::fs::write(ui.path().parent().unwrap().join("secret.txt"), "no").unwrap();
    let d = start_with(UiSource::Folder(ui.path().to_owned())).await;
    let get = |path: &str| d.client.get(format!("http://{}{path}", d.addr)).send();

    let index = get("/").await.unwrap();
    assert_eq!(index.headers()["cache-control"], "no-cache");
    assert!(
        index.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert_eq!(index.text().await.unwrap(), "<!doctype html>app");

    let script = get("/_app/immutable/start.abc.js").await.unwrap();
    assert!(
        script.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("immutable")
    );
    assert!(
        script.headers()["content-type"]
            .to_str()
            .unwrap()
            .contains("javascript")
    );

    // A page of the app gets index.html; a missing file doesn't.
    assert_eq!(
        get("/settings").await.unwrap().text().await.unwrap(),
        "<!doctype html>app"
    );
    assert_eq!(
        get("/missing.js").await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    // Nothing outside the folder, and the API stays the API.
    let escape = get("/..%2Fsecret.txt").await.unwrap();
    assert_ne!(escape.text().await.unwrap(), "no");
    let (status, body) = error_of(get("/api/v2/jobs").await.unwrap()).await;
    assert_eq!(
        (status, body.reason),
        (StatusCode::NOT_FOUND, Reason::NoSuchEndpoint)
    );
}

#[tokio::test]
async fn without_a_ui_build_the_page_says_how_to_make_one() {
    let empty = tempfile::tempdir().unwrap();
    let d = start_with(UiSource::Folder(empty.path().to_owned())).await;
    let page = d
        .client
        .get(format!("http://{}/", d.addr))
        .send()
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        page.text()
            .await
            .unwrap()
            .contains("pnpm --filter ui build")
    );
}

#[tokio::test]
async fn settings_are_read_and_changed() {
    let d = start().await;
    let offered = format!("{EVENTS_PROTOCOL}, {TOKEN_PROTOCOL_PREFIX}{TOKEN}");
    let (mut events, _) = d.events(&offered).await.unwrap();
    let settings = || async {
        let response = d.get("/settings").send().await.unwrap();
        response.json::<Settings>().await.unwrap()
    };
    let defaults = Settings {
        setup_done: false,
        max_concurrent: 3,
        global_limit: 0,
        max_retries: 10,
        retries_before_yield: 3,
        first_retry_wait_ms: 2000,
        appearance: None,
        language: None,
    };
    assert_eq!(settings().await, defaults, "a new daemon offers the setup");

    let patch = |change: serde_json::Value| {
        let request = d.client.patch(d.url("/settings")).bearer_auth(TOKEN);
        request.json(&change).send()
    };
    let change = serde_json::json!({
        "setupDone": true,
        "maxConcurrent": 5,
        "globalLimit": 4_000_000,
        "firstRetryWaitMs": 500,
        "appearance": { "mode": "dark", "palette": "neutral" },
        "language": "fr",
    });
    let response = patch(change).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let changed = Settings {
        setup_done: true,
        max_concurrent: 5,
        global_limit: 4_000_000,
        first_retry_wait_ms: 500,
        appearance: Some(Appearance {
            mode: AppearanceMode::Dark,
            palette: Palette::Neutral,
        }),
        language: Some("fr".into()),
        ..defaults
    };
    assert_eq!(settings().await, changed);
    // Every client hears of it, in one event.
    tokio::time::timeout(TIMEOUT, async {
        loop {
            let Some(Ok(Message::Text(text))) = events.next().await else {
                continue;
            };
            if let Event::SettingsChanged { settings } = serde_json::from_str(&text).unwrap() {
                assert_eq!(settings, changed);
                return;
            }
        }
    })
    .await
    .expect("no settingsChanged event");

    // Fields left out stay as they are.
    patch(serde_json::to_value(SettingsChange::default()).unwrap())
        .await
        .unwrap();
    assert_eq!(settings().await, changed);

    // Unknown values are refused, and nothing changes.
    let wrong = serde_json::json!({ "maxConcurrent": 1, "appearance": { "mode": "teal", "palette": "sea" } });
    let response = patch(wrong).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = response.json::<ApiError>().await.unwrap();
    assert_eq!(body.reason, Reason::InvalidBody);
    assert_eq!(settings().await, changed);

    // So are values out of their range, and odd languages.
    let refused = |change: serde_json::Value| async {
        let response = patch(change).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        response.json::<ApiError>().await.unwrap().reason
    };
    let too_many = serde_json::json!({ "maxRetries": 2, "maxConcurrent": 0 });
    let out_of_range = Reason::OutOfRange {
        setting: "maxConcurrent".into(),
        min: 1,
        max: 50,
    };
    assert_eq!(refused(too_many).await, out_of_range);
    let odd = serde_json::json!({ "language": "../fr" });
    let invalid = Reason::InvalidLanguage {
        language: "../fr".into(),
    };
    assert_eq!(refused(odd).await, invalid);
    assert_eq!(settings().await, changed);
}

#[tokio::test]
async fn categories_are_listed_added_matched_changed_and_removed() {
    let d = start().await;
    let offered = format!("{EVENTS_PROTOCOL}, {TOKEN_PROTOCOL_PREFIX}{TOKEN}");
    let (mut events, _) = d.events(&offered).await.unwrap();
    let list = || async {
        let response = d.get("/categories").send().await.unwrap();
        response.json::<Vec<Category>>().await.unwrap()
    };
    let built_in: Vec<_> = list().await.iter().map(|c| c.built_in).collect();
    assert_eq!(built_in, [Some(BuiltIn::General), Some(BuiltIn::Other)]);

    let folder = d.dir.path().join("videos").to_string_lossy().into_owned();
    let videos = CategoryBody {
        name: "Videos".into(),
        folder: Some(folder.clone()),
        extensions: vec![".MKV".into(), "mp4".into()],
        speed_limit: 0,
        max_concurrent: 2,
    };
    let added = d.post("/categories").json(&videos).send().await.unwrap();
    assert_eq!(added.status(), StatusCode::CREATED);
    let id = added.json::<AddedCategory>().await.unwrap().id;
    let listed = list().await;
    assert_eq!(listed[2].extensions, ["mkv", "mp4"]);
    assert_eq!(listed[2].folder.as_deref(), Some(folder.as_str()));
    assert_eq!(listed[2].built_in, None);
    // Subscribers hear about it.
    tokio::time::timeout(TIMEOUT, async {
        loop {
            let Some(Ok(Message::Text(text))) = events.next().await else {
                continue;
            };
            if let Event::CategoryChanged { category } = serde_json::from_str(&text).unwrap() {
                assert_eq!(category.id, id);
                return;
            }
        }
    })
    .await
    .expect("no categoryChanged event");

    let link = serde_json::json!({ "url": format!("{}/Film.MKV?token=x", d.files) });
    let matched: Matched = d
        .post("/categories/match")
        .json(&link)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        (matched.category, matched.extension.as_deref()),
        (id, Some("mkv"))
    );
    // A folder of no category: Other (ID 2 on a new database).
    let elsewhere = d
        .dir
        .path()
        .join("elsewhere")
        .to_string_lossy()
        .into_owned();
    let link = serde_json::json!({ "url": format!("{}/Film.MKV", d.files), "destDir": elsewhere });
    let matched: Matched = d
        .post("/categories/match")
        .json(&link)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!((matched.category, matched.by_folder), (2, true));

    let taken = CategoryBody {
        name: "Films".into(),
        extensions: vec!["mkv".into()],
        ..videos.clone()
    };
    let (status, refused) =
        error_of(d.post("/categories").json(&taken).send().await.unwrap()).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(matches!(refused.reason, Reason::ExtensionTaken { .. }));
    let relative = CategoryBody {
        name: "Films".into(),
        folder: Some("films".into()),
        extensions: vec![],
        ..videos.clone()
    };
    let (status, refused) =
        error_of(d.post("/categories").json(&relative).send().await.unwrap()).await;
    assert_eq!(
        (status, refused.reason),
        (StatusCode::BAD_REQUEST, Reason::RelativeFolder)
    );

    let renamed = CategoryBody {
        name: "Movies".into(),
        ..videos
    };
    let changed = d
        .client
        .put(d.url(&format!("/categories/{id}")))
        .bearer_auth(TOKEN);
    assert_eq!(
        changed.json(&renamed).send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(list().await[2].name, "Movies");

    // A download can be moved in, and goes back to General when its category goes.
    let job = d.add("file.bin").await;
    let moved = d
        .post(&format!("/jobs/{job}/category"))
        .json(&serde_json::json!({ "category": id }));
    assert_eq!(moved.send().await.unwrap().status(), StatusCode::NO_CONTENT);
    assert_eq!(d.job(&job).await.category, id);
    let removed = d
        .client
        .delete(d.url(&format!("/categories/{id}")))
        .bearer_auth(TOKEN);
    assert_eq!(
        removed.send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(d.job(&job).await.category, 1);
    assert_eq!(list().await.len(), 2);

    let general = d.client.delete(d.url("/categories/1")).bearer_auth(TOKEN);
    let (status, refused) = error_of(general.send().await.unwrap()).await;
    let category = "General".into();
    assert_eq!(
        (status, refused.reason),
        (
            StatusCode::CONFLICT,
            Reason::CannotRemoveBuiltIn { category }
        )
    );
}
