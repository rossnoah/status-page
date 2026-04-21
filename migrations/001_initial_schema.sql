CREATE TABLE IF NOT EXISTS services (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    probe_type TEXT NOT NULL DEFAULT 'http',
    probe_config TEXT NOT NULL DEFAULT '{}',
    interval_secs INTEGER NOT NULL DEFAULT 60,
    status TEXT NOT NULL DEFAULT 'unknown',
    uptime_pct REAL NOT NULL DEFAULT 100.0,
    avg_latency_ms REAL NOT NULL DEFAULT 0.0,
    region TEXT NOT NULL DEFAULT '',
    is_public INTEGER NOT NULL DEFAULT 1,
    enabled INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS incidents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'investigating',
    severity TEXT NOT NULL DEFAULT 'none',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT,
    is_public INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS incident_updates (
    id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    message TEXT NOT NULL,
    created_by TEXT NOT NULL DEFAULT 'admin',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS incident_services (
    incident_id TEXT NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    PRIMARY KEY (incident_id, service_id)
);

CREATE TABLE IF NOT EXISTS maintenance (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'scheduled',
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    impact TEXT NOT NULL DEFAULT 'none',
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS maintenance_services (
    maintenance_id TEXT NOT NULL REFERENCES maintenance(id) ON DELETE CASCADE,
    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    PRIMARY KEY (maintenance_id, service_id)
);

CREATE TABLE IF NOT EXISTS uptime_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    checked_at TEXT NOT NULL DEFAULT (datetime('now')),
    status TEXT NOT NULL,
    latency_ms REAL
);

CREATE TABLE IF NOT EXISTS daily_aggregates (
    service_id TEXT NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    date TEXT NOT NULL,
    total_checks INTEGER NOT NULL DEFAULT 0,
    ok_checks INTEGER NOT NULL DEFAULT 0,
    avg_latency_ms REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (service_id, date)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    token TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_uptime_service_time ON uptime_checks(service_id, checked_at);
CREATE INDEX IF NOT EXISTS idx_incident_updates ON incident_updates(incident_id, created_at);
CREATE INDEX IF NOT EXISTS idx_incidents_status ON incidents(status);
CREATE INDEX IF NOT EXISTS idx_maintenance_status ON maintenance(status);
CREATE INDEX IF NOT EXISTS idx_services_category ON services(category);
