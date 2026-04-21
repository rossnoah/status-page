use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maintenance {
    pub id: String,
    pub title: String,
    pub status: String,
    pub start_time: String,
    pub end_time: String,
    pub impact: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceWithServices {
    pub maintenance: Maintenance,
    pub service_names: Vec<String>,
}

impl Maintenance {
    pub fn impact_class(&self) -> &str {
        match self.impact.as_str() {
            "degraded" => "warn",
            "down" => "bad",
            _ => "maint",
        }
    }
}
