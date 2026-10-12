//! `GET /events`: a WebSocket that pushes job changes and progress.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use ottie_core::JobEvent;
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;

use crate::AppState;
use crate::auth::EVENTS_PROTOCOL;
use crate::types::{Event, Job};

pub(crate) async fn subscribe(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    // Subscribe now rather than after the upgrade, so a client that fetches
    // `GET /jobs` once connected can't miss a change in between.
    let events = state.manager.subscribe();
    let shutdown = state.shutdown.clone();
    ws.protocols([EVENTS_PROTOCOL])
        .on_upgrade(move |socket| forward(socket, events, shutdown))
}

/// Sends engine events to the client until it leaves or the daemon stops.
async fn forward(
    mut socket: WebSocket,
    mut events: Receiver<JobEvent>,
    shutdown: CancellationToken,
) {
    loop {
        let event = tokio::select! {
            _ = shutdown.cancelled() => break,
            event = events.recv() => event,
            // The client sends nothing we use; this notices when it leaves.
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => continue,
            },
        };
        let message = match event {
            Ok(JobEvent::Changed(job)) => Event::Changed {
                job: Job::from(job),
            },
            Ok(JobEvent::Progress {
                id,
                downloaded,
                total,
            }) => Event::Progress {
                id: id.to_string(),
                downloaded,
                total,
            },
            // Not part of the API yet (restarts, categories).
            Ok(_) => continue,
            Err(RecvError::Lagged(_)) => Event::Resync,
            Err(RecvError::Closed) => break,
        };
        let Ok(text) = serde_json::to_string(&message) else {
            continue; // Can't happen: these types always serialize.
        };
        if socket.send(Message::Text(text.into())).await.is_err() {
            return;
        }
    }
    // Fails only if the client is already gone.
    let _ = socket.send(Message::Close(None)).await;
}
