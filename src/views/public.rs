use std::collections::HashMap;

use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;
use chrono::Utc;

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
}

/// Expand a sparse list of daily aggregates into a full 90-day vector where
/// each slot corresponds to its actual date. The last element is always today;
/// dates with no data are filled with a no-data placeholder.
fn expand_days(service_id: &str, sparse: Vec<DailyAggregate>, num_days: usize) -> Vec<DailyAggregate> {
    let today = Utc::now().date_naive();
    let by_date: HashMap<String, DailyAggregate> = sparse
        .into_iter()
        .map(|d| (d.date.clone(), d))
        .collect();

    (0..num_days)
        .map(|i| {
            let date = today - chrono::Duration::days((num_days - 1 - i) as i64);
            let date_str = date.format("%Y-%m-%d").to_string();
            by_date
                .get(&date_str)
                .cloned()
                .unwrap_or_else(|| DailyAggregate::no_data(service_id, &date_str))
        })
        .collect()
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
            days: expand_days(&svc.id, days, 90),
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

