use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

use crate::auth;
use crate::db::DbPool;
use crate::views::admin;
use crate::AppState;

pub fn router(db: DbPool) -> Router<AppState> {
    let public_routes = Router::new()
        .route("/admin/setup", get(admin::onboarding_page).post(admin::onboarding_submit))
        .route("/admin/login", get(admin::login_page).post(admin::login_submit));

    let protected_routes = Router::new()
        .route("/admin", get(admin::dashboard))
        .route("/admin/logout", post(admin::logout))
        // Services
        .route("/admin/services", get(admin::services_list))
        .route(
            "/admin/services/new",
            get(admin::service_new).post(admin::service_create),
        )
        .route(
            "/admin/services/{id}/edit",
            get(admin::service_edit).post(admin::service_update),
        )
        .route("/admin/services/{id}/delete", post(admin::service_delete))
        .route("/admin/services/import-export", get(admin::import_export_page))
        .route("/admin/services/import", post(admin::service_import))
        // Incidents
        .route("/admin/incidents", get(admin::incidents_list))
        .route(
            "/admin/incidents/new",
            get(admin::incident_new).post(admin::incident_create),
        )
        .route(
            "/admin/incidents/{id}/edit",
            get(admin::incident_edit).post(admin::incident_update),
        )
        .route(
            "/admin/incidents/{id}/update",
            post(admin::incident_add_update),
        )
        .route("/admin/incidents/{id}/delete", post(admin::incident_delete))
        // Settings
        .route(
            "/admin/settings",
            get(admin::settings_page).post(admin::settings_update),
        )
        // Maintenance
        .route("/admin/maintenance", get(admin::maintenance_list))
        .route(
            "/admin/maintenance/new",
            get(admin::maintenance_new).post(admin::maintenance_create),
        )
        .route(
            "/admin/maintenance/{id}/edit",
            get(admin::maintenance_edit).post(admin::maintenance_update),
        )
        .route(
            "/admin/maintenance/{id}/delete",
            post(admin::maintenance_delete),
        )
        .route_layer(middleware::from_fn_with_state(db, auth::require_auth));

    public_routes.merge(protected_routes)
}
