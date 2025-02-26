use anyhow::Result;
use axum::Router;
use axum::{response::IntoResponse, routing::get};
use std::future::ready;
use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

pub async fn start_metrics_server(bind_addr: SocketAddr) -> Result<()> {
    let recorder_handle = setup_metrics_recorder().expect("Failed to setup prometheus metrics");
    let app = Router::new().route("/metrics", get(move || ready(recorder_handle.render())));
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!("Prometheus listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

fn setup_metrics_recorder() -> anyhow::Result<PrometheusHandle> {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    Ok(PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_requests_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )?
        .install_recorder()?)
}

pub async fn track_metrics(req: Request, next: Next) -> impl IntoResponse {
    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };
    let method = req.method().clone();

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [
        ("method", method.to_string()),
        ("path", path),
        ("status", status),
    ];

    metrics::counter!("http_requests_total", &labels).increment(1);
    metrics::histogram!("http_requests_duration_seconds", &labels).record(latency);

    response
}
