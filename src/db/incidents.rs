use crate::error::AppError;
use crate::models::incident::{Incident, IncidentUpdate, IncidentWithUpdates};
use rusqlite::{params, Connection};

pub fn list_all(conn: &Connection) -> Result<Vec<IncidentWithUpdates>, AppError> {
    let incidents = list_incidents(conn, None)?;
    let mut result = Vec::new();
    for inc in incidents {
        let updates = list_updates(conn, &inc.id)?;
        let service_names = list_service_names(conn, &inc.id)?;
        result.push(IncidentWithUpdates {
            incident: inc,
            updates,
            service_names,
        });
    }
    Ok(result)
}

pub fn list_active(conn: &Connection) -> Result<Vec<IncidentWithUpdates>, AppError> {
    let incidents = list_incidents(conn, Some("status != 'resolved'"))?;
    let mut result = Vec::new();
    for inc in incidents {
        let updates = list_updates(conn, &inc.id)?;
        let service_names = list_service_names(conn, &inc.id)?;
        result.push(IncidentWithUpdates {
            incident: inc,
            updates,
            service_names,
        });
    }
    Ok(result)
}

pub fn list_recent_resolved(conn: &Connection, limit: usize) -> Result<Vec<IncidentWithUpdates>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, severity, started_at, resolved_at, is_public, created_at
         FROM incidents WHERE status = 'resolved' ORDER BY resolved_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], map_incident)?;
    let incidents: Vec<Incident> = rows.collect::<Result<Vec<_>, _>>()?;
    let mut result = Vec::new();
    for inc in incidents {
        let service_names = list_service_names(conn, &inc.id)?;
        result.push(IncidentWithUpdates {
            updates: Vec::new(),
            incident: inc,
            service_names,
        });
    }
    Ok(result)
}

fn list_incidents(conn: &Connection, filter: Option<&str>) -> Result<Vec<Incident>, AppError> {
    let query = if let Some(f) = filter {
        format!(
            "SELECT id, title, status, severity, started_at, resolved_at, is_public, created_at
             FROM incidents WHERE {f} ORDER BY started_at DESC"
        )
    } else {
        "SELECT id, title, status, severity, started_at, resolved_at, is_public, created_at
         FROM incidents ORDER BY started_at DESC"
            .to_string()
    };
    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map([], map_incident)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

fn map_incident(row: &rusqlite::Row) -> rusqlite::Result<Incident> {
    Ok(Incident {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        severity: row.get(3)?,
        started_at: row.get(4)?,
        resolved_at: row.get(5)?,
        is_public: row.get(6)?,
        created_at: row.get(7)?,
    })
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<IncidentWithUpdates>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, severity, started_at, resolved_at, is_public, created_at
         FROM incidents WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], map_incident)?;
    match rows.next() {
        Some(Ok(inc)) => {
            let updates = list_updates(conn, &inc.id)?;
            let service_names = list_service_names(conn, &inc.id)?;
            Ok(Some(IncidentWithUpdates {
                incident: inc,
                updates,
                service_names,
            }))
        }
        Some(Err(e)) => Err(AppError::from(e)),
        None => Ok(None),
    }
}

fn list_updates(conn: &Connection, incident_id: &str) -> Result<Vec<IncidentUpdate>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, incident_id, status, message, created_by, created_at
         FROM incident_updates WHERE incident_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![incident_id], |row| {
        Ok(IncidentUpdate {
            id: row.get(0)?,
            incident_id: row.get(1)?,
            status: row.get(2)?,
            message: row.get(3)?,
            created_by: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

fn list_service_names(conn: &Connection, incident_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT s.name FROM incident_services iss
         JOIN services s ON s.id = iss.service_id
         WHERE iss.incident_id = ?1 ORDER BY s.name",
    )?;
    let rows = stmt.query_map(params![incident_id], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn insert(conn: &Connection, inc: &Incident, service_ids: &[String]) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO incidents (id, title, status, severity, started_at, is_public)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![inc.id, inc.title, inc.status, inc.severity, inc.started_at, inc.is_public],
    )?;
    for sid in service_ids {
        conn.execute(
            "INSERT OR IGNORE INTO incident_services (incident_id, service_id) VALUES (?1, ?2)",
            params![inc.id, sid],
        )?;
    }
    Ok(())
}

pub fn add_update(conn: &Connection, update: &IncidentUpdate) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO incident_updates (id, incident_id, status, message, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![update.id, update.incident_id, update.status, update.message, update.created_by],
    )?;
    conn.execute(
        "UPDATE incidents SET status = ?2 WHERE id = ?1",
        params![update.incident_id, update.status],
    )?;
    if update.status == "resolved" {
        conn.execute(
            "UPDATE incidents SET resolved_at = datetime('now') WHERE id = ?1",
            params![update.incident_id],
        )?;
    }
    Ok(())
}

pub fn update_incident(conn: &Connection, inc: &Incident, service_ids: &[String]) -> Result<(), AppError> {
    conn.execute(
        "UPDATE incidents SET title=?2, status=?3, severity=?4, is_public=?5 WHERE id=?1",
        params![inc.id, inc.title, inc.status, inc.severity, inc.is_public],
    )?;
    conn.execute("DELETE FROM incident_services WHERE incident_id=?1", params![inc.id])?;
    for sid in service_ids {
        conn.execute(
            "INSERT OR IGNORE INTO incident_services (incident_id, service_id) VALUES (?1, ?2)",
            params![inc.id, sid],
        )?;
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM incidents WHERE id = ?1", params![id])?;
    Ok(())
}
