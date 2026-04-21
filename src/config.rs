use std::net::SocketAddr;

pub struct Config {
    pub listen: SocketAddr,
    pub database_path: String,
    pub admin_password: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        Self {
            listen: format!("{host}:{port}").parse().unwrap(),
            database_path: std::env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "data/status.db".into()),
            admin_password: std::env::var("ADMIN_PASSWORD").ok(),
        }
    }
}
