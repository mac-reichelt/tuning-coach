use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use tracing::warn;

use crate::{
    lap_validity::{DirtyReason, DirtyReasonCode, LapValidityEvent},
    AppState,
};

#[derive(Debug, Serialize)]
struct ErrorResponse<'a> {
    error: &'a str,
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/mark-lap-dirty", post(mark_lap_dirty))
        .route("/mark-lap-clean", post(mark_lap_clean))
        .route("/force-pit-start", post(force_pit_start))
        .route("/force-pit-end", post(force_pit_end))
        .route("/force-session-boundary", post(force_session_boundary))
}

enum ApiError {
    NoActiveSession,
    NoLapInProgress,
    AlreadyInPit,
    NotInPit,
    Internal,
}

impl ApiError {
    fn internal(err: impl std::fmt::Display) -> Self {
        tracing::error!(module = module_path!(), %err, "hotkey request failed");
        Self::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NoActiveSession => (StatusCode::SERVICE_UNAVAILABLE, "no active session"),
            Self::NoLapInProgress => (StatusCode::CONFLICT, "no lap in progress"),
            Self::AlreadyInPit => (StatusCode::CONFLICT, "already in pit stop"),
            Self::NotInPit => (StatusCode::CONFLICT, "not currently in pit stop"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

fn active_session_id(state: &AppState) -> Result<i64, ApiError> {
    state
        .storage
        .active_session_id()
        .map_err(ApiError::internal)?
        .ok_or(ApiError::NoActiveSession)
}

async fn mark_lap_dirty(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let session_id = active_session_id(&state)?;
    let lap = state
        .current_lap_context()
        .ok_or(ApiError::NoLapInProgress)?;
    let lap_id = state
        .storage
        .mark_lap_dirty_manual_override(session_id, lap.lap_number)
        .map_err(|err| match err {
            crate::storage::StorageError::Sqlite(rusqlite::Error::QueryReturnedNoRows) => {
                ApiError::NoLapInProgress
            }
            _ => ApiError::internal(err),
        })?;

    state.emit_lap_validity_event(LapValidityEvent::LapDirtyDetected {
        lap_id,
        reason: DirtyReason {
            code: DirtyReasonCode::ManualOverride,
            best_effort: false,
        },
        at_ms: lap.at_ms,
        lap_number: lap.lap_number,
    });

    let payload = json!({
        "lap_number": lap.lap_number,
        "lap_id": lap_id,
    });
    let marked_dirty_at = state
        .storage
        .insert_hotkey_event(session_id, Some(lap.at_ms), "mark_lap_dirty", &payload)
        .map_err(ApiError::internal)?;

    Ok(Json(json!({
        "lap_number": lap.lap_number,
        "marked_dirty_at": marked_dirty_at
    })))
}

async fn mark_lap_clean(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let session_id = active_session_id(&state)?;
    let lap = state
        .current_lap_context()
        .ok_or(ApiError::NoLapInProgress)?;
    let overridden_reason = state
        .storage
        .mark_lap_clean(session_id, lap.lap_number)
        .map_err(|err| match err {
            crate::storage::StorageError::Sqlite(rusqlite::Error::QueryReturnedNoRows) => {
                ApiError::NoLapInProgress
            }
            _ => ApiError::internal(err),
        })?;
    if let Some(reason) = &overridden_reason {
        if reason != DirtyReasonCode::ManualOverride.as_str() {
            warn!(
                module = module_path!(),
                %reason,
                lap_number = lap.lap_number,
                "manual clean override cleared a heuristic dirty reason"
            );
        }
    }

    state.emit_lap_validity_event(LapValidityEvent::LapCleanMarked {
        session_id,
        lap_number: lap.lap_number,
        at_ms: lap.at_ms,
    });

    let payload = json!({
        "lap_number": lap.lap_number,
        "overridden_reason": overridden_reason
    });
    state
        .storage
        .insert_hotkey_event(session_id, Some(lap.at_ms), "mark_lap_clean", &payload)
        .map_err(ApiError::internal)?;

    Ok(Json(json!({
        "lap_number": lap.lap_number
    })))
}

async fn force_pit_start(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let session_id = active_session_id(&state)?;
    let lap = state
        .current_lap_context()
        .ok_or(ApiError::NoLapInProgress)?;
    if state.pit_runtime().is_some() {
        return Err(ApiError::AlreadyInPit);
    }

    state
        .storage
        .mark_lap_pit_stop(session_id, lap.lap_number)
        .map_err(ApiError::internal)?;
    state.emit_lap_validity_event(LapValidityEvent::PitStopStarted {
        session_id,
        lap_number: lap.lap_number,
        at_ms: lap.at_ms,
    });

    let payload = json!({
        "session_id": session_id,
        "lap_number": lap.lap_number
    });
    state
        .storage
        .insert_hotkey_event(session_id, Some(lap.at_ms), "force_pit_start", &payload)
        .map_err(ApiError::internal)?;

    Ok(Json(payload))
}

async fn force_pit_end(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let session_id = active_session_id(&state)?;
    let pit = state.pit_runtime().ok_or(ApiError::NotInPit)?;
    let at_ms = state
        .current_lap_context()
        .map(|lap| lap.at_ms)
        .unwrap_or(pit.started_at_ms);
    let duration_s = (at_ms.saturating_sub(pit.started_at_ms)) as f32 / 1_000.0;
    state.emit_lap_validity_event(LapValidityEvent::PitStopEnded {
        session_id: pit.session_id,
        lap_number: pit.lap_number,
        at_ms,
        duration_s,
    });

    let payload = json!({
        "session_id": pit.session_id,
        "lap_number": pit.lap_number,
        "duration_s": duration_s
    });
    state
        .storage
        .insert_hotkey_event(session_id, Some(at_ms), "force_pit_end", &payload)
        .map_err(ApiError::internal)?;

    Ok(Json(json!({
        "duration_s": duration_s
    })))
}

async fn force_session_boundary(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let prior_session_id = active_session_id(&state)?;
    let lap = state.current_lap_context();
    state
        .storage
        .end_session(prior_session_id)
        .map_err(ApiError::internal)?;
    let new_session_id = state
        .storage
        .start_session(lap.map(|ctx| ctx.car_ordinal), env!("CARGO_PKG_VERSION"))
        .map_err(ApiError::internal)?;
    if let Some(ctx) = lap {
        state
            .storage
            .ensure_lap(new_session_id, ctx.lap_number, ctx.at_ms)
            .map_err(ApiError::internal)?;
    }

    state.clear_pit_runtime();
    let at_ms = lap.map(|ctx| ctx.at_ms).unwrap_or_default();
    state.emit_lap_validity_event(LapValidityEvent::SessionResetDetected {
        prior_session_id,
        new_session_id,
        at_ms,
    });

    let payload = json!({
        "prior_session_id": prior_session_id,
        "new_session_id": new_session_id
    });
    state
        .storage
        .insert_hotkey_event(
            new_session_id,
            lap.map(|ctx| ctx.at_ms),
            "force_session_boundary",
            &payload,
        )
        .map_err(ApiError::internal)?;

    Ok(Json(payload))
}
