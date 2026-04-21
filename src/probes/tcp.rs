use std::time::{Duration, Instant};
use tokio::net::TcpStream;

pub struct TcpProbeResult {
    pub ok: bool,
    pub latency_ms: f64,
}

pub async fn probe(addr: &str, timeout_secs: u64) -> TcpProbeResult {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    match tokio::time::timeout(timeout, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => {
            let latency = start.elapsed();
            TcpProbeResult {
                ok: true,
                latency_ms: latency.as_secs_f64() * 1000.0,
            }
        }
        Ok(Err(_)) => {
            let latency = start.elapsed();
            TcpProbeResult {
                ok: false,
                latency_ms: latency.as_secs_f64() * 1000.0,
            }
        }
        Err(_) => TcpProbeResult {
            ok: false,
            latency_ms: timeout.as_secs_f64() * 1000.0,
        },
    }
}
