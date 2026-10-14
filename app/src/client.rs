//! The app as a client of the daemon's HTTP API, like every other front end
//! (CLAUDE.md, Architecture): what the tray reads and does. Only the fields
//! it needs are read; the rest of each answer is ignored.

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tauri::Url;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// The events stream's protocol, and the prefix of the one carrying the
/// token (docs/decisions/0014).
const EVENTS_PROTOCOL: &str = "ottie.v1";
const TOKEN_PROTOCOL: &str = "ottie.token.";

pub type Events = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, Deserialize)]
pub struct Job {
    pub id: String,
    pub state: String,
    pub downloaded: u64,
    /// Its file is moving: it can't be paused or resumed until it's done.
    pub moving: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub global_limit: u64,
}

/// A message on the events stream; the others don't matter to the tray.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    Changed {
        job: Job,
    },
    Progress {
        id: String,
        downloaded: u64,
    },
    Removed {
        id: String,
    },
    SettingsChanged {
        settings: Settings,
    },
    Resync,
    #[serde(other)]
    Other,
}

pub struct Daemon {
    origin: Url,
    token: String,
    http: Client,
}

impl Daemon {
    /// From the signed-in link `ottie link` prints (`http://…/#token=…`).
    pub fn new(link: &Url) -> Result<Daemon> {
        let http = Client::builder()
            .no_proxy()
            // Pausing a running download waits until it has stopped.
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Daemon {
            origin: link.join("/")?,
            token: token(link).to_owned(),
            http,
        })
    }

    fn url(&self, path: &str) -> Result<Url> {
        Ok(self.origin.join(&format!("api/v1/{path}"))?)
    }

    /// The events stream. Connect first, then list, so nothing is missed.
    pub async fn events(&self) -> Result<Events> {
        let mut url = self.url("events")?;
        let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(scheme)
            .map_err(|()| anyhow::anyhow!("no WebSocket address for {url}"))?;
        let mut request = url.as_str().into_client_request()?;
        let offered = format!("{EVENTS_PROTOCOL}, {TOKEN_PROTOCOL}{}", self.token);
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", offered.parse()?);
        let (events, _) = tokio_tungstenite::connect_async(request)
            .await
            .context("can't follow Ottie's events")?;
        Ok(events)
    }

    pub async fn jobs(&self) -> Result<Vec<Job>> {
        Ok(self.get("jobs").await?.json().await?)
    }

    pub async fn settings(&self) -> Result<Settings> {
        Ok(self.get("settings").await?.json().await?)
    }

    pub async fn pause(&self, id: &str) -> Result<()> {
        self.post(&format!("jobs/{id}/pause")).await
    }

    pub async fn resume(&self, id: &str) -> Result<()> {
        self.post(&format!("jobs/{id}/resume")).await
    }

    /// Bytes per second for all downloads together; 0 = no limit.
    pub async fn set_global_limit(&self, limit: u64) -> Result<()> {
        self.http
            .patch(self.url("settings")?)
            .bearer_auth(&self.token)
            .json(&json!({ "globalLimit": limit }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<reqwest::Response> {
        Ok(self
            .http
            .get(self.url(path)?)
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?)
    }

    async fn post(&self, path: &str) -> Result<()> {
        self.http
            .post(self.url(path)?)
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

/// The token in a signed-in link's fragment (`#token=…`).
pub fn token(link: &Url) -> &str {
    link.fragment()
        .and_then(|fragment| fragment.strip_prefix("token="))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_read_only_what_the_tray_needs() {
        let progress: Event = serde_json::from_str(
            r#"{"type":"progress","id":"a","downloaded":5,"total":null,"ranges":[]}"#,
        )
        .unwrap();
        assert!(matches!(progress, Event::Progress { downloaded: 5, .. }));
        let view: Event = serde_json::from_str(r#"{"type":"viewRemoved","id":3}"#).unwrap();
        assert!(matches!(view, Event::Other));
        let settings: Event = serde_json::from_str(
            r#"{"type":"settingsChanged","settings":{"globalLimit":1024,"maxConcurrent":3}}"#,
        )
        .unwrap();
        assert!(matches!(
            settings,
            Event::SettingsChanged {
                settings: Settings { global_limit: 1024 }
            }
        ));
    }

    #[test]
    fn the_api_lives_under_the_daemons_origin() {
        let link = Url::parse("http://127.0.0.1:24068/#token=abc").unwrap();
        let daemon = Daemon::new(&link).unwrap();
        assert_eq!(token(&link), "abc");
        assert_eq!(
            daemon.url("jobs/x/pause").unwrap().as_str(),
            "http://127.0.0.1:24068/api/v1/jobs/x/pause"
        );
    }
}
