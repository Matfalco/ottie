use std::future::Future;

use reqwest::Client;
use tokio_util::sync::CancellationToken;

use crate::download::{Download, Event, Outcome};
use crate::error::Error;
use crate::http;

/// Something that can carry out a download. The manager only talks to this,
/// so other protocols (aria2, yt-dlp…) can plug in later.
pub trait Backend: Send + Sync + 'static {
    /// Same contract as [`crate::run`]: download until done or until `cancel`
    /// fires, keeping `dl` in step with what's on disk.
    fn run(
        &self,
        dl: &mut Download,
        cancel: &CancellationToken,
        on_event: &mut (dyn FnMut(Event) + Send),
    ) -> impl Future<Output = Result<Outcome, Error>> + Send;
}

/// Plain HTTP(S) downloads over a single connection.
pub struct HttpBackend {
    client: Client,
}

impl HttpBackend {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            client: http::build_client()?,
        })
    }
}

impl Backend for HttpBackend {
    async fn run(
        &self,
        dl: &mut Download,
        cancel: &CancellationToken,
        on_event: &mut (dyn FnMut(Event) + Send),
    ) -> Result<Outcome, Error> {
        http::run(&self.client, dl, cancel, on_event).await
    }
}
