//! `/speed`: the total download speed over the last hour, for a graph
//! (docs/decisions/0044).

use axum::Json;
use axum::extract::State;

use crate::AppState;
use crate::error::Failure;
use crate::types::SpeedHistory;

/// The total download speed, one sample per second, oldest first: up to the
/// last hour, since the daemon started. Each running download's speed is
/// estimated from its progress (smoothed), then they're added up. A client
/// keeps it going from the `progress` events.
#[utoipa::path(
    get,
    path = "/speed",
    operation_id = "getSpeedHistory",
    tag = "speed",
    responses((status = OK, description = "Up to an hour of samples", body = SpeedHistory))
)]
pub(crate) async fn history(State(state): State<AppState>) -> Result<Json<SpeedHistory>, Failure> {
    let samples = state.manager.speed_history().await?;
    Ok(Json(SpeedHistory { samples }))
}
