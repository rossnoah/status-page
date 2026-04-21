use crate::error::AppError;
use crate::models::maintenance::{Maintenance, MaintenanceWithServices};
use rusqlite::{params, Connection};

pub fn list_all(conn: &Connection) -> Result<Vec<MaintenanceWithServices>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, start_time, end_time, impact,
                notes, created_at
         FROM maintenance ORDER BY start_time DESC",
    )?;
    let rows = stmt.query_map([], map_maint)?;
    let items: Vec<Maintenance> = rows.collect::<Result<Vec<_>, _>>()?;
    let mut result = Vec::new();
    for m in items {
        let service_names = list_service_names(conn, &m.id)?;
        result.push(MaintenanceWithServices {
            maintenance: m,
            service_names,
        });
    }
    Ok(result)
}

pub fn list_upcoming(conn: &Connection) -> Result<Vec<MaintenanceWithServices>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, start_time, end_time, impact,
                notes, created_at
         FROM maintenance WHERE status IN ('scheduled', 'in_progress')
         ORDER BY start_time ASC",
    )?;
    let rows = stmt.query_map([], map_maint)?;
    let items: Vec<Maintenance> = rows.collect::<Result<Vec<_>, _>>()?;
    let mut result = Vec::new();
    for m in items {
        let service_names = list_service_names(conn, &m.id)?;
        result.push(MaintenanceWithServices {
            maintenance: m,
            service_names,
        });
    }
    Ok(result)
}

fn map_maint(row: &rusqlite::Row) -> rusqlite::Result<Maintenance> {
    Ok(Maintenance {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        start_time: row.get(3)?,
        end_time: row.get(4)?,
        impact: row.get(5)?,
        notes: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn list_service_names(conn: &Connection, maint_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT s.name FROM maintenance_services ms
         JOIN services s ON s.id = ms.service_id
         WHERE ms.maintenance_id = ?1 ORDER BY s.name",
    )?;
    let rows = stmt.query_map(params![maint_id], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<MaintenanceWithServices>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, start_time, end_time, impact,
                notes, created_at
         FROM maintenance WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], map_maint)?;
    match rows.next() {
        Some(Ok(m)) => {
            let service_names = list_service_names(conn, &m.id)?;
            Ok(Some(MaintenanceWithServices {
                maintenance: m,
                service_names,
            }))
        }
        Some(Err(e)) => Err(AppError::from(e)),
        None => Ok(None),
    }
}

pub fn insert(conn: &Connection, m: &Maintenance, service_ids: &[String]) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO maintenance (id, title, status, start_time, end_time, impact, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            m.id, m.title, m.status, m.start_time, m.end_time, m.impact, m.notes,
        ],
    )?;
    for sid in service_ids {
        conn.execute(
            "INSERT OR IGNORE INTO maintenance_services (maintenance_id, service_id) VALUES (?1, ?2)",
            params![m.id, sid],
        )?;
    }
    Ok(())
}

pub fn update(conn: &Connection, m: &Maintenance, service_ids: &[String]) -> Result<(), AppError> {
    conn.execute(
        "UPDATE maintenance SET title=?2, status=?3, start_time=?4, end_time=?5,
         impact=?6, notes=?7 WHERE id=?1",
        params![
            m.id, m.title, m.status, m.start_time, m.end_time, m.impact, m.notes,
        ],
    )?;
    conn.execute("DELETE FROM maintenance_services WHERE maintenance_id=?1", params![m.id])?;
    for sid in service_ids {
        conn.execute(
            "INSERT OR IGNORE INTO maintenance_services (maintenance_id, service_id) VALUES (?1, ?2)",
            params![m.id, sid],
        )?;
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM maintenance WHERE id = ?1", params![id])?;
    Ok(())
}
