use alerts::{severity_rank, Alert, AlertsResponse, SourceStatus};
use axum::{extract::State, response::Html, routing::get, Json, Router};
use config::{Config, SharedConfig};
use futures::stream::{self, StreamExt as _};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::compression::CompressionLayer;

pub mod alerts;
pub mod config;

static INDEX_HTML: &str = include_str!("../static/index.html");
static STYLE_CSS: &str = include_str!("../static/style.css");
static APP_JS: &str = include_str!("../static/app.js");
static THEME_JS: &str = include_str!("../static/theme.js");

// PWA assets (manifest, service worker, icons) embedded in the binary.
static MANIFEST: &str = include_str!("../static/manifest.webmanifest");
static SW_JS: &str = include_str!("../static/sw.js");
static ICON_192: &[u8] = include_bytes!("../static/icons/icon-192.png");
static ICON_512: &[u8] = include_bytes!("../static/icons/icon-512.png");
static ICON_MASKABLE_512: &[u8] = include_bytes!("../static/icons/icon-maskable-512.png");
static APPLE_ICON: &[u8] = include_bytes!("../static/icons/apple-touch-icon.png");

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

/// What one source looked like at the last poll. The whole vector is replaced
/// atomically, in config order, so a reader never sees a half-updated picture.
#[derive(Clone)]
struct SourceSnapshot {
    status: SourceStatus,
    alerts: Vec<alerts::Alert>,
}

type SharedSnapshot = Arc<tokio::sync::RwLock<Vec<SourceSnapshot>>>;

// How many sources are fetched at once
const MAX_CONCURRENT_FETCHES: usize = 8;

/// How long the very first request waits for the first poll before answering
/// with whatever is there. Only ever reached just after startup.
const FIRST_POLL_WAIT: Duration = Duration::from_secs(10);

// Maximum number of concurrent SSE connections
const MAX_SSE_CONNECTIONS: usize = 100;

// SSE Event types
#[derive(Clone, Debug)]
enum AppEvent {
    NewAlert(Box<Alert>),
    ConfigReloaded,
    /// A reload was refused; the previous configuration is still being served.
    ConfigError(String),
    /// Ends every SSE stream so a graceful shutdown can actually drain.
    Shutdown,
}

struct AppState {
    config: SharedConfig,
    client: reqwest::Client,
    /// The latest result of each source, updated as each one answers; a source
    /// not yet answered is listed as "pending". `/api/alerts` reads this and nothing else: a
    /// browser request never reaches a source, so the load upstream follows the
    /// number of *sources*, not the number of people watching.
    snapshot: SharedSnapshot,
    /// Flips once the first poll has finished, so the first request can wait
    /// for real data instead of rendering an empty dashboard.
    first_poll: tokio::sync::watch::Receiver<bool>,
    tx: broadcast::Sender<AppEvent>,
    sse_connections: Arc<AtomicUsize>,
    /// Fingerprints already announced over SSE, per source name. Kept here
    /// rather than derived from the snapshot: a source that fails keeps its
    /// previous entry, so a transient outage does not re-announce its whole
    /// backlog as new when it comes back.
    known_fps: Arc<tokio::sync::RwLock<HashMap<String, HashSet<String>>>>,
    /// Signalled when the configuration changes, so a reload applies at once
    /// instead of at the end of the current polling nap.
    wake: Arc<tokio::sync::Notify>,
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
        let end = tail.find(['/', ' ', '"', ',']).unwrap_or(tail.len());
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
        tracing::warn!(
            "TLS certificate verification is DISABLED - this is insecure for production!"
        );
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
    let snapshot: SharedSnapshot = Arc::new(tokio::sync::RwLock::new(Vec::new()));
    let (first_poll_tx, first_poll) = tokio::sync::watch::channel(false);

    // Create broadcast channel for SSE notifications
    let (tx, _rx) = broadcast::channel::<AppEvent>(100);

    let state = Arc::new(AppState {
        config: shared_config.clone(),
        client,
        snapshot,
        first_poll,
        tx: tx.clone(),
        sse_connections: Arc::new(AtomicUsize::new(0)),
        known_fps: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        wake: Arc::new(tokio::sync::Notify::new()),
    });

    // Poll the sources on a schedule of our own rather than on the browsers'.
    tokio::spawn(poll_loop(state.clone(), first_poll_tx));

    // Start config file watcher
    start_config_watcher(
        shared_config.clone(),
        config_path,
        watch_method,
        poll_interval,
        tx.clone(),
        state.wake.clone(),
    );

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/theme.js", get(serve_theme_js))
        .route("/manifest.webmanifest", get(serve_manifest))
        .route("/sw.js", get(serve_sw))
        .route("/icons/icon-192.png", get(serve_icon_192))
        .route("/icons/icon-512.png", get(serve_icon_512))
        .route("/icons/icon-maskable-512.png", get(serve_icon_maskable))
        .route("/icons/apple-touch-icon.png", get(serve_apple_icon))
        .route("/api/alerts", get(get_alerts))
        .route("/health", get(health_check))
        .route("/events", get(sse_handler))
        .layer(axum::middleware::from_fn(security_headers))
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

/// Headers every response carries. Cheap, and they close off whole classes:
/// `nosniff` stops a browser from second-guessing a content type, the referrer
/// policy keeps the dashboard URL — filters included — out of the runbooks it
/// links to, and framing is refused outright.
async fn security_headers(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("referrer-policy", "no-referrer".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    resp
}

/// The document, with its Content-Security-Policy.
///
/// `script-src 'self'` with no `unsafe-inline` is the load-bearing part: it is
/// what stops an injected `onmouseover=` from running, which is exactly the
/// shape of the escaping bug fixed in 0.10.0. That is why the two scripts that
/// used to be inline now live in files.
///
/// `style-src` still allows `unsafe-inline`: six generated `style=` attributes
/// remain, and inline CSS cannot execute — a far weaker position than inline
/// script. The custom stylesheet's origin is added when there is one, otherwise
/// it would simply be refused — to `font-src` and `img-src` too, for the fonts
/// and images it loads from its own host.
async fn serve_index(
    State(state): State<Arc<AppState>>,
) -> ([(&'static str, String); 2], Html<String>) {
    let custom = {
        let config = state.config.read().await;
        custom_stylesheet(&config.display).map(str::to_string)
    };
    let extra = custom
        .as_deref()
        .and_then(css_origin)
        .map(|origin| format!(" {origin}"))
        .unwrap_or_default();

    let csp = format!(
        "default-src 'self'; \
         script-src 'self'; \
         style-src 'self' 'unsafe-inline'{extra}; \
         img-src 'self' data:{extra}; \
         font-src 'self'{extra}; \
         connect-src 'self'; \
         manifest-src 'self'; \
         object-src 'none'; \
         base-uri 'none'; \
         form-action 'none'; \
         frame-ancestors 'none'"
    );

    (
        [
            ("content-security-policy", csp),
            ("cache-control", ASSET_CACHE.to_string()),
        ],
        Html(INDEX_HTML.replace("__APP_VERSION__", VERSION)),
    )
}

/// The stylesheet the frontend layers on top of the theme: `custom_css`, or a
/// `theme` holding a URL — the legacy spelling, still documented and still
/// loaded. Mirrors `data.custom_css || data.theme` in app.js; the two must
/// agree, or the page loads a stylesheet its own policy refuses.
fn custom_stylesheet(display: &config::DisplayConfig) -> Option<&str> {
    display
        .custom_css
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(display.theme.as_deref())
}

/// `https://cdn.example.com/a/b.css` -> `https://cdn.example.com`, so a custom
/// stylesheet can be allowed without opening the policy to every host.
fn css_origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty()).then(|| format!("{scheme}://{host}"))
}

/// Revalidate on every load rather than let the browser guess.
///
/// These carried no cache directive at all, so browsers applied heuristic
/// caching and an upgrade needed a forced refresh to take — which matters more
/// now that the stylesheet carries the icons. `no-cache` does not mean "do not
/// store": the copy is kept and a conditional request confirms it, so the cost
/// is one 304 rather than a re-download.
const ASSET_CACHE: &str = "no-cache";

async fn serve_css() -> ([(&'static str, &'static str); 2], &'static str) {
    (
        [
            ("content-type", "text/css; charset=utf-8"),
            ("cache-control", ASSET_CACHE),
        ],
        STYLE_CSS,
    )
}

async fn serve_js() -> ([(&'static str, &'static str); 2], &'static str) {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", ASSET_CACHE),
        ],
        APP_JS,
    )
}

async fn serve_theme_js() -> ([(&'static str, &'static str); 2], &'static str) {
    (
        [
            ("content-type", "application/javascript; charset=utf-8"),
            ("cache-control", ASSET_CACHE),
        ],
        THEME_JS,
    )
}

async fn serve_manifest() -> ([(&'static str, &'static str); 1], &'static str) {
    (
        [("content-type", "application/manifest+json; charset=utf-8")],
        MANIFEST,
    )
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
) -> Result<
    axum::response::Sse<
        impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
    >,
    axum::http::StatusCode,
> {
    use axum::response::sse::{Event, KeepAlive};
    use futures::stream::StreamExt as _;
    use tokio::sync::broadcast::error::RecvError;

    // Reserve a slot; reject if we'd exceed the limit. Decrementing is handled
    // by SseGuard's Drop so a client disconnect can never leak a slot.
    let count = state.sse_connections.fetch_add(1, Ordering::SeqCst) + 1;
    if count > MAX_SSE_CONNECTIONS {
        state.sse_connections.fetch_sub(1, Ordering::SeqCst);
        tracing::warn!(
            "SSE connection limit reached ({}/{})",
            count - 1,
            MAX_SSE_CONNECTIONS
        );
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
                    let event = Event::default()
                        .event("config_reloaded")
                        .data("config reloaded");
                    return Some((event, (rx, guard)));
                }
                Ok(AppEvent::ConfigError(reason)) => {
                    let event = Event::default().event("config_error").data(reason);
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
    // Wait for the first poll rather than answer with an empty dashboard. Only
    // ever hit on the first request after startup; bounded so a source that
    // never answers cannot hang the browser.
    if !*state.first_poll.borrow() {
        let mut first_poll = state.first_poll.clone();
        let _ = tokio::time::timeout(FIRST_POLL_WAIT, first_poll.wait_for(|done| *done)).await;
    }

    let (display, refresh_interval) = {
        let config = state.config.read().await;
        (config.display.clone(), config.refresh_interval)
    };

    // Served from the last poll: this handler never talks to a source.
    let (mut all_alerts, source_statuses) = {
        let snapshot = state.snapshot.read().await;
        let mut alerts = Vec::with_capacity(snapshot.iter().map(|s| s.alerts.len()).sum());
        let mut statuses = Vec::with_capacity(snapshot.len());
        for source in snapshot.iter() {
            alerts.extend(source.alerts.iter().cloned());
            statuses.push(source.status.clone());
        }
        (alerts, statuses)
    };

    // Decorate-sort-undecorate: severity_rank walks the configured order and
    // normalises aliases, too much work to redo on every comparison.
    let mut ranked: Vec<(usize, Alert)> = all_alerts
        .drain(..)
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
        title_annotations: display.title_annotations,
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
                last_error
                    .as_ref()
                    .map(|e| e.to_string())
                    .unwrap_or_default()
            );
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        let result =
            tokio::time::timeout(timeout, alerts::fetch_source_alerts(client, source)).await;

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
    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("Unknown error after {} retries", max_retries)))
}

/// How often the sources are polled: `cache_ttl_seconds` when it is set — it
/// always meant "do not hit the sources more often than this" — and otherwise
/// the browser's own `refresh_interval`, which is the rate the dashboard
/// expects data to change at.
fn poll_interval(config: &Config) -> Duration {
    if config.cache_ttl_seconds > 0 {
        Duration::from_secs(config.cache_ttl_seconds)
    } else {
        Duration::from_secs(config.refresh_interval)
    }
}

/// Polls every source on a schedule of AlertView's own, rather than on the
/// browsers'. Before this, `/api/alerts` fetched upstream on every request:
/// with caching off — the default — ten dashboards meant ten times the load on
/// Alertmanager and Zabbix, an unauthenticated request amplified into four to
/// seven of them, and a slow source left the browser hanging for up to a minute
/// behind a single-flight gate. The load now follows the number of sources.
async fn poll_loop(state: Arc<AppState>, first_poll: tokio::sync::watch::Sender<bool>) {
    loop {
        let interval = {
            let config = state.config.read().await;
            poll_interval(&config)
        };

        poll_once(&state).await;
        let _ = first_poll.send(true);

        // A config reload should apply now, not at the end of the current nap.
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = state.wake.notified() => {
                tracing::debug!("Poll cycle woken early by a config change");
            }
        }
    }
}

/// One pass over every source: fetch, publish, and announce what is new.
async fn poll_once(state: &Arc<AppState>) {
    let (sources, display) = {
        let config = state.config.read().await;
        (config.sources.clone(), config.display.clone())
    };

    // Line the snapshot up with the configuration before fetching anything: a
    // source never answered yet shows as "pending" rather than being absent,
    // so a dashboard served mid-poll does not read as "all clear".
    {
        let mut snapshot = state.snapshot.write().await;
        let mut previous: HashMap<String, SourceSnapshot> = snapshot
            .drain(..)
            .map(|s| (s.status.name.clone(), s))
            .collect();
        *snapshot = sources
            .iter()
            .map(|source| {
                previous
                    .remove(&source.name)
                    .unwrap_or_else(|| SourceSnapshot {
                        status: SourceStatus {
                            name: source.name.clone(),
                            status: "pending".to_string(),
                            alert_count: 0,
                            error: None,
                        },
                        alerts: Vec::new(),
                    })
            })
            .collect();
    }
    {
        let configured: HashSet<&str> = sources.iter().map(|s| s.name.as_str()).collect();
        state
            .known_fps
            .write()
            .await
            .retain(|name, _| configured.contains(name.as_str()));
    }

    let jobs: Vec<_> = sources
        .iter()
        .map(|source| {
            let client = state.client.clone();
            async move {
                tracing::debug!("Polling source {}", source.name);
                (
                    source,
                    fetch_source_alerts_with_retry(&client, source).await,
                )
            }
        })
        .collect();

    // Each source is published as soon as it answers: one that is slow or
    // retrying no longer holds back the others, which matters most on the first
    // poll, when there is nothing older to show.
    let mut results = stream::iter(jobs).buffer_unordered(MAX_CONCURRENT_FETCHES);
    while let Some((source, result)) = results.next().await {
        let fresh = match result {
            Ok(mut alerts) => {
                tracing::debug!("Got {} alerts from {}", alerts.len(), source.name);
                apply_display_links(&mut alerts, source, &display);
                announce_new(state, &source.name, &alerts).await;
                SourceSnapshot {
                    status: SourceStatus {
                        name: source.name.clone(),
                        status: "ok".to_string(),
                        alert_count: alerts.len(),
                        error: None,
                    },
                    alerts,
                }
            }
            Err(e) => {
                // Redacted in the log too: logs get shipped off the host.
                let reason = redact_credentials(&e.to_string());
                tracing::warn!("Failed to fetch from {}: {}", source.name, reason);
                SourceSnapshot {
                    status: SourceStatus {
                        name: source.name.clone(),
                        status: "error".to_string(),
                        alert_count: 0,
                        error: Some(reason),
                    },
                    alerts: Vec::new(),
                }
            }
        };

        let mut snapshot = state.snapshot.write().await;
        if let Some(slot) = snapshot.iter_mut().find(|s| s.status.name == source.name) {
            *slot = fresh;
        }
    }
}

/// Records a source's fingerprints and announces the ones not seen before. A
/// source that fails is never passed here, so it keeps its previous entry and a
/// transient outage does not re-announce its whole backlog when it comes back.
async fn announce_new(state: &Arc<AppState>, source: &str, alerts: &[Alert]) {
    let fps: HashSet<String> = alerts.iter().map(|a| a.fingerprint.clone()).collect();
    let mut known = state.known_fps.write().await;
    match known.get(source) {
        Some(seen) => {
            for alert in alerts.iter().filter(|a| !seen.contains(&a.fingerprint)) {
                let _ = state.tx.send(AppEvent::NewAlert(Box::new(alert.clone())));
            }
        }
        // No entry yet means this source has never been fetched successfully:
        // prime it silently rather than announcing everything it is already
        // firing.
        None => tracing::debug!("Priming {} known alert(s) for source {}", fps.len(), source),
    }
    known.insert(source.to_string(), fps);
}

/// The config-wide link settings, applied on top of the per-source ones the
/// fetcher has already resolved.
fn apply_display_links(
    alerts: &mut [alerts::Alert],
    source: &config::Source,
    display: &config::DisplayConfig,
) {
    if display.alert_link_template.is_none() && display.source_link {
        return;
    }
    for alert in alerts.iter_mut() {
        // A source that declares its own template but could not resolve it
        // deliberately does *not* fall back to the global one.
        if alert.alert_link_url.is_none() && source.alert_link_template.is_none() {
            if let Some(template) = display.alert_link_template.as_deref() {
                alert.alert_link_url = alerts::apply_link_template(template, alert);
            }
        }
        if !display.source_link && source.source_link != Some(true) {
            alert.link_url = None;
        }
    }
}

/// Reload the configuration, or explain why it could not be.
///
/// Written once rather than three times: the copy in the fallback polling
/// watcher had drifted and was logging source URLs without redacting the
/// credentials in them. A rejected reload is now broadcast as well as logged —
/// the running config stays up, and until now the only trace was a line nobody
/// reads on a wall display.
async fn reload_config(
    shared_config: &SharedConfig,
    config_path: &str,
    tx: &broadcast::Sender<AppEvent>,
    wake: &tokio::sync::Notify,
) {
    match Config::load_async(config_path).await {
        Ok(new_config) => {
            let mut cfg = shared_config.write().await;
            *cfg = new_config;
            tracing::info!(
                "Config reloaded successfully with {} source(s)",
                cfg.sources.len()
            );
            for s in &cfg.sources {
                tracing::info!("  • {} ({})", s.name, redact_credentials(&s.url));
            }
            let _ = tx.send(AppEvent::ConfigReloaded);
            // Re-poll now: the new sources should show up immediately, not at
            // the end of the current nap.
            wake.notify_one();
        }
        Err(e) => {
            let reason = redact_credentials(&e.to_string());
            tracing::error!(
                "Failed to reload config, keeping the running one: {}",
                reason
            );
            let _ = tx.send(AppEvent::ConfigError(reason));
        }
    }
}

// Function to watch config file for changes (using inotify or polling)
// Automatically reloads configuration when file changes are detected
fn start_config_watcher(
    shared_config: SharedConfig,
    config_path: String,
    watch_method: String,
    poll_interval_secs: u64,
    tx: broadcast::Sender<AppEvent>,
    wake: Arc<tokio::sync::Notify>,
) {
    use std::path::Path;
    use std::{fs, time::SystemTime};

    let poll_interval = Duration::from_secs(poll_interval_secs);

    // Check if config file exists before watching
    if !Path::new(&config_path).exists() {
        tracing::error!(
            "Config file {} does not exist. Auto-reload is DISABLED.",
            config_path
        );
        tracing::error!("Changes to config file will not be detected.");
        return;
    }

    let config_path_for_task = config_path.clone();

    if watch_method == "polling" {
        tracing::info!(
            "Using polling method to watch config file {} (interval: {}s)...",
            config_path,
            poll_interval_secs
        );

        // Use polling method
        tokio::spawn(async move {
            let mut last_modified: Option<SystemTime> = None;

            // Get initial modification time
            if let Ok(metadata) = fs::metadata(&config_path_for_task) {
                last_modified = Some(metadata.modified().ok().unwrap_or(SystemTime::now()));
            }

            let shared_config_clone = shared_config.clone();
            let config_path_clone = config_path_for_task.clone();
            let wake_clone = wake.clone();

            loop {
                tokio::time::sleep(poll_interval).await;

                // Check if file was modified
                match fs::metadata(&config_path_clone) {
                    Ok(metadata) => {
                        if let Ok(modified) = metadata.modified() {
                            if last_modified.as_ref().is_none_or(|&last| modified > last) {
                                // File was modified
                                tracing::info!(
                                    "Config file {} modified, reloading...",
                                    config_path_clone
                                );
                                last_modified = Some(modified);

                                reload_config(
                                    &shared_config_clone,
                                    &config_path_clone,
                                    &tx,
                                    &wake_clone,
                                )
                                .await;
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
                tracing::info!(
                    "Using polling method to watch config file {} (interval: {}s)...",
                    config_path,
                    poll_interval_secs
                );
                start_polling_watcher(
                    shared_config,
                    config_path,
                    poll_interval_secs,
                    tx.clone(),
                    wake,
                );
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
                tracing::info!(
                    "Watching config file {} for changes using inotify...",
                    config_path
                );
            }
            Err(e) => {
                tracing::error!("Failed to watch config file {}: {}", config_path_clone, e);
                tracing::error!("Falling back to polling method...");
                // Fallback to polling
                tracing::info!(
                    "Using polling method to watch config file {} (interval: {}s)...",
                    config_path,
                    poll_interval_secs
                );
                start_polling_watcher(
                    shared_config,
                    config_path,
                    poll_interval_secs,
                    tx.clone(),
                    wake,
                );
                return;
            }
        }

        let tx_clone = tx.clone();
        let shared_config_clone = shared_config.clone();
        let config_path_for_task_clone = config_path_for_task.clone();
        let wake_outer = wake.clone();

        // Spawn blocking task to handle inotify events (rx is sync mpsc)
        tokio::task::spawn_blocking(move || {
            while let Ok(Ok(events)) = rx.recv() {
                // Debouncer emits event for any modification
                tracing::info!(
                    "Detected {} file change event(s) for {}, reloading...",
                    events.len(),
                    config_path_for_task
                );

                let tx_clone = tx_clone.clone();
                let shared_config_clone = shared_config_clone.clone();
                let config_path_for_task = config_path_for_task_clone.clone();
                let wake_clone = wake_outer.clone();

                // Load config in a blocking context, but we need async for Config::load_async
                // We'll use tokio::runtime::Handle to spawn an async task
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        reload_config(
                            &shared_config_clone,
                            &config_path_for_task,
                            &tx_clone,
                            &wake_clone,
                        )
                        .await;
                    });
                }
            }
        });
    }
}

// Fallback polling watcher function
// Used when inotify is not available or fails to initialize
fn start_polling_watcher(
    shared_config: SharedConfig,
    config_path: String,
    poll_interval_secs: u64,
    tx: broadcast::Sender<AppEvent>,
    wake: Arc<tokio::sync::Notify>,
) {
    use std::fs;
    use std::time::Duration;
    use std::time::SystemTime;

    let poll_interval = Duration::from_secs(poll_interval_secs);
    let tx_clone = tx.clone();
    let wake_clone = wake.clone();

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
                            tracing::info!(
                                "Config file {} modified, reloading...",
                                config_path_clone
                            );
                            last_modified = Some(modified);

                            reload_config(
                                &shared_config_clone,
                                &config_path_clone,
                                &tx_clone,
                                &wake_clone,
                            )
                            .await;
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
        assert!(
            help.contains(VERSION),
            "the help banner names the running version"
        );
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

    use std::sync::atomic::AtomicU32;

    /// A stub Alertmanager that counts how many times it is asked for alerts.
    /// The count is the point: it is what proves a browser request does not
    /// reach a source.
    async fn spawn_counting_stub() -> (String, Arc<AtomicU32>) {
        use axum::{routing::get, Router};
        let hits = Arc::new(AtomicU32::new(0));
        let alerts_hits = hits.clone();
        const ALERTS: &str = r#"[{
            "fingerprint": "abc",
            "status": {"state": "active"},
            "labels": {"alertname": "DiskFull", "severity": "critical"},
            "annotations": {"summary": "disk full"},
            "startsAt": "2026-09-11T10:00:00Z",
            "endsAt": "0001-01-01T00:00:00Z"
        }]"#;

        let app = Router::new()
            .route(
                "/api/v2/alerts",
                get(move || {
                    alerts_hits.fetch_add(1, Ordering::SeqCst);
                    async { ([("content-type", "application/json")], ALERTS) }
                }),
            )
            .route(
                "/api/v2/silences",
                get(|| async { ([("content-type", "application/json")], "[]") }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{}", addr), hits)
    }

    fn test_state(url: &str) -> (Arc<AppState>, tokio::sync::watch::Sender<bool>) {
        test_state_yaml(&format!(
            "refresh_interval: 30\nsources:\n  - name: \"AM\"\n    type: alertmanager\n    url: \"{url}\"\n"
        ))
    }

    fn test_state_yaml(yaml: &str) -> (Arc<AppState>, tokio::sync::watch::Sender<bool>) {
        let config = Config::from_yaml("test", yaml).expect("test config");
        let (tx, _rx) = broadcast::channel(100);
        let (first_tx, first_poll) = tokio::sync::watch::channel(false);
        (
            Arc::new(AppState {
                config: Arc::new(tokio::sync::RwLock::new(config)),
                client: reqwest::Client::new(),
                snapshot: Arc::new(tokio::sync::RwLock::new(Vec::new())),
                first_poll,
                tx,
                sse_connections: Arc::new(AtomicUsize::new(0)),
                known_fps: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
                wake: Arc::new(tokio::sync::Notify::new()),
            }),
            first_tx,
        )
    }

    #[tokio::test]
    async fn test_a_browser_request_never_reaches_a_source() {
        // The whole point of polling in the background. Before it, every
        // request fetched upstream: one unauthenticated call to AlertView
        // became several to Alertmanager, Grafana and Zabbix, and the load grew
        // with the number of people watching.
        let (url, hits) = spawn_counting_stub().await;
        let (state, first_tx) = test_state(&url);

        poll_once(&state).await;
        let _ = first_tx.send(true);
        assert_eq!(hits.load(Ordering::SeqCst), 1, "the poll fetches once");

        for _ in 0..20 {
            let Json(payload) = get_alerts(State(state.clone())).await;
            assert_eq!(payload.alerts.len(), 1);
            assert_eq!(payload.sources[0].status, "ok");
        }
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "20 dashboard requests must not add a single upstream one"
        );
    }

    #[tokio::test]
    async fn test_a_failing_source_is_reported_not_hidden() {
        let (state, first_tx) = test_state("http://127.0.0.1:1/nowhere");
        {
            // One attempt, so the test does not sit through the retry ladder.
            let mut config = state.config.write().await;
            config.sources[0].retry_policy.max_retries = 0;
            config.sources[0].timeout = 2;
        }

        poll_once(&state).await;
        let _ = first_tx.send(true);

        let Json(payload) = get_alerts(State(state.clone())).await;
        assert_eq!(payload.sources.len(), 1);
        assert_eq!(payload.sources[0].status, "error");
        assert!(payload.sources[0].error.is_some());
        assert!(payload.alerts.is_empty());
    }

    #[tokio::test]
    async fn test_a_slow_source_does_not_hold_back_the_others() {
        // A source that accepts the connection and never answers: with the
        // whole poll published at once, the first dashboard showed nothing at
        // all — an empty "all clear" — until it gave up.
        let (url, _hits) = spawn_counting_stub().await;
        let silent = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let silent_url = format!("http://{}", silent.local_addr().unwrap());
        let (state, first_tx) = test_state_yaml(&format!(
            "refresh_interval: 30\nsources:\n  \
               - name: \"Slow\"\n    type: alertmanager\n    url: \"{silent_url}\"\n    timeout: 30\n  \
               - name: \"AM\"\n    type: alertmanager\n    url: \"{url}\"\n"
        ));

        // As after FIRST_POLL_WAIT: requests stop waiting and read the snapshot.
        let _ = first_tx.send(true);
        let poll = tokio::spawn({
            let state = state.clone();
            async move { poll_once(&state).await }
        });
        let mut payload = None;
        for _ in 0..100 {
            let Json(p) = get_alerts(State(state.clone())).await;
            if p.sources.iter().any(|s| s.name == "AM" && s.status == "ok") {
                payload = Some(p);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        poll.abort();
        drop(first_tx);

        let payload = payload.expect("the fast source is published while the slow one hangs");
        assert_eq!(payload.alerts.len(), 1);
        // Listed in configuration order, the slow one as pending rather than
        // missing, so the dashboard does not read as "all clear".
        assert_eq!(payload.sources[0].name, "Slow");
        assert_eq!(payload.sources[0].status, "pending");
        drop(silent);
    }

    #[tokio::test]
    async fn test_new_alerts_are_announced_once() {
        let (url, _hits) = spawn_counting_stub().await;
        let (state, _first_tx) = test_state(&url);
        let mut events = state.tx.subscribe();

        // The first poll primes: a dashboard starting up must not announce
        // everything already firing as new.
        poll_once(&state).await;
        assert!(
            events.try_recv().is_err(),
            "the priming poll announces nothing"
        );

        // The same alerts again are not new either.
        poll_once(&state).await;
        assert!(
            events.try_recv().is_err(),
            "unchanged alerts announce nothing"
        );
    }

    #[test]
    fn test_custom_stylesheet() {
        let display = |yaml: &str| -> config::DisplayConfig {
            serde_yaml::from_str(yaml).expect("display config")
        };
        // The legacy spelling is still loaded by the page, so the policy must
        // allow it: before, only `custom_css` was, and the theme was refused.
        let legacy = display("theme: https://cdn.example.com/t.css");
        assert_eq!(
            custom_stylesheet(&legacy).and_then(css_origin).as_deref(),
            Some("https://cdn.example.com")
        );
        let both = display(
            "theme: https://old.example.com/t.css\ncustom_css: https://new.example.com/c.css",
        );
        assert_eq!(
            custom_stylesheet(&both),
            Some("https://new.example.com/c.css")
        );
        // An empty custom_css falls back like `data.custom_css || data.theme`.
        let empty = display("theme: https://cdn.example.com/t.css\ncustom_css: \"\"");
        assert_eq!(
            custom_stylesheet(&empty),
            Some("https://cdn.example.com/t.css")
        );
        // A plain theme name adds nothing.
        assert_eq!(
            custom_stylesheet(&display("theme: dark")).and_then(css_origin),
            None
        );
    }

    #[test]
    fn test_css_origin() {
        // Only the origin goes into the policy: allowing the full URL would not
        // work, and allowing the scheme alone would open it to every host.
        assert_eq!(
            css_origin("https://cdn.example.com/themes/dark.css").as_deref(),
            Some("https://cdn.example.com")
        );
        assert_eq!(
            css_origin("http://cdn.example.com:8080/a.css?v=2").as_deref(),
            Some("http://cdn.example.com:8080")
        );
        // Anything that is not http(s) must not reach the header.
        assert_eq!(css_origin("javascript:alert(1)"), None);
        assert_eq!(css_origin("data:text/css,body{}"), None);
        assert_eq!(css_origin("dark"), None);
    }

    #[test]
    fn test_redact_credentials() {
        // reqwest puts the failing URL in its message, and /api/alerts hands
        // that message to every browser.
        assert_eq!(
            redact_credentials(
                "error sending request for url (http://bob:s3cret@zbx.test/api_jsonrpc.php)"
            ),
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

/// Scanners that tie a documentation claim to a fact in the code.
///
/// The documentation drifted until 71 % of it described options that never
/// existed, because nothing connected the two. These three checks are cheap and
/// they close the ways it drifted: invented placeholders, invented environment
/// variables, and links to files that were deleted.
#[cfg(test)]
mod docs {
    use std::path::{Path, PathBuf};

    fn markdown_files() -> Vec<PathBuf> {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "md") {
                    out.push(path);
                }
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut out = vec![root.join("README.md")];
        walk(&root.join("docs"), &mut out);
        out
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    }

    /// The same text with shell fences removed. `docker inspect -f
    /// '{{.NetworkSettings.IPAddress}}'` is a Go template belonging to Docker,
    /// not a link template belonging to AlertView.
    fn without_shell_blocks(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut in_shell = false;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                let lang = trimmed.trim_start_matches('`').trim();
                if in_shell {
                    in_shell = false;
                    continue;
                }
                in_shell = matches!(lang, "bash" | "sh" | "shell" | "console");
                continue;
            }
            if !in_shell {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    #[test]
    fn every_documented_placeholder_exists() {
        // An unresolved placeholder makes apply_link_template drop the whole
        // URL, so a documented one that does not exist silently removes the
        // link the reader was promised.
        let mut wrong = Vec::new();
        for path in markdown_files() {
            let text = without_shell_blocks(&read(&path));
            for (offset, _) in text.match_indices("{{.") {
                let Some(end) = text[offset..].find("}}") else {
                    continue;
                };
                let placeholder = &text[offset..offset + end + 2];
                let ok = placeholder.starts_with("{{.Labels.")
                    || placeholder.starts_with("{{.Annotations.")
                    || crate::alerts::SCALAR_PLACEHOLDERS.contains(&placeholder);
                if !ok {
                    wrong.push(format!("{}: {}", path.display(), placeholder));
                }
            }
        }
        wrong.sort();
        wrong.dedup();
        assert!(
            wrong.is_empty(),
            "placeholders the code does not substitute:\n  {}",
            wrong.join("\n  ")
        );
    }

    #[test]
    fn environment_variables_match_the_code() {
        let files = markdown_files();
        let corpus: String = files.iter().map(|p| read(p)).collect();

        // Invented ones: ALERTVIEW_CONFIG_PATH, ALERTVIEW_TLS_INSECURE and two
        // more were documented for months.
        let mut invented = Vec::new();
        for (offset, _) in corpus.match_indices("ALERTVIEW_") {
            let name: String = corpus[offset..]
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || *c == '_' || c.is_ascii_digit())
                .collect();
            let is_bare_prefix = name == "ALERTVIEW_";
            if !is_bare_prefix
                && name != "ALERTVIEW_VERSION"
                && !crate::config::ENV_VARS.contains(&name.as_str())
            {
                invented.push(name);
            }
        }
        invented.sort();
        invented.dedup();
        assert!(
            invented.is_empty(),
            "documented but never read: {invented:?}"
        );

        // And the reverse: two real variables appeared nowhere.
        let missing: Vec<_> = crate::config::ENV_VARS
            .iter()
            .filter(|v| !corpus.contains(*v))
            .collect();
        assert!(
            missing.is_empty(),
            "read but documented nowhere: {missing:?}"
        );
    }

    #[test]
    fn every_internal_link_resolves() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut dead = Vec::new();
        for path in markdown_files() {
            let text = read(&path);
            let dir = path.parent().unwrap();
            for (offset, _) in text.match_indices("](") {
                let rest = &text[offset + 2..];
                let Some(end) = rest.find(')') else { continue };
                let target = rest[..end].split('#').next().unwrap_or("");
                if target.is_empty() || target.contains("://") || target.starts_with('#') {
                    continue;
                }
                if !(target.ends_with(".md") || target.ends_with(".example")) {
                    continue;
                }
                let resolved = if let Some(stripped) = target.strip_prefix('/') {
                    root.join(stripped)
                } else {
                    dir.join(target)
                };
                if !resolved.exists() {
                    dead.push(format!("{} -> {}", path.display(), target));
                }
            }
        }
        assert!(dead.is_empty(), "dead links:\n  {}", dead.join("\n  "));
    }
}
