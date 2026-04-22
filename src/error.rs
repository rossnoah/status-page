use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};

#[derive(Debug)]
pub enum AppError {
    Db(rusqlite::Error),
    Pool(r2d2::Error),
    NotFound,
    Unauthorized,
    BadRequest(String),
    RateLimited,
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(e) => write!(f, "database error: {e}"),
            Self::Pool(e) => write!(f, "pool error: {e}"),
            Self::NotFound => write!(f, "not found"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::BadRequest(msg) => write!(f, "bad request: {msg}"),
            Self::RateLimited => write!(f, "rate limited"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            Self::Unauthorized => Redirect::to("/admin/login").into_response(),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            Self::RateLimited => {
                (StatusCode::TOO_MANY_REQUESTS, "Too many requests").into_response()
            }
            other => {
                tracing::error!("Internal error: {other}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e)
    }
}

impl From<r2d2::Error> for AppError {
    fn from(e: r2d2::Error) -> Self {
        Self::Pool(e)
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::Internal(e.to_string())
    }
}
