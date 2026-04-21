pub mod incidents;
pub mod maintenance;
pub mod services;
pub mod settings;
pub mod uptime;

use crate::error::AppError;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DbPool {
    pub read: Pool<SqliteConnectionManager>,
    pub write: Arc<Mutex<Connection>>,
}

impl DbPool {
    pub fn new(path: &str) -> Result<Self, AppError> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let write_conn = Connection::open(path)?;
        write_conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA foreign_keys=ON;
             PRAGMA synchronous=NORMAL;",
        )?;

        run_migrations(&write_conn)?;

        let manager = SqliteConnectionManager::file(path);
        let read_pool = Pool::builder().max_size(8).build(manager)?;

        {
            let conn = read_pool.get()?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA foreign_keys=ON;",
            )?;
        }

        Ok(Self {
            read: read_pool,
            write: Arc::new(Mutex::new(write_conn)),
        })
    }

    pub async fn read<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.read.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            f(&conn)
        })
        .await?
    }

    pub async fn write<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let write = self.write.clone();
        tokio::task::spawn_blocking(move || {
            let conn = write.blocking_lock();
            f(&conn)
        })
        .await?
    }
}

fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(include_str!("../../migrations/001_initial_schema.sql"))?;
    Ok(())
}
