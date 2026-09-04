use alerts::{severity_rank, Alert, AlertsResponse, SourceStatus};
use axum::{extract::State, response::Html, routing::get, Json, Router};
use config::{Config, SharedConfig};
use futures::stream;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::compression::CompressionLayer;

pub mod alerts;
pub mod config;

static INDEX_HTML: &str = include_str!("../static/index.html");
static STYLE_CSS: &str  = include_str!("../static/style.css");
static APP_JS: &str     = include_str!("../static/app.js");

// PWA assets (manifest, service worker, icons) embedded in the binary.
static MANIFEST: &str = include_str!("../static/manifest.webmanifest");
static SW_JS: &str    = include_str!("../static/sw.js");
static ICON_192: &[u8]          = include_bytes!("../static/icons/icon-192.png");
static ICON_512: &[u8]          = include_bytes!("../static/icons/icon-512.png");
static ICON_MASKABLE_512: &[u8] = include_bytes!("../static/icons/icon-maskable-512.png");
static APPLE_ICON: &[u8]        = include_bytes!("../static/icons/apple-touch-icon.png");

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

// SSE Event types
#[derive(Clone, Debug)]
enum AppEvent {
    NewAlert(Box<Alert>),
    ConfigReloaded,
}

struct AppState {
    config: SharedConfig,
    client: reqwest::Client,
    cache: SharedAlertCache,
    tx: broadcast::Sender<AppEvent>,
    sse_connections: Arc<AtomicUsize>,
    /// Fingerprints already announced over SSE, per source name. Kept here
    /// instead of being derived from `cache`: caching is opt-in
    /// (`cache_ttl_seconds`, disabled by default), so with the default config
    /// the cache is always empty and every poll re-announced every alert as
    /// new — which made clients refresh in a loop.
    known_fps: Arc<tokio::sync::RwLock<HashMap<String, HashSet<String>>>>,
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

    // No global request timeout: each fetch is wrapped in the source's own
    // `timeout` (see fetch_source_alerts_with_retry). A client-wide timeout
    // would silently cap a source configured with a longer one.
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(config.tls_insecure)
        .connect_timeout(Duration::from_secs(10))
        .user_agent(concat!("alertview/", env!("ALERTVIEW_VERSION")))
        .build()?;

    let shared_config = Arc::new(tokio::sync::RwLock::new(config));
    let cache: SharedAlertCache = Arc::new(tokio::sync::RwLock::new(AlertCache::new()));
    
    // Create broadcast channel for SSE notifications
    let (tx, _rx) = broadcast::channel::<AppEvent>(100);
    
    let state = Arc::new(AppState {
        config: shared_config.clone(),
        client,
        cache,
        tx: tx.clone(),
        sse_connections: Arc::new(AtomicUsize::new(0)),
        known_fps: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    });

    // Start config file watcher
    start_config_watcher(shared_config.clone(), config_path, watch_method, poll_interval, tx.clone());

    // Log cache size limit
    tracing::debug!("Alert cache limited to {} entries", MAX_CACHE_ENTRIES);

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/manifest.webmanifest", get(serve_manifest))
        .route("/sw.js", get(serve_sw))
        .route("/icons/icon-192.png", get(serve_icon_192))
        .route("/icons/icon-512.png", get(serve_icon_512))
        .route("/icons/icon-maskable-512.png", get(serve_icon_maskable))
        .route("/icons/apple-touch-icon.png", get(serve_apple_icon))
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

/// Release version, injected at build time by build.rs (CI git tag, else Cargo version).
const VERSION: &str = env!("ALERTVIEW_VERSION");

async fn serve_index() -> Html<String> {
    Html(INDEX_HTML.replace("__APP_VERSION__", VERSION))
}

async fn serve_css() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "text/css; charset=utf-8")], STYLE_CSS)
}

async fn serve_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript; charset=utf-8")], APP_JS)
}

async fn serve_manifest() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/manifest+json; charset=utf-8")], MANIFEST)
}

async fn serve_sw() -> ([(&'static str, &'static str); 2], &'static str) {
    // service-worker-allowed lets the SW control the whole origin scope ("/").
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("service-worker-allowed", "/"),
        ],
        SW_JS,
    )
}

fn png_response(bytes: &'static [u8]) -> ([(&'static str, &'static str); 2], &'static [u8]) {
    (
        [
            ("content-type", "image/png"),
            ("cache-control", "public, max-age=604800"),
        ],
        bytes,
    )
}

async fn serve_icon_192() -> ([(&'static str, &'static str); 2], &'static [u8]) {
    png_response(ICON_192)
}

async fn serve_icon_512() -> ([(&'static str, &'static str); 2], &'static [u8]) {
    png_response(ICON_512)
}

async fn serve_icon_maskable() -> ([(&'static str, &'static str); 2], &'static [u8]) {
    png_response(ICON_MASKABLE_512)
}

async fn serve_apple_icon() -> ([(&'static str, &'static str); 2], &'static [u8]) {
    png_response(APPLE_ICON)
}

async fn health_check() -> &'static str {
    "OK"
}

// Server-Sent Events handler for real-time alert notifications
// Establishes a connection with the client and streams alert updates in real-time
// Decrements the SSE connection counter whenever the stream is dropped,
// regardless of how it ends (client disconnect, server shutdown, lag).
struct SseGuard(Arc<AtomicUsize>);
impl Drop for SseGuard {
    fn drop(&mut self) {
        let remaining = self.0.fetch_sub(1, Ordering::SeqCst).saturating_sub(1);
        tracing::debug!("SSE connection closed (total: {})", remaining);
    }
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Result<axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, axum::http::StatusCode> {
    use axum::response::sse::{Event, KeepAlive};
    use futures::stream::StreamExt as _;
    use tokio::sync::broadcast::error::RecvError;

    // Reserve a slot; reject if we'd exceed the limit. Decrementing is handled
    // by SseGuard's Drop so a client disconnect can never leak a slot.
    let count = state.sse_connections.fetch_add(1, Ordering::SeqCst) + 1;
    if count > MAX_SSE_CONNECTIONS {
        state.sse_connections.fetch_sub(1, Ordering::SeqCst);
        tracing::warn!("SSE connection limit reached ({}/{})", count - 1, MAX_SSE_CONNECTIONS);
        return Err(axum::http::StatusCode::TOO_MANY_REQUESTS);
    }
    tracing::debug!("SSE connection opened (total: {})", count);
    let guard = SseGuard(state.sse_connections.clone());

    let rx = state.tx.subscribe();

    // Create a stream of SSE events from the broadcast channel. The guard is
    // carried in the stream state so it drops (and decrements) with the stream.
    let event_stream = stream::unfold((rx, guard), move |(mut rx, guard)| async move {
        loop {
            match rx.recv().await {
                Ok(AppEvent::NewAlert(alert)) => {
                    let json = serde_json::to_string(&*alert).unwrap_or_default();
                    let event = Event::default().event("new_alert").data(json);
                    return Some((event, (rx, guard)));
                }
                Ok(AppEvent::ConfigReloaded) => {
                    let event = Event::default().event("config_reloaded").data("config reloaded");
                    return Some((event, (rx, guard)));
                }
                // Receiver fell behind: skip the missed events, keep the connection.
                Err(RecvError::Lagged(_)) => continue,
                // Sender dropped (server shutdown): end the stream.
                Err(RecvError::Closed) => return None,
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
    
    // Fingerprints already announced, per source. Snapshotted so the lock is not
    // held across the network I/O below.
    let known_before: HashMap<String, HashSet<String>> = state.known_fps.read().await.clone();
    let mut refreshed_fps: HashMap<String, HashSet<String>> = HashMap::new();

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
                
                // Detect new alerts and broadcast them. A source with no entry
                // yet has never been fetched successfully (server start, new
                // source in the config): prime it silently rather than
                // announcing its whole backlog as new.
                let fps: HashSet<String> =
                    alerts.iter().map(|a| a.fingerprint.clone()).collect();
                match known_before.get(&source.name) {
                    Some(known) => new_alerts.extend(
                        alerts.iter().filter(|a| !known.contains(&a.fingerprint)).cloned(),
                    ),
                    None => tracing::debug!(
                        "Priming {} known alert(s) for source {}",
                        fps.len(),
                        source.name
                    ),
                }
                refreshed_fps.insert(source.name.clone(), fps);
                
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

    // Remember what has been announced. Sources served from cache or that
    // failed keep their previous entry, so a transient failure does not
    // re-announce everything once the source comes back.
    {
        let configured: HashSet<&str> = config.sources.iter().map(|s| s.name.as_str()).collect();
        let mut known = state.known_fps.write().await;
        known.extend(refreshed_fps);
        known.retain(|name, _| configured.contains(name.as_str()));
    }

    // Broadcast new alerts to SSE clients
    for alert in new_alerts {
        let _ = state.tx.send(AppEvent::NewAlert(Box::new(alert)));
    }

    // Per-source settings are already applied; fill in the config-wide ones.
    // A source that declares nothing inherits display.alert_link_template, and
    // display.source_link: false hides the ↗ button unless the source opts in.
    if config.display.alert_link_template.is_some() || !config.display.source_link {
        let sources: HashMap<&str, &config::Source> =
            config.sources.iter().map(|s| (s.name.as_str(), s)).collect();
        for alert in &mut all_alerts {
            let source = sources.get(alert.source.as_str());
            if alert.alert_link_url.is_none() {
                if let Some(template) = config.display.alert_link_template.as_deref() {
                    if source.is_none_or(|s| s.alert_link_template.is_none()) {
                        alert.alert_link_url = alerts::apply_link_template(template, alert);
                    }
                }
            }
            if !config.display.source_link && source.is_none_or(|s| s.source_link != Some(true)) {
                alert.link_url = None;
            }
        }
    }

    let severity_order = &config.display.severity_order;
    all_alerts.sort_by(|a, b| {
        severity_rank(severity_order, &a.severity)
            .cmp(&severity_rank(severity_order, &b.severity))
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
        custom_css: config.display.custom_css.clone(),
        play_sounds: config.display.play_sounds,
        groups,
        group_by: config.display.group_by.clone(),
        severity_order: severity_order.clone(),
        prefix_labels: config.display.prefix_labels.clone(),
        prefix_separator: config.display.prefix_separator.clone(),
        tv_mode_default: config.display.tv_mode_default,
        link_new_tab: config.display.link_new_tab,
    })
}

// Fetch alerts from a source with retry logic and per-source timeout
// Implements exponential backoff for retries and respects source-specific timeouts
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
                // Don't retry on HTTP 4xx: a 404, 401 or 403 is a configuration
                // problem, retrying it only delays the error by several seconds.
                if let Some(http) = e.downcast_ref::<alerts::HttpStatusError>() {
                    if http.status.is_client_error() {
                        return Err(e);
                    }
                }
                last_error = Some(e);
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
// Automatically reloads configuration when file changes are detected
fn start_config_watcher(shared_config: SharedConfig, config_path: String, watch_method: String, poll_interval_secs: u64, tx: broadcast::Sender<AppEvent>) {
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
                                        // Notify frontend via SSE
                                        let _ = tx.send(AppEvent::ConfigReloaded);
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
        let (debouncer_tx, rx) = std::sync::mpsc::channel();

        // Create debouncer with 500ms delay
        let mut debouncer = match new_debouncer(Duration::from_millis(500), debouncer_tx) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to create config watcher debouncer: {}", e);
                tracing::error!("Falling back to polling method...");
                // Fallback to polling
                tracing::info!("Using polling method to watch config file {} (interval: {}s)...", 
                              config_path, poll_interval_secs);
                start_polling_watcher(shared_config, config_path, poll_interval_secs, tx.clone());
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
                start_polling_watcher(shared_config, config_path, poll_interval_secs, tx.clone());
                return;
            }
        }

        let tx_clone = tx.clone();
        let shared_config_clone = shared_config.clone();
        let config_path_for_task_clone = config_path_for_task.clone();
        
        // Spawn blocking task to handle inotify events (rx is sync mpsc)
        tokio::task::spawn_blocking(move || {
            while let Ok(Ok(events)) = rx.recv() {
                // Debouncer emits event for any modification
                tracing::info!("Detected {} file change event(s) for {}, reloading...", events.len(), config_path_for_task);
                
                let tx_clone = tx_clone.clone();
                let shared_config_clone = shared_config_clone.clone();
                let config_path_for_task = config_path_for_task_clone.clone();
                
                // Load config in a blocking context, but we need async for Config::load_async
                // We'll use tokio::runtime::Handle to spawn an async task
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        match Config::load_async(&config_path_for_task).await {
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
                                // Notify frontend via SSE
                                let _ = tx_clone.send(AppEvent::ConfigReloaded);
                            }
                            Err(e) => {
                                tracing::error!("Failed to reload config: {}", e);
                            }
                        }
                    });
                }
            }
        });
    }
}

// Fallback polling watcher function
// Used when inotify is not available or fails to initialize
fn start_polling_watcher(shared_config: SharedConfig, config_path: String, poll_interval_secs: u64, tx: broadcast::Sender<AppEvent>) {
    use std::fs;
    use std::time::SystemTime;
    use std::time::Duration;

    let poll_interval = Duration::from_secs(poll_interval_secs);
    let tx_clone = tx.clone();

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
                                    // Notify frontend via SSE
                                    let _ = tx_clone.send(AppEvent::ConfigReloaded);
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
