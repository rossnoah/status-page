use crate::error::AppError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FooterLink {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct SiteSettings {
    pub site_name: String,
    pub footer_links: Vec<FooterLink>,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            site_name: "Status Page".into(),
            footer_links: Vec::new(),
        }
    }
}

impl SiteSettings {
    pub fn load(conn: &Connection) -> Result<Self, AppError> {
        let mut s = Self::default();
        if let Some(v) = crate::db::settings::get(conn, "site_name")? {
            s.site_name = v;
        }
        if let Some(json) = crate::db::settings::get(conn, "footer_links")? {
            if let Ok(links) = serde_json::from_str::<Vec<FooterLink>>(&json) {
                s.footer_links = links;
            }
        }
        Ok(s)
    }

    pub fn save(&self, conn: &Connection) -> Result<(), AppError> {
        crate::db::settings::set(conn, "site_name", &self.site_name)?;
        let json = serde_json::to_string(&self.footer_links)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        crate::db::settings::set(conn, "footer_links", &json)?;
        Ok(())
    }
}
