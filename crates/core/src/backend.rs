use std::future::Future;

use reqwest::Client;
use tokio_util::sync::CancellationToken;

use crate::download::{Download, Event, Outcome};
use crate::error::Error;
use crate::http::{self, HttpConfig};
use crate::throttle::Throttle;

/// Something that can carry out a download. The manager only talks to this,
/// so other protocols (aria2, yt-dlp…) can plug in later.
pub trait Backend: Send + Sync + 'static {
    /// Same contract as [`crate::run`]: download until done or until `cancel`
    /// fires, keeping `dl` in step with what's on disk, and pass every
    /// downloaded byte through `throttle` so speed limits apply.
    fn run(
        &self,
        dl: &mut Download,
        throttle: &Throttle,
        cancel: &CancellationToken,
        on_event: &mut (dyn FnMut(Event) + Send),
    ) -> impl Future<Output = Result<Outcome, Error>> + Send;
}

/// HTTP(S) downloads, split across several connections when possible.
pub struct HttpBackend {
    client: Client,
    config: HttpConfig,
}

impl HttpBackend {
    pub fn new() -> Result<Self, Error> {
        Self::with_config(HttpConfig::default())
    }

    pub fn with_config(config: HttpConfig) -> Result<Self, Error> {
        Ok(Self {
            client: http::build_client()?,
            config,
        })
    }
}

impl Backend for HttpBackend {
    async fn run(
        &self,
        dl: &mut Download,
        throttle: &Throttle,
        cancel: &CancellationToken,
        on_event: &mut (dyn FnMut(Event) + Send),
    ) -> Result<Outcome, Error> {
        http::run(&self.client, &self.config, dl, throttle, cancel, on_event).await
    }
}
