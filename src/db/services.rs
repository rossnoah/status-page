use crate::error::AppError;
use crate::models::service::Service;
use rusqlite::{params, Connection};

pub fn list_all(conn: &Connection) -> Result<Vec<Service>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, url, probe_type, probe_config, interval_secs,
                status, uptime_pct, avg_latency_ms, region, is_public, enabled,
                sort_order, created_at, updated_at
         FROM services ORDER BY category, sort_order, name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Service {
            id: row.get(0)?,
            name: row.get(1)?,
            category: row.get(2)?,
            url: row.get(3)?,
            probe_type: row.get(4)?,
            probe_config: row.get(5)?,
            interval_secs: row.get(6)?,
            status: row.get(7)?,
            uptime_pct: row.get(8)?,
            avg_latency_ms: row.get(9)?,
            region: row.get(10)?,
            is_public: row.get(11)?,
            enabled: row.get(12)?,
            sort_order: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn list_public(conn: &Connection) -> Result<Vec<Service>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, url, probe_type, probe_config, interval_secs,
                status, uptime_pct, avg_latency_ms, region, is_public, enabled,
                sort_order, created_at, updated_at
         FROM services WHERE is_public = 1 AND enabled = 1
         ORDER BY category, sort_order, name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Service {
            id: row.get(0)?,
            name: row.get(1)?,
            category: row.get(2)?,
            url: row.get(3)?,
            probe_type: row.get(4)?,
            probe_config: row.get(5)?,
            interval_secs: row.get(6)?,
            status: row.get(7)?,
            uptime_pct: row.get(8)?,
            avg_latency_ms: row.get(9)?,
            region: row.get(10)?,
            is_public: row.get(11)?,
            enabled: row.get(12)?,
            sort_order: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Service>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, category, url, probe_type, probe_config, interval_secs,
                status, uptime_pct, avg_latency_ms, region, is_public, enabled,
                sort_order, created_at, updated_at
         FROM services WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Service {
            id: row.get(0)?,
            name: row.get(1)?,
            category: row.get(2)?,
            url: row.get(3)?,
            probe_type: row.get(4)?,
            probe_config: row.get(5)?,
            interval_secs: row.get(6)?,
            status: row.get(7)?,
            uptime_pct: row.get(8)?,
            avg_latency_ms: row.get(9)?,
            region: row.get(10)?,
            is_public: row.get(11)?,
            enabled: row.get(12)?,
            sort_order: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    })?;
    match rows.next() {
        Some(Ok(s)) => Ok(Some(s)),
        Some(Err(e)) => Err(AppError::from(e)),
        None => Ok(None),
    }
}

pub fn insert(conn: &Connection, svc: &Service) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO services (id, name, category, url, probe_type, probe_config,
         interval_secs, status, region, is_public, enabled, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            svc.id,
            svc.name,
            svc.category,
            svc.url,
            svc.probe_type,
            svc.probe_config,
            svc.interval_secs,
            svc.status,
            svc.region,
            svc.is_public,
            svc.enabled,
            svc.sort_order,
        ],
    )?;
    Ok(())
}

pub fn update(conn: &Connection, svc: &Service) -> Result<(), AppError> {
    conn.execute(
        "UPDATE services SET name=?2, category=?3, url=?4, probe_type=?5,
         probe_config=?6, interval_secs=?7, region=?8, is_public=?9,
         enabled=?10, sort_order=?11, updated_at=datetime('now')
         WHERE id=?1",
        params![
            svc.id,
            svc.name,
            svc.category,
            svc.url,
            svc.probe_type,
            svc.probe_config,
            svc.interval_secs,
            svc.region,
            svc.is_public,
            svc.enabled,
            svc.sort_order,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM services WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn update_status(
    conn: &Connection,
    id: &str,
    status: &str,
    latency_ms: f64,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE services SET status=?2, avg_latency_ms=?3, updated_at=datetime('now') WHERE id=?1",
        params![id, status, latency_ms],
    )?;
    Ok(())
}

pub fn categories(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt =
        conn.prepare("SELECT DISTINCT category FROM services WHERE is_public=1 ORDER BY category")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}
