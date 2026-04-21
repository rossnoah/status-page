use crate::db::DbPool;
use crate::error::AppError;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use rusqlite::{params, Connection};

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed =
        PasswordHash::new(hash).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn create_session(conn: &Connection, token: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO sessions (token, expires_at) VALUES (?1, datetime('now', '+7 days'))",
        params![token],
    )?;
    Ok(())
}

pub fn validate_session(conn: &Connection, token: &str) -> Result<bool, AppError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE token = ?1 AND expires_at > datetime('now')",
        params![token],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub fn delete_session(conn: &Connection, token: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
    Ok(())
}

pub fn cleanup_expired_sessions(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM sessions WHERE expires_at <= datetime('now')",
        [],
    )?;
    Ok(())
}

fn get_session_token(req: &Request) -> Option<String> {
    let cookie_header = req.headers().get("cookie")?.to_str().ok()?;
    cookie_header
        .split(';')
        .find_map(|c| c.trim().strip_prefix("session=").map(|v| v.to_string()))
}

pub async fn require_auth(
    State(db): State<DbPool>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let done = db
        .read(|conn| crate::db::settings::is_onboarding_complete(conn))
        .await?;
    if !done {
        return Ok(axum::response::Redirect::to("/admin/setup").into_response());
    }

    let token = get_session_token(&request).ok_or(AppError::Unauthorized)?;

    let valid = db
        .read(move |conn| {
            validate_session(conn, &token)
        })
        .await?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}
