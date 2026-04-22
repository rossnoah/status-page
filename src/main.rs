mod auth;
mod config;
mod db;
mod error;
mod models;
mod probes;
mod routes;
#[allow(dead_code)]
mod sketchy;
mod views;

use axum::body::Body;
use axum::extract::FromRef;
use axum::http::{header, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rust_embed::Embed;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: db::DbPool,
    pub scheduler: Arc<probes::ProbeScheduler>,
    pub login_limiter: Arc<auth::LoginRateLimiter>,
}

impl FromRef<AppState> for db::DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for Arc<probes::ProbeScheduler> {
    fn from_ref(state: &AppState) -> Self {
        state.scheduler.clone()
    }
}

impl FromRef<AppState> for Arc<auth::LoginRateLimiter> {
    fn from_ref(state: &AppState) -> Self {
        state.login_limiter.clone()
    }
}

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

async fn static_handler(req: Request<Body>) -> Response {
    let path = req.uri().path().trim_start_matches("/static/");
    match StaticAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime.as_ref().to_string()),
                    (
                        header::CACHE_CONTROL,
                        "public, max-age=86400".to_string(),
                    ),
                ],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,status_page=debug".parse().unwrap()),
        )
        .init();

    let cfg = config::Config::from_env();
    tracing::info!("Starting status-page on {}", cfg.listen);

    let pool = db::DbPool::new(&cfg.database_path).expect("Failed to initialize database");

    {
        let onboarded = pool
            .read(|conn| crate::db::settings::is_onboarding_complete(conn))
            .await
            .unwrap_or(false);

        if !onboarded {
            if let Some(ref pw) = cfg.admin_password {
                let pw = pw.clone();
                let _ = pool
                    .write(move |conn| {
                        let hash = auth::hash_password(&pw)?;
                        crate::db::settings::set(conn, "admin_password_hash", &hash)?;
                        crate::db::settings::set(conn, "site_name", "Status Page")?;
                        crate::db::settings::set(conn, "onboarding_complete", "true")?;
                        Ok(())
                    })
                    .await;
                tracing::info!("Admin password set from ADMIN_PASSWORD env var");
            } else {
                let has_password = pool
                    .read(|conn| {
                        Ok(crate::db::settings::get(conn, "admin_password_hash")?.is_some())
                    })
                    .await
                    .unwrap_or(false);

                if has_password {
                    let _ = pool
                        .write(|conn| {
                            crate::db::settings::set(conn, "onboarding_complete", "true")?;
                            Ok(())
                        })
                        .await;
                    tracing::info!("Existing database detected, marked as onboarded");
                } else {
                    tracing::info!("══════════════════════════════════════");
                    tracing::info!("  Visit /admin/setup to complete onboarding");
                    tracing::info!("══════════════════════════════════════");
                }
            }
        }
    }

    let scheduler = Arc::new(probes::ProbeScheduler::start(pool.clone()));

    let login_limiter = Arc::new(auth::LoginRateLimiter::new(
        10,
        std::time::Duration::from_secs(60),
    ));

    let state = AppState {
        db: pool.clone(),
        scheduler: scheduler.clone(),
        login_limiter,
    };

    let cleanup_db = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            let _ = cleanup_db
                .write(|conn| {
                    crate::db::uptime::cleanup_old_checks(conn, 90)?;
                    crate::auth::cleanup_expired_sessions(conn)?;
                    Ok(())
                })
                .await;
        }
    });

    let app = Router::new()
        .merge(routes::public::router())
        .merge(routes::admin::router(pool.clone()))
        .merge(routes::api::router())
        .route("/static/{*rest}", get(static_handler))
        .layer(CompressionLayer::new())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(cfg.listen)
        .await
        .expect("Failed to bind");

    tracing::info!("Listening on http://{}", cfg.listen);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Shutting down...");
}
