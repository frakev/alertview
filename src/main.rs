use alerts::{severity_order, Alert, AlertsResponse, SourceStatus};
use axum::{extract::State, response::Html, routing::get, Json, Router};
use config::{Config, SharedConfig};
use futures::stream;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::compression::CompressionLayer;

pub mod alerts;
pub mod config;

static INDEX_HTML: &str = include_str!("../static/index.html");
static STYLE_CSS: &str  = include_str!("../static/style.css");
static APP_JS: &str     = include_str!("../static/app.js");

fn print_help() {
    println!("AlertView - Alert Aggregation Dashboard");
    println!();
    println!("Usage:");
    println!("  alertview [OPTIONS] [CONFIG_FILE]");
    println!();
    println!("Arguments:");
    println!("  CONFIG_FILE    Path to the configuration file (default: config.yaml)");
    println!();
    println!("Options:");
    println!("  -h, --help     Show this help message and exit");
    println!();
    println!("Environment Variables:");
    println!("  ALERTVIEW_CONFIG    Path to the configuration file");
    println!("  ALERTVIEW_PORT      Port to listen on (default: 8080)");
    println!("  ALERTVIEW_LOG_FORMAT Log format: 'text' or 'json' (default: text)");
    println!("  RUST_LOG           Log level: error, warn, info, debug, trace");
    println!();
    println!("Examples:");
    println!("  alertview                          # Use default config.yaml");
    println!("  alertview /etc/alertview/config.yaml");
    println!("  alertview --help");
}

// Type aliases for cleaner code
type AlertCache = HashMap<String, (Vec<alerts::Alert>, Instant)>;
type SharedAlertCache = Arc<tokio::sync::RwLock<AlertCache>>;

// Maximum cache size to prevent memory exhaustion
const MAX_CACHE_ENTRIES: usize = 1000;

// Maximum number of concurrent SSE connections
const MAX_SSE_CONNECTIONS: usize = 100;

struct AppState {
    config: SharedConfig,
    client: reqwest::Client,
    cache: SharedAlertCache,
    tx: broadcast::Sender<Alert>,
    sse_connections: Arc<tokio::sync::RwLock<usize>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    let config_path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "config.yaml".to_string());

    let config = Config::load(&config_path)?;
    let port = config.port;
    
    // Extract config watch settings before moving config
    let watch_method = config.config_watch_method.clone();
    let poll_interval = config.config_poll_interval;

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

    // Warn if TLS verification is disabled
    if config.tls_insecure {
        tracing::warn!("TLS certificate verification is DISABLED - this is insecure for production!");
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(config.tls_insecure)
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("alertview/0.1")
        .build()?;

    let shared_config = Arc::new(tokio::sync::RwLock::new(config));
    let cache: SharedAlertCache = Arc::new(tokio::sync::RwLock::new(AlertCache::new()));
    
    // Create broadcast channel for WebSocket notifications
    let (tx, _rx) = broadcast::channel::<Alert>(100);
    
    let state = Arc::new(AppState {
        config: shared_config.clone(),
        client,
        cache,
        tx: tx.clone(),
        sse_connections: Arc::new(tokio::sync::RwLock::new(0)),
    });

    // Start config file watcher
    start_config_watcher(shared_config.clone(), config_path, watch_method, poll_interval);

    // Log cache size limit
    tracing::debug!("Alert cache limited to {} entries", MAX_CACHE_ENTRIES);

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/api/alerts", get(get_alerts))
        .route("/health", get(health_check))
        .route("/events", get(sse_handler))
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

// Server-Sent Events handler for real-time alert notifications
async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Result<axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, axum::http::StatusCode> {
    use axum::response::sse::{Event, KeepAlive};
    use futures::stream::StreamExt as _;
    
    // Check connection limit
    {
        let connections = state.sse_connections.read().await;
        if *connections >= MAX_SSE_CONNECTIONS {
            tracing::warn!("SSE connection limit reached ({}/{})", *connections, MAX_SSE_CONNECTIONS);
            return Err(axum::http::StatusCode::TOO_MANY_REQUESTS);
        }
    }
    
    // Increment connection counter
    {
        let mut connections = state.sse_connections.write().await;
        *connections += 1;
        tracing::debug!("SSE connection opened (total: {})", *connections);
    }
    
    let rx = state.tx.subscribe();
    
    // Create a stream of SSE events from the broadcast channel
    let event_stream = stream::unfold(rx, move |mut rx| {
        let state_clone = state.clone();
        async move {
            match rx.recv().await {
                Ok(alert) => {
                    let json = serde_json::to_string(&alert).unwrap_or_default();
                    let event = Event::default()
                        .event("new_alert")
                        .data(json);
                    Some((event, rx))
                }
                Err(_) => {
                    // Decrement connection counter on error
                    let mut connections = state_clone.sse_connections.write().await;
                    *connections = connections.saturating_sub(1);
                    tracing::debug!("SSE connection closed (total: {})", *connections);
                    None // Channel closed
                }
            }
        }
    });
    
    // Convert to Result stream (SSE requires Result)
    let event_stream = event_stream.map(Ok);
    
    Ok(axum::response::Sse::new(event_stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30))))
}

async fn get_alerts(State(state): State<Arc<AppState>>) -> Json<AlertsResponse> {
    let mut all_alerts = Vec::new();
    let mut source_statuses = Vec::new();
    let mut new_alerts = Vec::new();

    let config = state.config.read().await;
    let cache_ttl = Duration::from_secs(config.cache_ttl_seconds);
    let use_cache = cache_ttl > Duration::from_secs(0);
    
    // Get previous fingerprints to detect new alerts
    let prev_fingerprints: HashSet<String> = {
        let cache = state.cache.read().await;
        cache.iter()
            .flat_map(|(_, (alerts, _))| alerts.iter().map(|a| a.fingerprint.clone()))
            .collect()
    };

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
                
                // Detect new alerts and broadcast them
                for alert in &alerts {
                    if !prev_fingerprints.contains(&alert.fingerprint) {
                        new_alerts.push(alert.clone());
                    }
                }
                
                // Cache the results if caching is enabled
                if use_cache {
                    let mut cache = state.cache.write().await;
                    cache.insert(cache_key, (alerts.clone(), Instant::now()));
                    
                    // Limit cache size to prevent memory exhaustion
                    if cache.len() > MAX_CACHE_ENTRIES {
                        // Remove oldest entries (simple strategy: remove first N)
                        let keys_to_remove: Vec<String> = cache.keys().take(cache.len() - MAX_CACHE_ENTRIES).cloned().collect();
                        let count = keys_to_remove.len();
                        for key in &keys_to_remove {
                            cache.remove(key);
                        }
                        tracing::warn!("Cache size limit reached, removed {} entries", count);
                    }
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

    // Broadcast new alerts to WebSocket clients
    for alert in new_alerts {
        let _ = state.tx.send(alert);
    }

    all_alerts.sort_by(|a, b| {
        severity_order(&a.severity)
            .cmp(&severity_order(&b.severity))
            .then_with(|| b.starts_at.cmp(&a.starts_at))
    });

    // Group alerts if group_by is configured
    let groups = if !config.display.group_by.is_empty() {
        alerts::group_alerts(&all_alerts, &config.display.group_by)
    } else {
        vec![]
    };

    Json(AlertsResponse {
        alerts: all_alerts,
        sources: source_statuses,
        refresh_interval: config.refresh_interval,
        display_labels: config.display.labels.clone(),
        timezone: Some(config.display.timezone.clone()),
        theme: config.display.theme.clone(),
        play_sounds: config.display.play_sounds,
        groups,
        group_by: config.display.group_by.clone(),
    })
}

// Fetch alerts from a source with retry logic and per-source timeout
async fn fetch_source_alerts_with_retry(
    client: &reqwest::Client,
    source: &config::Source,
) -> Result<Vec<alerts::Alert>, anyhow::Error> {
    let max_retries = source.retry_policy.max_retries;
    let mut last_error: Option<anyhow::Error> = None;
    
    for attempt in 0..=max_retries {
        let timeout = Duration::from_secs(source.timeout);
        
        // Calculate exponential backoff delay
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
                // Don't retry on HTTP 4xx client errors
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
    
    // Return the last error, or a generic error if none (shouldn't happen)
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error after {} retries", max_retries)))
}

// Function to watch config file for changes (using inotify or polling)
fn start_config_watcher(shared_config: SharedConfig, config_path: String, watch_method: String, poll_interval_secs: u64) {
    use std::path::Path;
    use std::{fs, time::SystemTime};

    let poll_interval = Duration::from_secs(poll_interval_secs);

    // Check if config file exists before watching
    if !Path::new(&config_path).exists() {
        tracing::error!("Config file {} does not exist. Auto-reload is DISABLED.", config_path);
        tracing::error!("Changes to config file will not be detected.");
        return;
    }

    let config_path_for_task = config_path.clone();

    if watch_method == "polling" {
        tracing::info!("Using polling method to watch config file {} (interval: {}s)...", 
                      config_path, poll_interval_secs);
        
        // Use polling method
        tokio::spawn(async move {
            let mut last_modified: Option<SystemTime> = None;
            
            // Get initial modification time
            if let Ok(metadata) = fs::metadata(&config_path_for_task) {
                last_modified = Some(metadata.modified().ok().unwrap_or(SystemTime::now()));
            }
            
            let shared_config_clone = shared_config.clone();
            let config_path_clone = config_path_for_task.clone();
            
            loop {
                tokio::time::sleep(poll_interval).await;
                
                // Check if file was modified
                match fs::metadata(&config_path_clone) {
                    Ok(metadata) => {
                        if let Ok(modified) = metadata.modified() {
                            if last_modified.as_ref().is_none_or(|&last| modified > last) {
                                // File was modified
                                tracing::info!("Config file {} modified, reloading...", config_path_clone);
                                last_modified = Some(modified);
                                
                                match Config::load_async(&config_path_clone).await {
                                    Ok(new_config) => {
                                        let mut cfg = shared_config_clone.write().await;
                                        *cfg = new_config;
                                        tracing::info!(
                                            "Config reloaded successfully with {} source(s)",
                                            cfg.sources.len()
                                        );
                                        for s in &cfg.sources {
                                            tracing::info!("  • {} ({})", s.name, s.url);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to reload config: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Could not read config file metadata: {}", e);
                    }
                }
            }
        });
    } else {
        // Use inotify method (default)
        let (tx, rx) = std::sync::mpsc::channel();

        // Create debouncer with 500ms delay
        let mut debouncer = match new_debouncer(Duration::from_millis(500), tx) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to create config watcher debouncer: {}", e);
                tracing::error!("Falling back to polling method...");
                // Fallback to polling
                tracing::info!("Using polling method to watch config file {} (interval: {}s)...", 
                              config_path, poll_interval_secs);
                start_polling_watcher(shared_config, config_path, poll_interval_secs);
                return;
            }
        };

        // Watch config file
        let config_path_clone = config_path.clone();
        match debouncer
            .watcher()
            .watch(Path::new(&config_path), RecursiveMode::NonRecursive)
        {
            Ok(_) => {
                tracing::info!("Watching config file {} for changes using inotify...", config_path);
            }
            Err(e) => {
                tracing::error!("Failed to watch config file {}: {}", config_path_clone, e);
                tracing::error!("Falling back to polling method...");
                // Fallback to polling
                tracing::info!("Using polling method to watch config file {} (interval: {}s)...", 
                              config_path, poll_interval_secs);
                start_polling_watcher(shared_config, config_path, poll_interval_secs);
                return;
            }
        }

        // Spawn task to handle change events
        tokio::spawn(async move {
            while let Ok(Ok(events)) = rx.recv() {
                // Debouncer emits event for any modification
                tracing::info!("Detected {} file change event(s) for {}, reloading...", events.len(), config_path_for_task);
                match Config::load_async(&config_path_for_task).await {
                    Ok(new_config) => {
                        let mut cfg = shared_config.write().await;
                        *cfg = new_config;
                        tracing::info!(
                            "Config reloaded successfully with {} source(s)",
                            cfg.sources.len()
                        );
                        for s in &cfg.sources {
                            tracing::info!("  • {} ({})", s.name, s.url);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to reload config: {}", e);
                    }
                }
            }
        });
    }
}

// Fallback polling watcher function
fn start_polling_watcher(shared_config: SharedConfig, config_path: String, poll_interval_secs: u64) {
    use std::fs;
    use std::time::SystemTime;
    use std::time::Duration;

    let poll_interval = Duration::from_secs(poll_interval_secs);

    tokio::spawn(async move {
        let mut last_modified: Option<SystemTime> = None;
        
        // Get initial modification time
        if let Ok(metadata) = fs::metadata(&config_path) {
            last_modified = Some(metadata.modified().ok().unwrap_or(SystemTime::now()));
        }
        
        let shared_config_clone = shared_config.clone();
        let config_path_clone = config_path.clone();
        
        loop {
            tokio::time::sleep(poll_interval).await;
            
            // Check if file was modified
            match fs::metadata(&config_path_clone) {
                Ok(metadata) => {
                    if let Ok(modified) = metadata.modified() {
                        if last_modified.as_ref().is_none_or(|&last| modified > last) {
                            // File was modified
                            tracing::info!("Config file {} modified, reloading...", config_path_clone);
                            last_modified = Some(modified);
                            
                            match Config::load_async(&config_path_clone).await {
                                Ok(new_config) => {
                                    let mut cfg = shared_config_clone.write().await;
                                    *cfg = new_config;
                                    tracing::info!(
                                        "Config reloaded successfully with {} source(s)",
                                        cfg.sources.len()
                                    );
                                    for s in &cfg.sources {
                                        tracing::info!("  • {} ({})", s.name, s.url);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to reload config: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Could not read config file metadata: {}", e);
                }
            }
        }
    });
}
