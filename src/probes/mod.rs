pub mod http;
pub mod tcp;

use crate::db::DbPool;
use crate::models::service::Service;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

pub enum ProbeCommand {
    Add(Service),
    Remove(String),
    Reload(Service),
}

pub struct ProbeScheduler {
    tx: mpsc::Sender<ProbeCommand>,
}

impl ProbeScheduler {
    pub fn start(db: DbPool) -> Self {
        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(run_scheduler(db, rx));
        Self { tx }
    }

    pub async fn send(&self, cmd: ProbeCommand) {
        let _ = self.tx.send(cmd).await;
    }
}

struct RunningProbe {
    handle: tokio::task::JoinHandle<()>,
}

async fn run_scheduler(db: DbPool, mut rx: mpsc::Receiver<ProbeCommand>) {
    let tasks: Arc<Mutex<HashMap<String, RunningProbe>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let services = db
        .read(|conn| crate::db::services::list_all(conn))
        .await;

    if let Ok(services) = services {
        for svc in services {
            if svc.enabled {
                let handle = spawn_probe(db.clone(), svc.clone());
                tasks
                    .lock()
                    .await
                    .insert(svc.id.clone(), RunningProbe { handle });
            }
        }
    }

    while let Some(cmd) = rx.recv().await {
        match cmd {
            ProbeCommand::Add(svc) => {
                let handle = spawn_probe(db.clone(), svc.clone());
                tasks
                    .lock()
                    .await
                    .insert(svc.id.clone(), RunningProbe { handle });
            }
            ProbeCommand::Remove(id) => {
                if let Some(probe) = tasks.lock().await.remove(&id) {
                    probe.handle.abort();
                }
            }
            ProbeCommand::Reload(svc) => {
                if let Some(probe) = tasks.lock().await.remove(&svc.id) {
                    probe.handle.abort();
                }
                if svc.enabled {
                    let handle = spawn_probe(db.clone(), svc.clone());
                    tasks
                        .lock()
                        .await
                        .insert(svc.id.clone(), RunningProbe { handle });
                }
            }
        }
    }
}

fn spawn_probe(db: DbPool, svc: Service) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval = std::time::Duration::from_secs(svc.interval_secs.max(10) as u64);

        loop {
            let (ok, latency_ms) = run_probe(&svc).await;
            let status = if ok { "ok" } else { "down" };

            let svc_id = svc.id.clone();
            let status_str = status.to_string();
            let lat = latency_ms;

            let _ = db
                .write(move |conn| {
                    crate::db::uptime::record_check(conn, &svc_id, &status_str, lat)?;
                    crate::db::services::update_status(conn, &svc_id, &status_str, lat)?;
                    Ok(())
                })
                .await;

            tokio::time::sleep(interval).await;
        }
    })
}

async fn run_probe(svc: &Service) -> (bool, f64) {
    let config: serde_json::Value =
        serde_json::from_str(&svc.probe_config).unwrap_or(serde_json::Value::Object(Default::default()));

    let timeout = config
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(10);

    match svc.probe_type.as_str() {
        "tcp" => {
            let addr = config
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or(&svc.url);
            let result = tcp::probe(addr, timeout).await;
            (result.ok, result.latency_ms)
        }
        _ => {
            let expected = config
                .get("expected_status")
                .and_then(|v| v.as_u64())
                .unwrap_or(200) as u16;
            let result = http::probe(&svc.url, timeout, expected).await;
            (result.ok, result.latency_ms)
        }
    }
}
