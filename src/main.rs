use alerts::{fetch_source_alerts, severity_order, AlertsResponse, SourceStatus};
use axum::{extract::State, response::Html, routing::get, Json, Router};
use config::Config;
use std::sync::Arc;

mod alerts;
mod config;

static INDEX_HTML: &str = include_str!("../static/index.html");

struct AppState {
    config: Config,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.yaml".to_string());

    let config = Config::load(&config_path)?;
    let port = config.port;

    tracing::info!(
        "Starting AlertView on port {port} with {} source(s)",
        config.sources.len()
    );
    for s in &config.sources {
        tracing::info!("  • {} ({})", s.name, s.url);
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(config.tls_insecure)
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("alertview/0.1")
        .build()?;

    let state = Arc::new(AppState { config, client });

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/alerts", get(get_alerts))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("Listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn get_alerts(State(state): State<Arc<AppState>>) -> Json<AlertsResponse> {
    let mut all_alerts = Vec::new();
    let mut source_statuses = Vec::new();

    for source in &state.config.sources {
        match fetch_source_alerts(&state.client, source).await {
            Ok(mut alerts) => {
                let count = alerts.len();
                tracing::debug!("Fetched {} alerts from {}", count, source.name);
                source_statuses.push(SourceStatus {
                    name: source.name.clone(),
                    status: "ok".to_string(),
                    alert_count: count,
                    error: None,
                });
                all_alerts.append(&mut alerts);
            }
            Err(e) => {
                tracing::warn!("Failed to fetch from {}: {}", source.name, e);
                source_statuses.push(SourceStatus {
                    name: source.name.clone(),
                    status: "error".to_string(),
                    alert_count: 0,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    all_alerts.sort_by(|a, b| {
        severity_order(&a.severity)
            .cmp(&severity_order(&b.severity))
            .then_with(|| b.starts_at.cmp(&a.starts_at))
    });

    Json(AlertsResponse {
        alerts: all_alerts,
        sources: source_statuses,
        refresh_interval: state.config.refresh_interval,
        display_labels: state.config.display.labels.clone(),
    })
}
