use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyAggregate {
    pub service_id: String,
    pub date: String,
    pub total_checks: i64,
    pub ok_checks: i64,
    pub avg_latency_ms: f64,
}

impl DailyAggregate {
    pub fn status(&self) -> &str {
        if self.total_checks == 0 {
            return "nodata";
        }
        let pct = self.ok_checks as f64 / self.total_checks as f64;
        if pct >= 0.99 {
            "ok"
        } else if pct >= 0.90 {
            "degraded"
        } else {
            "down"
        }
    }

    pub fn status_class(&self) -> &str {
        match self.status() {
            "ok" => "ok",
            "degraded" => "warn",
            "down" => "bad",
            _ => "nodata",
        }
    }

    pub fn latency_fmt(&self) -> String {
        format!("{:.0}", self.avg_latency_ms)
    }
}
