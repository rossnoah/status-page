use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::incident::IncidentWithUpdates;
use crate::models::maintenance::MaintenanceWithServices;
use crate::models::service::Service;
use crate::models::settings::{FooterLink, SiteSettings};
use crate::models::uptime::DailyAggregate;

pub struct ServiceCard {
    pub service: Service,
    pub days: Vec<DailyAggregate>,
    pub uptime_pct_90d: f64,
}

impl ServiceCard {
    pub fn uptime_fmt(&self) -> String {
        format!("{:.2}", self.uptime_pct_90d)
    }
    pub fn latency_fmt(&self) -> String {
        format!("{:.0}", self.service.avg_latency_ms)
    }
    pub fn category_lower(&self) -> String {
        self.service.category.to_lowercase()
    }
    pub fn name_lower(&self) -> String {
        self.service.name.to_lowercase()
    }
    pub fn empty_days(&self) -> usize {
        90usize.saturating_sub(self.days.len())
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "public/index.html")]
pub struct IndexTemplate {
    pub site_name: String,
    pub footer_links: Vec<FooterLink>,
    pub services: Vec<ServiceCard>,
    pub categories: Vec<String>,
    pub active_incidents: Vec<IncidentWithUpdates>,
    pub recent_resolved: Vec<IncidentWithUpdates>,
    pub upcoming_maintenance: Vec<MaintenanceWithServices>,
    pub overall_status: String,
    pub ok_count: usize,
    pub degraded_count: usize,
    pub down_count: usize,
}

pub async fn index(State(db): State<DbPool>) -> Result<IndexTemplate, AppError> {
    let (services, categories, active_incidents, recent_resolved, upcoming_maintenance, settings) = tokio::try_join!(
        db.read(|conn| crate::db::services::list_public(conn)),
        db.read(|conn| crate::db::services::categories(conn)),
        db.read(|conn| crate::db::incidents::list_active(conn)),
        db.read(|conn| crate::db::incidents::list_recent_resolved(conn, 5)),
        db.read(|conn| crate::db::maintenance::list_upcoming(conn)),
        db.read(|conn| SiteSettings::load(conn)),
    )?;

    let mut cards = Vec::new();
    for svc in &services {
        let svc_id = svc.id.clone();
        let svc_id2 = svc.id.clone();
        let (days, uptime_pct_90d) = tokio::try_join!(
            db.read(move |conn| crate::db::uptime::get_daily_aggregates(conn, &svc_id, 90)),
            db.read(move |conn| crate::db::uptime::compute_uptime_pct(conn, &svc_id2, 90)),
        )?;
        cards.push(ServiceCard {
            service: svc.clone(),
            days,
            uptime_pct_90d,
        });
    }

    let overall_status = if services.iter().any(|s| s.status == "down") {
        "down"
    } else if services.iter().any(|s| s.status == "degraded") {
        "degraded"
    } else {
        "ok"
    }
    .to_string();

    let ok_count = services.iter().filter(|s| s.status == "ok").count();
    let degraded_count = services.iter().filter(|s| s.status == "degraded").count();
    let down_count = services.iter().filter(|s| s.status == "down").count();

    Ok(IndexTemplate {
        site_name: settings.site_name,
        footer_links: settings.footer_links,
        services: cards,
        categories,
        active_incidents,
        recent_resolved,
        upcoming_maintenance,
        overall_status,
        ok_count,
        degraded_count,
        down_count,
    })
}

