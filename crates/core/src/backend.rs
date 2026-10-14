use std::future::Future;

use tokio_util::sync::CancellationToken;

use crate::download::{Download, Event, Outcome};
use crate::error::Error;
use crate::http::{self, HttpConfig};
use crate::playback::Playback;
use crate::throttle::Throttle;

/// Something that can carry out a download. The manager only talks to this,
/// so other protocols (yt-dlp, FTP…) can plug in later.
pub trait Backend: Send + Sync + 'static {
    /// Same contract as [`crate::run`]: download until done or until `cancel`
    /// fires, keeping `dl` in step with what's on disk, and pass every
    /// downloaded byte through `throttle` so speed limits apply. `playback`
    /// tells where players read the file, so the download can go there first.
    fn run(
        &self,
        dl: &mut Download,
        throttle: &Throttle,
        playback: &Playback,
        cancel: &CancellationToken,
        on_event: &mut (dyn FnMut(Event) + Send),
    ) -> impl Future<Output = Result<Outcome, Error>> + Send;
}

/// HTTP(S) downloads, split across several connections when possible.
///
/// Each run of a download (start, resume, retry) gets an HTTP client of its
/// own, so it opens new connections. Connections kept from earlier may have
/// died silently when the network changed (a VPN going up or down, another
/// Wi-Fi): reusing one would hang until the stall timeout, again on every
/// retry. Within a run, its connections are still used again. A client costs
/// microseconds to build.
pub struct HttpBackend {
    config: HttpConfig,
}

impl HttpBackend {
    pub fn new() -> Result<Self, Error> {
        Self::with_config(HttpConfig::default())
    }

    /// Fails if no HTTP client can be built (TLS can't be set up).
    pub fn with_config(config: HttpConfig) -> Result<Self, Error> {
        http::build_client()?;
        Ok(Self { config })
    }
}

impl Backend for HttpBackend {
    async fn run(
        &self,
        dl: &mut Download,
        throttle: &Throttle,
        playback: &Playback,
        cancel: &CancellationToken,
        on_event: &mut (dyn FnMut(Event) + Send),
    ) -> Result<Outcome, Error> {
        let client = http::build_client()?;
        let env = http::Env {
            client: &client,
            config: &self.config,
            throttle,
            playback,
            cancel,
        };
        http::run_in(&env, dl, on_event).await
    }
}
