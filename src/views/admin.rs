use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Path, State};
use axum_extra::extract::Form;
use axum::response::{Html, Redirect, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::incident::{Incident, IncidentUpdate, IncidentWithUpdates};
use crate::models::maintenance::{Maintenance, MaintenanceWithServices};
use crate::models::service::Service;
use crate::models::settings::{FooterLink, SiteSettings};
use crate::probes::{ProbeCommand, ProbeScheduler};

use std::sync::Arc;

// ── Login ───────────────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

pub async fn login_page(State(db): State<DbPool>) -> Result<Response, AppError> {
    let done = db
        .read(|conn| crate::db::settings::is_onboarding_complete(conn))
        .await?;
    if !done {
        return Ok(Redirect::to("/admin/setup").into_response());
    }
    Ok(LoginTemplate { error: None }.into_response())
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub password: String,
}

pub async fn login_submit(
    State(db): State<DbPool>,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    let done = db
        .read(|conn| crate::db::settings::is_onboarding_complete(conn))
        .await?;
    if !done {
        return Ok(Redirect::to("/admin/setup").into_response());
    }

    let pw = form.password.clone();
    let valid = db
        .read(move |conn| {
            let hash: String = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'admin_password_hash'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|_| AppError::Unauthorized)?;
            auth::verify_password(&pw, &hash)
        })
        .await?;

    if !valid {
        let tmpl = LoginTemplate {
            error: Some("Invalid password".into()),
        };
        return Ok(Html(
            tmpl.render()
                .map_err(|e| AppError::Internal(e.to_string()))?,
        )
        .into_response());
    }

    let token = uuid::Uuid::new_v4().to_string();
    let token_clone = token.clone();
    db.write(move |conn| auth::create_session(conn, &token_clone))
        .await?;

    Ok((
        [(
            axum::http::header::SET_COOKIE,
            format!(
                "session={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800"
            ),
        )],
        Redirect::to("/admin"),
    )
        .into_response())
}

pub async fn logout(State(db): State<DbPool>, req: axum::extract::Request) -> Result<Response, AppError> {
    if let Some(token) = extract_session_token(&req) {
        let _ = db
            .write(move |conn| auth::delete_session(conn, &token))
            .await;
    }
    Ok((
        [(
            axum::http::header::SET_COOKIE,
            "session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0".to_string(),
        )],
        Redirect::to("/admin/login"),
    )
        .into_response())
}

fn extract_session_token(req: &axum::extract::Request) -> Option<String> {
    let cookie_header = req.headers().get("cookie")?.to_str().ok()?;
    cookie_header
        .split(';')
        .find_map(|c| c.trim().strip_prefix("session=").map(|v| v.to_string()))
}

use axum::response::IntoResponse;

// ── Onboarding ─────────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/onboarding.html")]
pub struct OnboardingTemplate {
    pub error: Option<String>,
    pub site_name: String,
    pub footer_links: Vec<FooterLink>,
}

pub async fn onboarding_page(State(db): State<DbPool>) -> Result<Response, AppError> {
    let done = db
        .read(|conn| crate::db::settings::is_onboarding_complete(conn))
        .await?;
    if done {
        return Ok(Redirect::to("/admin/login").into_response());
    }

    Ok(OnboardingTemplate {
        error: None,
        site_name: String::new(),
        footer_links: Vec::new(),
    }
    .into_response())
}

#[derive(Deserialize)]
pub struct OnboardingForm {
    pub site_name: String,
    pub password: String,
    pub password_confirm: String,
    #[serde(default)]
    pub footer_links_json: String,
}

fn parse_footer_links(json: &str) -> Vec<FooterLink> {
    serde_json::from_str::<Vec<FooterLink>>(json).unwrap_or_default()
}

pub async fn onboarding_submit(
    State(db): State<DbPool>,
    Form(form): Form<OnboardingForm>,
) -> Result<Response, AppError> {
    let done = db
        .read(|conn| crate::db::settings::is_onboarding_complete(conn))
        .await?;
    if done {
        return Ok(Redirect::to("/admin/login").into_response());
    }

    let footer_links = parse_footer_links(&form.footer_links_json);

    if form.password != form.password_confirm {
        return Ok(OnboardingTemplate {
            error: Some("Passwords don't match".into()),
            site_name: form.site_name,
            footer_links,
        }
        .into_response());
    }

    if form.password.len() < 8 {
        return Ok(OnboardingTemplate {
            error: Some("Password must be at least 8 characters".into()),
            site_name: form.site_name,
            footer_links,
        }
        .into_response());
    }

    let password = form.password;
    let settings = SiteSettings {
        site_name: form.site_name,
        footer_links,
    };

    db.write(move |conn| {
        settings.save(conn)?;
        let hash = auth::hash_password(&password)?;
        crate::db::settings::set(conn, "admin_password_hash", &hash)?;
        crate::db::settings::set(conn, "onboarding_complete", "true")?;
        Ok(())
    })
    .await?;

    Ok(Redirect::to("/admin/login").into_response())
}

// ── Site Settings ──────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/settings.html")]
pub struct SettingsTemplate {
    pub site_name: String,
    pub settings: SiteSettings,
    pub success: Option<String>,
    pub error: Option<String>,
}

pub async fn settings_page(State(db): State<DbPool>) -> Result<SettingsTemplate, AppError> {
    let settings = db.read(|conn| SiteSettings::load(conn)).await?;
    Ok(SettingsTemplate {
        site_name: settings.site_name.clone(),
        settings,
        success: None,
        error: None,
    })
}

#[derive(Deserialize)]
pub struct SettingsForm {
    pub site_name: String,
    #[serde(default)]
    pub footer_links_json: String,
}

pub async fn settings_update(
    State(db): State<DbPool>,
    Form(form): Form<SettingsForm>,
) -> Result<Response, AppError> {
    let site_name = form.site_name.trim().to_string();
    if site_name.is_empty() {
        let settings = db.read(|conn| SiteSettings::load(conn)).await?;
        return Ok(SettingsTemplate {
            site_name: settings.site_name.clone(),
            settings,
            success: None,
            error: Some("Site name cannot be empty".into()),
        }
        .into_response());
    }

    let footer_links = parse_footer_links(&form.footer_links_json);

    let settings = SiteSettings {
        site_name,
        footer_links,
    };

    db.write(move |conn| settings.save(conn)).await?;

    let settings = db.read(|conn| SiteSettings::load(conn)).await?;
    Ok(SettingsTemplate {
        site_name: settings.site_name.clone(),
        settings,
        success: Some("Settings saved successfully".into()),
        error: None,
    }
    .into_response())
}

// ── Dashboard ───────────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardTemplate {
    pub site_name: String,
    pub total_services: usize,
    pub ok_services: usize,
    pub degraded_services: usize,
    pub down_services: usize,
    pub active_incidents: usize,
    pub upcoming_maintenance: usize,
    pub services: Vec<Service>,
}

pub async fn dashboard(State(db): State<DbPool>) -> Result<DashboardTemplate, AppError> {
    let (services, incidents, maintenance, settings) = tokio::try_join!(
        db.read(|conn| crate::db::services::list_all(conn)),
        db.read(|conn| crate::db::incidents::list_active(conn)),
        db.read(|conn| crate::db::maintenance::list_upcoming(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;

    Ok(DashboardTemplate {
        site_name: settings.site_name,
        total_services: services.len(),
        ok_services: services.iter().filter(|s| s.status == "ok").count(),
        degraded_services: services.iter().filter(|s| s.status == "degraded").count(),
        down_services: services.iter().filter(|s| s.status == "down").count(),
        active_incidents: incidents.len(),
        upcoming_maintenance: maintenance.len(),
        services,
    })
}

// ── Services ────────────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/services_list.html")]
pub struct ServicesListTemplate {
    pub site_name: String,
    pub services: Vec<Service>,
}

pub async fn services_list(State(db): State<DbPool>) -> Result<ServicesListTemplate, AppError> {
    let (services, settings) = tokio::try_join!(
        db.read(|conn| crate::db::services::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;
    Ok(ServicesListTemplate {
        site_name: settings.site_name,
        services,
    })
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/service_form.html")]
pub struct ServiceFormTemplate {
    pub site_name: String,
    pub service: Option<Service>,
    pub is_edit: bool,
}

pub async fn service_new(State(db): State<DbPool>) -> Result<ServiceFormTemplate, AppError> {
    let settings = db.read(|conn| SiteSettings::load(conn)).await?;
    Ok(ServiceFormTemplate {
        site_name: settings.site_name,
        service: None,
        is_edit: false,
    })
}

pub async fn service_edit(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<ServiceFormTemplate, AppError> {
    let (svc, settings) = tokio::try_join!(
        db.read(move |conn| crate::db::services::get_by_id(conn, &id)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;
    Ok(ServiceFormTemplate {
        site_name: settings.site_name,
        service: Some(svc.ok_or(AppError::NotFound)?),
        is_edit: true,
    })
}

#[derive(Deserialize)]
pub struct ServiceForm {
    pub name: String,
    pub category: String,
    pub url: String,
    pub probe_type: String,
    pub interval_secs: i64,
    pub region: String,
    pub is_public: Option<String>,
    pub enabled: Option<String>,
    pub sort_order: i64,
}

pub async fn service_create(
    State(db): State<DbPool>,
    State(scheduler): State<Arc<ProbeScheduler>>,
    Form(form): Form<ServiceForm>,
) -> Result<Redirect, AppError> {
    let svc = Service {
        id: uuid::Uuid::new_v4().to_string(),
        name: form.name,
        category: form.category,
        url: form.url,
        probe_type: form.probe_type,
        probe_config: "{}".into(),
        interval_secs: form.interval_secs,
        status: "unknown".into(),
        uptime_pct: 100.0,
        avg_latency_ms: 0.0,
        region: form.region,
        is_public: form.is_public.is_some(),
        enabled: form.enabled.is_some(),
        sort_order: form.sort_order,
        created_at: String::new(),
        updated_at: String::new(),
    };

    let svc_clone = svc.clone();
    db.write(move |conn| crate::db::services::insert(conn, &svc_clone))
        .await?;

    if svc.enabled {
        scheduler.send(ProbeCommand::Add(svc)).await;
    }

    Ok(Redirect::to("/admin/services"))
}

pub async fn service_update(
    State(db): State<DbPool>,
    State(scheduler): State<Arc<ProbeScheduler>>,
    Path(id): Path<String>,
    Form(form): Form<ServiceForm>,
) -> Result<Redirect, AppError> {
    let svc = Service {
        id,
        name: form.name,
        category: form.category,
        url: form.url,
        probe_type: form.probe_type,
        probe_config: "{}".into(),
        interval_secs: form.interval_secs,
        status: String::new(),
        uptime_pct: 0.0,
        avg_latency_ms: 0.0,
        region: form.region,
        is_public: form.is_public.is_some(),
        enabled: form.enabled.is_some(),
        sort_order: form.sort_order,
        created_at: String::new(),
        updated_at: String::new(),
    };

    let svc_clone = svc.clone();
    db.write(move |conn| crate::db::services::update(conn, &svc_clone))
        .await?;

    scheduler.send(ProbeCommand::Reload(svc)).await;

    Ok(Redirect::to("/admin/services"))
}

pub async fn service_delete(
    State(db): State<DbPool>,
    State(scheduler): State<Arc<ProbeScheduler>>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    let id_clone = id.clone();
    db.write(move |conn| crate::db::services::delete(conn, &id_clone))
        .await?;
    scheduler.send(ProbeCommand::Remove(id)).await;
    Ok(Redirect::to("/admin/services"))
}

// ── Import / Export ─────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/import_export.html")]
pub struct ImportExportTemplate {
    pub site_name: String,
    pub services: Vec<Service>,
}

pub async fn import_export_page(State(db): State<DbPool>) -> Result<ImportExportTemplate, AppError> {
    let (services, settings) = tokio::try_join!(
        db.read(|conn| crate::db::services::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;
    Ok(ImportExportTemplate {
        site_name: settings.site_name,
        services,
    })
}

#[derive(Deserialize)]
pub struct ImportPayload {
    pub services: Vec<ImportService>,
}

#[derive(Deserialize)]
pub struct ImportService {
    pub name: String,
    pub category: Option<String>,
    pub url: String,
    pub probe_type: String,
    pub probe_config: Option<String>,
    pub interval_secs: Option<i64>,
    pub region: Option<String>,
    pub is_public: Option<bool>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i64>,
}

#[derive(Serialize)]
pub struct ImportResponse {
    pub imported: usize,
    pub skipped: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn service_import(
    State(db): State<DbPool>,
    State(scheduler): State<Arc<ProbeScheduler>>,
    Json(payload): Json<ImportPayload>,
) -> Result<Json<ImportResponse>, AppError> {
    let mut imported = 0usize;
    let mut skipped = 0usize;

    // Get existing service names to detect conflicts
    let existing = db
        .read(|conn| crate::db::services::list_all(conn))
        .await?;
    let existing_names: std::collections::HashSet<String> = existing
        .iter()
        .map(|s| s.name.to_lowercase())
        .collect();

    for imp in &payload.services {
        if existing_names.contains(&imp.name.to_lowercase()) {
            skipped += 1;
            continue;
        }

        let interval = imp.interval_secs.unwrap_or(60).max(10);

        let svc = Service {
            id: uuid::Uuid::new_v4().to_string(),
            name: imp.name.clone(),
            category: imp.category.clone().unwrap_or_default(),
            url: imp.url.clone(),
            probe_type: imp.probe_type.clone(),
            probe_config: imp.probe_config.clone().unwrap_or_else(|| "{}".into()),
            interval_secs: interval,
            status: "unknown".into(),
            uptime_pct: 100.0,
            avg_latency_ms: 0.0,
            region: imp.region.clone().unwrap_or_default(),
            is_public: imp.is_public.unwrap_or(true),
            enabled: imp.enabled.unwrap_or(true),
            sort_order: imp.sort_order.unwrap_or(0),
            created_at: String::new(),
            updated_at: String::new(),
        };

        let svc_clone = svc.clone();
        db.write(move |conn| crate::db::services::insert(conn, &svc_clone))
            .await?;

        if svc.enabled {
            scheduler.send(ProbeCommand::Add(svc)).await;
        }

        imported += 1;
    }

    Ok(Json(ImportResponse {
        imported,
        skipped,
        error: None,
    }))
}

// ── Incidents ───────────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/incidents_list.html")]
pub struct IncidentsListTemplate {
    pub site_name: String,
    pub incidents: Vec<IncidentWithUpdates>,
}

pub async fn incidents_list(State(db): State<DbPool>) -> Result<IncidentsListTemplate, AppError> {
    let (incidents, settings) = tokio::try_join!(
        db.read(|conn| crate::db::incidents::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;
    Ok(IncidentsListTemplate {
        site_name: settings.site_name,
        incidents,
    })
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/incident_form.html")]
pub struct IncidentFormTemplate {
    pub site_name: String,
    pub incident: Option<IncidentWithUpdates>,
    pub services: Vec<Service>,
    pub is_edit: bool,
}

pub async fn incident_new(State(db): State<DbPool>) -> Result<IncidentFormTemplate, AppError> {
    let (services, settings) = tokio::try_join!(
        db.read(|conn| crate::db::services::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;
    Ok(IncidentFormTemplate {
        site_name: settings.site_name,
        incident: None,
        services,
        is_edit: false,
    })
}

pub async fn incident_edit(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<IncidentFormTemplate, AppError> {
    let (incident, services, settings) = tokio::try_join!(
        db.read({
            let id = id.clone();
            move |conn| crate::db::incidents::get_by_id(conn, &id)
        }),
        db.read(|conn| crate::db::services::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;

    Ok(IncidentFormTemplate {
        site_name: settings.site_name,
        incident: Some(incident.ok_or(AppError::NotFound)?),
        services,
        is_edit: true,
    })
}

#[derive(Deserialize)]
pub struct IncidentForm {
    pub title: String,
    pub severity: String,
    pub status: String,
    #[serde(default)]
    pub service_ids: Vec<String>,
    pub message: Option<String>,
}

pub async fn incident_create(
    State(db): State<DbPool>,
    Form(form): Form<IncidentForm>,
) -> Result<Redirect, AppError> {
    let inc = Incident {
        id: uuid::Uuid::new_v4().to_string(),
        title: form.title,
        status: form.status.clone(),
        severity: form.severity,
        started_at: String::new(),
        resolved_at: None,
        is_public: true,
        created_at: String::new(),
    };

    let inc_clone = inc.clone();
    let service_ids = form.service_ids.clone();
    db.write(move |conn| crate::db::incidents::insert(conn, &inc_clone, &service_ids))
        .await?;

    if let Some(msg) = form.message {
        if !msg.trim().is_empty() {
            let update = IncidentUpdate {
                id: uuid::Uuid::new_v4().to_string(),
                incident_id: inc.id.clone(),
                status: form.status,
                message: msg,
                created_by: "admin".into(),
                created_at: String::new(),
            };
            db.write(move |conn| crate::db::incidents::add_update(conn, &update))
                .await?;
        }
    }

    Ok(Redirect::to("/admin/incidents"))
}

pub async fn incident_update(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Form(form): Form<IncidentForm>,
) -> Result<Redirect, AppError> {
    let inc = Incident {
        id,
        title: form.title,
        status: form.status,
        severity: form.severity,
        started_at: String::new(),
        resolved_at: None,
        is_public: true,
        created_at: String::new(),
    };

    let inc_clone = inc.clone();
    let service_ids = form.service_ids;
    db.write(move |conn| crate::db::incidents::update_incident(conn, &inc_clone, &service_ids))
        .await?;

    Ok(Redirect::to("/admin/incidents"))
}

#[derive(Deserialize)]
pub struct IncidentUpdateForm {
    pub status: String,
    pub message: String,
}

pub async fn incident_add_update(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Form(form): Form<IncidentUpdateForm>,
) -> Result<Redirect, AppError> {
    let update = IncidentUpdate {
        id: uuid::Uuid::new_v4().to_string(),
        incident_id: id,
        status: form.status,
        message: form.message,
        created_by: "admin".into(),
        created_at: String::new(),
    };
    db.write(move |conn| crate::db::incidents::add_update(conn, &update))
        .await?;
    Ok(Redirect::to("/admin/incidents"))
}

pub async fn incident_delete(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    db.write(move |conn| crate::db::incidents::delete(conn, &id))
        .await?;
    Ok(Redirect::to("/admin/incidents"))
}

// ── Maintenance ─────────────────────────────────────────────

#[derive(Template, WebTemplate)]
#[template(path = "admin/maintenance_list.html")]
pub struct MaintenanceListTemplate {
    pub site_name: String,
    pub items: Vec<MaintenanceWithServices>,
}

pub async fn maintenance_list(
    State(db): State<DbPool>,
) -> Result<MaintenanceListTemplate, AppError> {
    let (items, settings) = tokio::try_join!(
        db.read(|conn| crate::db::maintenance::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;
    Ok(MaintenanceListTemplate {
        site_name: settings.site_name,
        items,
    })
}

#[derive(Template, WebTemplate)]
#[template(path = "admin/maintenance_form.html")]
pub struct MaintenanceFormTemplate {
    pub site_name: String,
    pub maintenance: Option<MaintenanceWithServices>,
    pub services: Vec<Service>,
    pub is_edit: bool,
}

pub async fn maintenance_new(
    State(db): State<DbPool>,
) -> Result<MaintenanceFormTemplate, AppError> {
    let (services, settings) = tokio::try_join!(
        db.read(|conn| crate::db::services::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;
    Ok(MaintenanceFormTemplate {
        site_name: settings.site_name,
        maintenance: None,
        services,
        is_edit: false,
    })
}

pub async fn maintenance_edit(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<MaintenanceFormTemplate, AppError> {
    let (maint, services, settings) = tokio::try_join!(
        db.read({
            let id = id.clone();
            move |conn| crate::db::maintenance::get_by_id(conn, &id)
        }),
        db.read(|conn| crate::db::services::list_all(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;

    Ok(MaintenanceFormTemplate {
        site_name: settings.site_name,
        maintenance: Some(maint.ok_or(AppError::NotFound)?),
        services,
        is_edit: true,
    })
}

#[derive(Deserialize)]
pub struct MaintenanceForm {
    pub title: String,
    pub status: String,
    pub start_time: String,
    pub end_time: String,
    pub impact: String,
    pub notes: String,
    #[serde(default)]
    pub service_ids: Vec<String>,
}

pub async fn maintenance_create(
    State(db): State<DbPool>,
    Form(form): Form<MaintenanceForm>,
) -> Result<Redirect, AppError> {
    let m = Maintenance {
        id: uuid::Uuid::new_v4().to_string(),
        title: form.title,
        status: form.status,
        start_time: form.start_time,
        end_time: form.end_time,
        impact: form.impact,
        notes: form.notes,
        created_at: String::new(),
    };
    let service_ids = form.service_ids;
    db.write(move |conn| crate::db::maintenance::insert(conn, &m, &service_ids))
        .await?;
    Ok(Redirect::to("/admin/maintenance"))
}

pub async fn maintenance_update(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Form(form): Form<MaintenanceForm>,
) -> Result<Redirect, AppError> {
    let m = Maintenance {
        id,
        title: form.title,
        status: form.status,
        start_time: form.start_time,
        end_time: form.end_time,
        impact: form.impact,
        notes: form.notes,
        created_at: String::new(),
    };
    let service_ids = form.service_ids;
    db.write(move |conn| crate::db::maintenance::update(conn, &m, &service_ids))
        .await?;
    Ok(Redirect::to("/admin/maintenance"))
}

pub async fn maintenance_delete(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    db.write(move |conn| crate::db::maintenance::delete(conn, &id))
        .await?;
    Ok(Redirect::to("/admin/maintenance"))
}
