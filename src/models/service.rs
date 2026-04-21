use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub category: String,
    pub url: String,
    pub probe_type: String,
    pub probe_config: String,
    pub interval_secs: i64,
    pub status: String,
    pub uptime_pct: f64,
    pub avg_latency_ms: f64,
    pub region: String,
    pub is_public: bool,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Service {
    pub fn status_label(&self) -> &str {
        match self.status.as_str() {
            "ok" => "operational",
            "degraded" => "degraded",
            "down" => "down",
            "paused" => "paused",
            _ => "unknown",
        }
    }

    pub fn status_class(&self) -> &str {
        match self.status.as_str() {
            "ok" => "ok",
            "degraded" => "warn",
            "down" => "bad",
            "paused" => "maint",
            _ => "maint",
        }
    }

    pub fn uptime_fmt(&self) -> String {
        format!("{:.2}", self.uptime_pct)
    }

    pub fn latency_fmt(&self) -> String {
        format!("{:.0}", self.avg_latency_ms)
    }

    pub fn category_lower(&self) -> String {
        self.category.to_lowercase()
    }
}
