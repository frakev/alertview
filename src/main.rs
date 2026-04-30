use alerts::{fetch_source_alerts, severity_order, AlertsResponse, SourceStatus};
use axum::{extract::State, response::Html, routing::get, Json, Router};
use config::{Config, SharedConfig};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::sync::Arc;
use std::time::Duration;

mod alerts;
mod config;

static INDEX_HTML: &str = include_str!("../static/index.html");
static STYLE_CSS: &str  = include_str!("../static/style.css");
static APP_JS: &str     = include_str!("../static/app.js");

struct AppState {
    config: SharedConfig,
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

    let shared_config = Arc::new(tokio::sync::RwLock::new(config));
    let state = Arc::new(AppState {
        config: shared_config.clone(),
        client,
    });

    // Démarrer le watcher de fichier de config
    start_config_watcher(shared_config, config_path);

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
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

async fn serve_css() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "text/css; charset=utf-8")], STYLE_CSS)
}

async fn serve_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript; charset=utf-8")], APP_JS)
}

async fn get_alerts(State(state): State<Arc<AppState>>) -> Json<AlertsResponse> {
    let mut all_alerts = Vec::new();
    let mut source_statuses = Vec::new();

    let config = state.config.read().await;
    for source in &config.sources {
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
        refresh_interval: config.refresh_interval,
        display_labels: config.display.labels.clone(),
    })
}

// Fonction pour watcher les changements du fichier de config
fn start_config_watcher(shared_config: SharedConfig, config_path: String) {
    use std::path::Path;

    let (tx, rx) = std::sync::mpsc::channel();

    // Créer le debouncer avec un délai de 500ms
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)
        .expect("Failed to create config watcher debouncer");

    // Watcher le fichier de config
    debouncer
        .watcher()
        .watch(Path::new(&config_path), RecursiveMode::NonRecursive)
        .expect("Failed to watch config file");

    tracing::info!("Watching config file {} for changes...", config_path);

    // Lancer une tâche tokio pour gérer les événements de changement
    tokio::spawn(async move {
        while let Ok(Ok(events)) = rx.recv() {
            // Le debouncer émet un événement pour toute modification
            if !events.is_empty() {
                tracing::info!("Config file changed, reloading...");
                match Config::load_async(&config_path).await {
                    Ok(new_config) => {
                        let mut config = shared_config.write().await;
                        *config = new_config;
                        tracing::info!(
                            "Config reloaded successfully with {} source(s)",
                            config.sources.len()
                        );
                        for s in &config.sources {
                            tracing::info!("  • {} ({})", s.name, s.url);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to reload config: {}", e);
                    }
                }
            }
        }
    });
}
