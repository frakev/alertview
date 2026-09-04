use alerts::{severity_rank, Alert, AlertsResponse, SourceStatus};
use axum::{extract::State, response::Html, routing::get, Json, Router};
use config::{Config, SharedConfig};
use futures::stream::{self, StreamExt as _};
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

/// Built as one string rather than printed line by line: `alertview --help |
/// head` closes the pipe early, and every `println!` after that panics with
/// "failed printing to stdout: Broken pipe".
fn help_text() -> String {
    format!(
        "\
AlertView {VERSION} - Alert Aggregation Dashboard

Usage:
  alertview [OPTIONS] [CONFIG_FILE]

Arguments:
  CONFIG_FILE    Path to the configuration file (default: config.yaml)

Options:
  -h, --help     Show this help message and exit
  -V, --version  Print the version and exit

Environment Variables:
  ALERTVIEW_CONFIG               Path to the configuration file
  ALERTVIEW_PORT                 Port to listen on (default: 8080)
  ALERTVIEW_REFRESH_INTERVAL     Browser refresh interval, seconds (default: 30)
  ALERTVIEW_CACHE_TTL            Per-source cache TTL, seconds (default: 0, off)
  ALERTVIEW_LOG_FORMAT           Log format: 'text' or 'json' (default: text)
  ALERTVIEW_CONFIG_WATCH_METHOD  'polling' or 'inotify' (default: polling)
  ALERTVIEW_CONFIG_POLL_INTERVAL Polling interval, seconds (default: 10)
  RUST_LOG                       Log level: error, warn, info, debug, trace

A value written in the configuration file wins over its environment variable.

Examples:
  alertview                          # Use default config.yaml
  alertview /etc/alertview/config.yaml
  alertview --config /etc/alertview/config.yaml
  alertview --version
"
    )
}

/// Writes to stdout without panicking on a closed pipe.
fn print_out(text: &str) {
    use std::io::Write as _;
    let _ = std::io::stdout().write_all(text.as_bytes());
}

// Type aliases for cleaner code
type AlertCache = HashMap<String, (Vec<alerts::Alert>, Instant)>;
type SharedAlertCache = Arc<tokio::sync::RwLock<AlertCache>>;

// How many sources are fetched at once
const MAX_CONCURRENT_FETCHES: usize = 8;

// Maximum number of concurrent SSE connections
const MAX_SSE_CONNECTIONS: usize = 100;

// SSE Event types
#[derive(Clone, Debug)]
enum AppEvent {
    NewAlert(Box<Alert>),
    ConfigReloaded,
    /// Ends every SSE stream so a graceful shutdown can actually drain.
    Shutdown,
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
    /// One gate per cache key. Without it, every browser that arrived while a
    /// cache entry was expired fired its own upstream request.
    inflight: Arc<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

/// Look the cache up, returning a copy only while the entry is still fresh.
async fn cache_hit(state: &AppState, key: &str, ttl: Duration) -> Option<Vec<alerts::Alert>> {
    let cache = state.cache.read().await;
    cache
        .get(key)
        .and_then(|(alerts, ts)| (ts.elapsed() < ttl).then(|| alerts.clone()))
}

/// Credentials embedded in a URL, blanked out. Source errors are served to
/// every browser through /api/alerts, and reqwest puts the failing URL in its
/// message — including a `http://user:pass@host` userinfo.
fn redact_credentials(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(start) = rest.find("://") {
        let (head, tail) = rest.split_at(start + 3);
        out.push_str(head);
        // The authority ends at the first delimiter; look for userinfo inside it.
        let end = tail
            .find(['/', ' ', '"', ','])
            .unwrap_or(tail.len());
        let (authority, after) = tail.split_at(end);
        match authority.rsplit_once('@') {
            Some((_, host)) => {
                out.push_str("***@");
                out.push_str(host);
            }
            None => out.push_str(authority),
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_out(&help_text());
        std::process::exit(0);
    }

    // Answered before anything is read or bound, so `alertview --version`
    // works without a configuration file and without a free port.
    if args.iter().any(|a| a == "--version" || a == "-V") {
        print_out(&format!("alertview {VERSION}\n"));
        std::process::exit(0);
    }

    // Precedence: --config <path> / positional argument, then ALERTVIEW_CONFIG
    // (documented in --help but never read until now), then the default.
    let config_path = args
        .iter()
        .position(|a| a == "--config")
        .and_then(|i| args.get(i + 1).cloned())
        .or_else(|| args.get(1).filter(|a| !a.starts_with('-')).cloned())
        .or_else(|| std::env::var("ALERTVIEW_CONFIG").ok())
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
        "Starting AlertView {VERSION} on port {port} with {} source(s)",
        config.sources.len()
    );
    for s in &config.sources {
        tracing::info!("  • {} ({})", s.name, redact_credentials(&s.url));
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
        inflight: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    });

    // Start config file watcher
    start_config_watcher(shared_config.clone(), config_path, watch_method, poll_interval, tx.clone());

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
    // Without this, a SIGTERM (a Kubernetes rolling update, `docker stop`)
    // killed in-flight requests instead of letting them finish.
    let shutdown_tx = tx.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            // SSE streams never end on their own, so draining would wait for
            // them forever: tell them to close.
            let _ = shutdown_tx.send(AppEvent::Shutdown);
        })
        .await?;
    tracing::info!("Shutdown complete");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(e) => {
                tracing::warn!("Cannot listen for SIGTERM: {}", e);
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Interrupt received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
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
                Ok(AppEvent::Shutdown) => return None,
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
    // Snapshot the config and release the lock: the fetches below take seconds,
    // and holding the read guard would block a config reload for that long.
    let (sources, display, refresh_interval, cache_ttl) = {
        let config = state.config.read().await;
        (
            config.sources.clone(),
            config.display.clone(),
            config.refresh_interval,
            Duration::from_secs(config.cache_ttl_seconds),
        )
    };
    let use_cache = cache_ttl > Duration::from_secs(0);

    // Sources are fetched concurrently — the response used to take the sum of
    // every source's latency. `buffered` keeps the results in config order.
    let jobs: Vec<_> = sources
        .iter()
        .map(|source| {
            let state = state.clone();
            async move {
                let cache_key = format!("{}:{}:{:?}", source.name, source.url, source.source_type);

                if use_cache {
                    if let Some(alerts) = cache_hit(&state, &cache_key, cache_ttl).await {
                        tracing::debug!("Cache hit for source {}", source.name);
                        return (source, cache_key, Ok(alerts), true);
                    }

                    // Single-flight: one fetch per source at a time. N browsers
                    // arriving on an expired entry used to mean N upstream
                    // requests at once, which is the moment the source can
                    // least afford them.
                    let gate = {
                        let mut inflight = state.inflight.lock().await;
                        inflight
                            .entry(cache_key.clone())
                            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                            .clone()
                    };
                    let _permit = gate.lock().await;

                    // The fetch we queued behind has just filled the cache.
                    if let Some(alerts) = cache_hit(&state, &cache_key, cache_ttl).await {
                        tracing::debug!("Cache filled while waiting, source {}", source.name);
                        return (source, cache_key, Ok(alerts), true);
                    }

                    tracing::debug!("Fetching from source {}", source.name);
                    let result = fetch_source_alerts_with_retry(&state.client, source).await;
                    if let Ok(alerts) = &result {
                        let mut cache = state.cache.write().await;
                        cache.insert(cache_key.clone(), (alerts.clone(), Instant::now()));
                    }
                    return (source, cache_key, result, false);
                }

                tracing::debug!("Fetching from source {}", source.name);
                let result = fetch_source_alerts_with_retry(&state.client, source).await;
                (source, cache_key, result, false)
            }
        })
        .collect();

    let fetches = stream::iter(jobs)
        .buffered(MAX_CONCURRENT_FETCHES)
        .collect::<Vec<_>>()
        .await;

    let mut all_alerts = Vec::new();
    let mut source_statuses = Vec::new();
    let mut cache_keys: HashSet<String> = HashSet::new();
    let mut refreshed_fps: HashMap<String, HashSet<String>> = HashMap::new();

    for (source, cache_key, result, from_cache) in fetches {
        cache_keys.insert(cache_key);
        match result {
            Ok(alerts) => {
                tracing::debug!("Got {} alerts from {}", alerts.len(), source.name);

                if !from_cache {
                    refreshed_fps.insert(
                        source.name.clone(),
                        alerts.iter().map(|a| a.fingerprint.clone()).collect(),
                    );
                }

                source_statuses.push(SourceStatus {
                    name: source.name.clone(),
                    status: "ok".to_string(),
                    alert_count: alerts.len(),
                    error: None,
                });
                all_alerts.extend(alerts);
            }
            Err(e) => {
                // Redacted in the log too: logs get shipped off the host.
                tracing::warn!("Failed to fetch from {}: {}", source.name, redact_credentials(&e.to_string()));
                source_statuses.push(SourceStatus {
                    name: source.name.clone(),
                    status: "error".to_string(),
                    alert_count: 0,
                    // /api/alerts is served to every browser: no credentials
                    // from a source URL may travel with the message.
                    error: Some(redact_credentials(&e.to_string())),
                });
            }
        }
    }

    // Diff and record in one critical section: two browsers polling at the
    // same instant used to read the same "already announced" snapshot and both
    // announce the same alerts. Sources served from cache or that failed keep
    // their previous entry, so a transient failure does not re-announce
    // everything once the source comes back.
    let new_fingerprints: HashSet<String> = {
        let configured: HashSet<&str> = sources.iter().map(|s| s.name.as_str()).collect();
        let mut known = state.known_fps.write().await;
        let mut fresh = HashSet::new();
        for (name, fps) in refreshed_fps {
            match known.get(&name) {
                Some(seen) => fresh.extend(fps.difference(seen).cloned()),
                // A source with no entry yet has never been fetched
                // successfully (server start, new source in the config): prime
                // it silently rather than announcing its whole backlog as new.
                None => tracing::debug!("Priming {} known alert(s) for source {}", fps.len(), name),
            }
            known.insert(name, fps);
        }
        known.retain(|name, _| configured.contains(name.as_str()));
        fresh
    };

    // A source that is renamed, removed or repointed leaves its cache entry
    // (and its gate) behind on every config reload otherwise.
    if use_cache {
        state.cache.write().await.retain(|key, _| cache_keys.contains(key));
        state.inflight.lock().await.retain(|key, _| cache_keys.contains(key));
    }

    // Per-source settings are already applied; fill in the config-wide ones.
    // A source that declares nothing inherits display.alert_link_template, and
    // display.source_link: false hides the ↗ button unless the source opts in.
    if display.alert_link_template.is_some() || !display.source_link {
        let by_name: HashMap<&str, &config::Source> =
            sources.iter().map(|s| (s.name.as_str(), s)).collect();
        for alert in &mut all_alerts {
            let source = by_name.get(alert.source.as_str());
            // Note: a source that declares its own template but could not
            // resolve it deliberately does *not* fall back to the global one.
            if alert.alert_link_url.is_none()
                && source.is_none_or(|s| s.alert_link_template.is_none())
            {
                if let Some(template) = display.alert_link_template.as_deref() {
                    alert.alert_link_url = alerts::apply_link_template(template, alert);
                }
            }
            if !display.source_link && source.is_none_or(|s| s.source_link != Some(true)) {
                alert.link_url = None;
            }
        }
    }

    // Broadcast after the links are filled in, so an SSE payload carries the
    // same alert as /api/alerts.
    if !new_fingerprints.is_empty() {
        for alert in all_alerts.iter().filter(|a| new_fingerprints.contains(&a.fingerprint)) {
            let _ = state.tx.send(AppEvent::NewAlert(Box::new(alert.clone())));
        }
    }

    // Decorate-sort-undecorate: severity_rank walks the configured order and
    // normalises aliases, too much work to redo on every comparison.
    let mut ranked: Vec<(usize, Alert)> = all_alerts
        .into_iter()
        .map(|a| (severity_rank(&display.severity_order, &a.severity), a))
        .collect();
    ranked.sort_by(|(ra, a), (rb, b)| ra.cmp(rb).then_with(|| b.starts_at.cmp(&a.starts_at)));
    let all_alerts: Vec<Alert> = ranked.into_iter().map(|(_, a)| a).collect();

    // Group alerts if group_by is configured
    let groups = if !display.group_by.is_empty() {
        alerts::group_alerts(&all_alerts, &display.group_by, &display.severity_order)
    } else {
        vec![]
    };

    Json(AlertsResponse {
        alerts: all_alerts,
        sources: source_statuses,
        refresh_interval,
        display_labels: display.labels,
        timezone: Some(display.timezone),
        theme: display.theme,
        custom_css: display.custom_css,
        play_sounds: display.play_sounds,
        groups,
        group_by: display.group_by,
        severity_order: display.severity_order,
        prefix_labels: display.prefix_labels,
        prefix_separator: display.prefix_separator,
        tv_mode_default: display.tv_mode_default,
        link_new_tab: display.link_new_tab,
        show_alert_name: display.show_alert_name,
        show_labels: display.show_labels,
        critical_icon: display.critical_icon,
        status_icons: display.status_icons,
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
        
        // Exponential backoff, capped at max_delay_ms. The cap used to be
        // applied to the multiplier instead of the delay, so it never bound.
        if attempt > 0 {
            let factor = 1u64.checked_shl(attempt as u32 - 1).unwrap_or(u64::MAX);
            let delay_ms = source
                .retry_policy
                .initial_delay_ms
                .saturating_mul(factor)
                .min(source.retry_policy.max_delay_ms);
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
                // 429 and 408 are the two client errors worth retrying: the
                // source is asking us to slow down, not telling us we are
                // misconfigured.
                if let Some(http) = e.downcast_ref::<alerts::HttpStatusError>() {
                    let retryable = matches!(http.status.as_u16(), 408 | 429);
                    if http.status.is_client_error() && !retryable {
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
                                            tracing::info!("  • {} ({})", s.name, redact_credentials(&s.url));
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
                                    tracing::info!("  • {} ({})", s.name, redact_credentials(&s.url));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_text_documents_every_flag_and_variable() {
        let help = help_text();
        assert!(help.contains(VERSION), "the help banner names the running version");
        for flag in ["-h, --help", "-V, --version", "CONFIG_FILE"] {
            assert!(help.contains(flag), "{flag} missing from --help");
        }
        // --help used to list four of the seven variables the config reads.
        for var in [
            "ALERTVIEW_CONFIG",
            "ALERTVIEW_PORT",
            "ALERTVIEW_REFRESH_INTERVAL",
            "ALERTVIEW_CACHE_TTL",
            "ALERTVIEW_LOG_FORMAT",
            "ALERTVIEW_CONFIG_WATCH_METHOD",
            "ALERTVIEW_CONFIG_POLL_INTERVAL",
            "RUST_LOG",
        ] {
            assert!(help.contains(var), "{var} missing from --help");
        }
    }

    #[test]
    fn test_redact_credentials() {
        // reqwest puts the failing URL in its message, and /api/alerts hands
        // that message to every browser.
        assert_eq!(
            redact_credentials("error sending request for url (http://bob:s3cret@zbx.test/api_jsonrpc.php)"),
            "error sending request for url (http://***@zbx.test/api_jsonrpc.php)"
        );
        // A URL without userinfo is untouched, and so is plain text.
        assert_eq!(
            redact_credentials("HTTP 404 Not Found from https://am.test/api/v2/alerts"),
            "HTTP 404 Not Found from https://am.test/api/v2/alerts"
        );
        assert_eq!(redact_credentials("Timeout after 15s"), "Timeout after 15s");
        // Several URLs in one message.
        assert_eq!(
            redact_credentials("http://a:b@x.test/ then https://c:d@y.test/z"),
            "http://***@x.test/ then https://***@y.test/z"
        );
    }
}
