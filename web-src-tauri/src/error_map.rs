use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use pwmgr_core::error::CommandError;

/// Wraps a `pwmgr_core::error::CommandError` so it can be returned from an axum handler with `?`.
pub struct ApiError(pub CommandError);

impl From<CommandError> for ApiError {
    fn from(e: CommandError) -> Self {
        ApiError(e)
    }
}

/// The documented, reviewed convention (spec §2): map each sanitized error code to one HTTP status.
/// `username_taken`/`confirm_required` are 409 Conflict (the request conflicts with existing state
/// and the remedy is a changed/confirmed re-request); anything unrecognized is a 500 so a future
/// code can never silently masquerade as success.
pub fn status_for(code: &str) -> StatusCode {
    match code {
        "invalid_credentials" | "locked" => StatusCode::UNAUTHORIZED,
        "invalid_input" => StatusCode::BAD_REQUEST,
        "not_found" => StatusCode::NOT_FOUND,
        "username_taken" | "confirm_required" => StatusCode::CONFLICT,
        "forbidden" => StatusCode::FORBIDDEN,
        "throttled" | "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
        "internal_error" => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = status_for(self.0.code());
        (status, Json(self.0)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_for_maps_every_known_error_code() {
        assert_eq!(status_for("invalid_credentials"), StatusCode::UNAUTHORIZED);
        assert_eq!(status_for("locked"), StatusCode::UNAUTHORIZED);
        assert_eq!(status_for("invalid_input"), StatusCode::BAD_REQUEST);
        assert_eq!(status_for("not_found"), StatusCode::NOT_FOUND);
        assert_eq!(status_for("username_taken"), StatusCode::CONFLICT);
        assert_eq!(status_for("confirm_required"), StatusCode::CONFLICT);
        assert_eq!(status_for("forbidden"), StatusCode::FORBIDDEN);
        assert_eq!(status_for("throttled"), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(status_for("rate_limited"), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(status_for("internal_error"), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn status_for_maps_an_unknown_code_to_500() {
        assert_eq!(status_for("some_future_code"), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
