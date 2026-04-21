use std::time::{Duration, Instant};

pub struct HttpProbeResult {
    pub ok: bool,
    pub latency_ms: f64,
}

pub async fn probe(url: &str, timeout_secs: u64, expected_status: u16) -> HttpProbeResult {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(false)
        .build();

    let client = match client {
        Ok(c) => c,
        Err(_) => {
            return HttpProbeResult {
                ok: false,
                latency_ms: 0.0,
            };
        }
    };

    let start = Instant::now();
    match client.get(url).send().await {
        Ok(resp) => {
            let latency = start.elapsed();
            let status = resp.status().as_u16();
            HttpProbeResult {
                ok: status == expected_status,
                latency_ms: latency.as_secs_f64() * 1000.0,
            }
        }
        Err(_) => {
            let latency = start.elapsed();
            HttpProbeResult {
                ok: false,
                latency_ms: latency.as_secs_f64() * 1000.0,
            }
        }
    }
}
