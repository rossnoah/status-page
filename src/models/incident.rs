use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: String,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub started_at: String,
    pub resolved_at: Option<String>,
    pub is_public: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentUpdate {
    pub id: String,
    pub incident_id: String,
    pub status: String,
    pub message: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentWithUpdates {
    pub incident: Incident,
    pub updates: Vec<IncidentUpdate>,
    pub service_names: Vec<String>,
}

impl Incident {
    pub fn severity_class(&self) -> &str {
        match self.severity.as_str() {
            "degraded" => "warn",
            "down" => "bad",
            _ => "maint",
        }
    }

    pub fn status_label(&self) -> &str {
        match self.status.as_str() {
            "investigating" => "investigating",
            "identified" => "identified",
            "monitoring" => "monitoring",
            "resolved" => "resolved",
            _ => &self.status,
        }
    }
}
