use crate::error::AppError;
use crate::models::uptime::DailyAggregate;
use rusqlite::{params, Connection};

pub fn record_check(
    conn: &Connection,
    service_id: &str,
    status: &str,
    latency_ms: f64,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO uptime_checks (service_id, status, latency_ms) VALUES (?1, ?2, ?3)",
        params![service_id, status, latency_ms],
    )?;

    let ok_val: i64 = if status == "ok" { 1 } else { 0 };
    conn.execute(
        "INSERT INTO daily_aggregates (service_id, date, total_checks, ok_checks, avg_latency_ms)
         VALUES (?1, date('now'), 1, ?2, ?3)
         ON CONFLICT(service_id, date) DO UPDATE SET
             total_checks = total_checks + 1,
             ok_checks = ok_checks + ?2,
             avg_latency_ms = (avg_latency_ms * total_checks + ?3) / (total_checks + 1)",
        params![service_id, ok_val, latency_ms],
    )?;
    Ok(())
}

pub fn get_daily_aggregates(
    conn: &Connection,
    service_id: &str,
    days: i64,
) -> Result<Vec<DailyAggregate>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT service_id, date, total_checks, ok_checks, avg_latency_ms
         FROM daily_aggregates
         WHERE service_id = ?1 AND date > date('now', ?2)
         ORDER BY date ASC",
    )?;
    let offset = format!("-{days} days");
    let rows = stmt.query_map(params![service_id, offset], |row| {
        Ok(DailyAggregate {
            service_id: row.get(0)?,
            date: row.get(1)?,
            total_checks: row.get(2)?,
            ok_checks: row.get(3)?,
            avg_latency_ms: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn compute_uptime_pct(conn: &Connection, service_id: &str, days: i64) -> Result<f64, AppError> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(
            CAST(SUM(ok_checks) AS REAL) / NULLIF(CAST(SUM(total_checks) AS REAL), 0) * 100.0,
            100.0
         )
         FROM daily_aggregates
         WHERE service_id = ?1 AND date > date('now', ?2)",
    )?;
    let offset = format!("-{days} days");
    let pct: f64 = stmt.query_row(params![service_id, offset], |row| row.get(0))?;
    Ok(pct)
}

pub fn cleanup_old_checks(conn: &Connection, days: i64) -> Result<usize, AppError> {
    let offset = format!("-{days} days");
    let deleted = conn.execute(
        "DELETE FROM uptime_checks WHERE checked_at < datetime('now', ?1)",
        params![offset],
    )?;
    Ok(deleted)
}
