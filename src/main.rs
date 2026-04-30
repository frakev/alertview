use alerts::{severity_order, AlertsResponse, SourceStatus};
use axum::{extract::State, response::Html, routing::get, Json, Router};
use config::{Config, SharedConfig};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::compression::CompressionLayer;

pub mod alerts;
pub mod config;

static INDEX_HTML: &str = include_str!("../static/index.html");
static STYLE_CSS: &str  = include_str!("../static/style.css");
static APP_JS: &str     = include_str!("../static/app.js");

struct AppState {
    config: SharedConfig,
    client: reqwest::Client,
    cache: Arc<tokio::sync::RwLock<HashMap<String, (Vec<alerts::Alert>, Instant)>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.yaml".to_string());

    let config = Config::load(&config_path)?;
    let port = config.port;

    // Configure logging format from config or env
    let use_json_logs = config.log_format == "json";
    
    if use_json_logs {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }

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
    let cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
    let state = Arc::new(AppState {
        config: shared_config.clone(),
        client,
        cache,
    });

    // Démarrer le watcher de fichier de config
    start_config_watcher(shared_config, config_path);

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/api/alerts", get(get_alerts))
        .route("/health", get(health_check))
        .layer(CompressionLayer::new())
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

async fn health_check() -> &'static str {
    "OK"
}

async fn get_alerts(State(state): State<Arc<AppState>>) -> Json<AlertsResponse> {
    let mut all_alerts = Vec::new();
    let mut source_statuses = Vec::new();

    let config = state.config.read().await;
    let cache_ttl = Duration::from_secs(config.cache_ttl_seconds);
    let use_cache = cache_ttl > Duration::from_secs(0);

    for source in &config.sources {
        // Generate cache key based on source config
        let cache_key = format!("{}:{}:{:?}", source.name, source.url, source.source_type);
        
        // Try to get from cache
        let cached_data = if use_cache {
            let cache = state.cache.read().await;
            cache.get(&cache_key).and_then(|(alerts, timestamp)| {
                if timestamp.elapsed() < cache_ttl {
                    Some(alerts.clone())
                } else {
                    None
                }
            })
        } else {
            None
        };

        match cached_data {
            Some(alerts) => {
                tracing::debug!("Cache hit for source {}", source.name);
                source_statuses.push(SourceStatus {
                    name: source.name.clone(),
                    status: "ok".to_string(),
                    alert_count: alerts.len(),
                    error: None,
                });
                all_alerts.extend(alerts);
                continue;
            }
            None => {
                tracing::debug!("Fetching from source {}", source.name);
            }
        }

        match fetch_source_alerts_with_retry(&state.client, source).await {
            Ok(mut alerts) => {
                let count = alerts.len();
                tracing::debug!("Fetched {} alerts from {}", count, source.name);
                
                // Cache the results if caching is enabled
                if use_cache {
                    let mut cache = state.cache.write().await;
                    cache.insert(cache_key, (alerts.clone(), Instant::now()));
                }
                
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
        timezone: Some(config.display.timezone.clone()),
        theme: config.display.theme.clone(),
        play_sounds: config.display.play_sounds,
    })
}

// Fonction pour fetch avec retry et timeout par source
async fn fetch_source_alerts_with_retry(
    client: &reqwest::Client,
    source: &config::Source,
) -> Result<Vec<alerts::Alert>, anyhow::Error> {
    let max_retries = source.retry_policy.max_retries;
    let mut last_error: Option<anyhow::Error> = None;
    
    for attempt in 0..=max_retries {
        let timeout = Duration::from_secs(source.timeout);
        
        // Calculer le délai avant la tentative (exponentiel)
        if attempt > 0 {
            let delay_ms = source.retry_policy.initial_delay_ms * (1 << (attempt - 1)).min(source.retry_policy.max_delay_ms);
            tracing::warn!(
                "Retry attempt {}/{} for {} after {}ms delay (error: {})",
                attempt,
                max_retries,
                source.name,
                delay_ms,
                last_error.as_ref().map(|e| e.to_string()).unwrap_or_default()
            );
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        
        let result = tokio::time::timeout(
            timeout,
            alerts::fetch_source_alerts(client, source)
        ).await;
        
        match result {
            Ok(Ok(alerts)) => return Ok(alerts),
            Ok(Err(e)) => {
                last_error = Some(e);
                // Pour les erreurs HTTP 4xx, on ne retry pas
                if let Some(status_code) = last_error.as_ref().and_then(|e| {
                    e.to_string().split_whitespace().next()
                        .and_then(|s| s.parse::<u16>().ok())
                }) {
                    if (400..500).contains(&status_code) {
                        return Err(last_error.unwrap());
                    }
                }
            }
            Err(_) => {
                last_error = Some(anyhow::anyhow!("Timeout after {}s", source.timeout));
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
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
