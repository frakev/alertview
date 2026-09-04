# Changelog

All notable changes to AlertView are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-09-04

### Changed
- **Link templates are percent-encoded and validated** — values substituted into a `link_template` are now percent-encoded, so a label containing a space, `&` or `/` no longer produces a broken URL. A template referencing a label the alert does not carry is treated as unusable instead of emitting a URL with `{{.Labels.foo}}` left in it: the source link falls back to the generator URL then to `dashboard_url`, and the alert link is simply dropped. Only `http`/`https` links are handed to the frontend, so a `javascript:` URL coming from a source's `generatorURL` can no longer be rendered as a clickable link.
- **Silences are only fetched when something is silenced** — the Alertmanager `/silences` endpoint was queried once per source on every poll just to resolve comments; it is now skipped when no alert reports a silence.
- **User agent reports the real version** — was hardcoded to `alertview/0.1`.

### Added
- **Prefix labels** — `display.prefix_labels` (default `["hostname"]`) shows labels in front of the alert name, joined by `display.prefix_separator` (default `" / "`), in both normal and TV mode. Only the labels the alert carries are rendered, they are shown even when absent from `display.labels`, and they are removed from the trailing label chips so nothing appears twice.
- **TV mode by default** — `display.tv_mode_default` starts the dashboard in TV mode in a browser where nobody has used the TV button yet. An explicit choice still wins and is remembered; `?tv=1` in the URL wins over both, which is the reliable option for a kiosk display.
- **Automatic light/dark theme** — `display.theme` accepts `auto` (now the default), which follows the OS setting and switches live when the OS switches. The theme button cycles `auto → light → dark` and its icon shows the preference rather than the resolved theme. The theme is now resolved by a small inline script before the first paint, so a light-theme user no longer gets a flash of the dark palette, and `<meta name="theme-color">` follows the resolved theme. `display.theme` set to `dark` or `light` finally has an effect — it was documented but silently ignored, only `localStorage` drove the theme. A `theme` holding a URL is still treated as a custom stylesheet, and `display.custom_css` is the explicit way to declare one.
- **Clickable alerts** — `display.alert_link_template` (overridable per source) makes the whole card or TV row clickable, with a URL built from the alert's labels. It is fully independent from the ↗ "open in the source" button, which keeps its own destination and can be hidden with `display.source_link: false` (also overridable per source). `display.link_new_tab: false` keeps links in the same tab for kiosk displays.
- **Configurable severity order** — `display.severity_order` defines the severity ranking (most severe first), used for alert sorting, the filter chips in normal and TV mode, the sound picked for a batch of new alerts and the group severity badges. Defaults to `["critical", "error", "high", "warning", "info", "none"]`. Custom levels can be declared and any severity missing from the list sorts after every listed level; matching is case-insensitive and understands the `crit`/`err`/`warn`/`information` aliases.

### Fixed
- **SSE feedback loop hammering the sources** — new alerts were detected by comparing against the alert cache, which is only written when `cache_ttl_seconds > 0` (disabled by default). With the default config the cache was always empty, so *every* `/api/alerts` call re-announced *every* alert as new over SSE, and the frontend refreshed on each `new_alert` event. With two browser tabs open this became self-sustaining: measured at ~87 requests/second to the upstream Alertmanager instead of one per `refresh_interval`. Announced fingerprints are now tracked per source in the server state, independently of the cache, and a source seen for the first time is primed silently instead of announcing its whole backlog. The frontend now treats an SSE event as a debounced "refresh soon" signal and no longer plays the sound and notification from there, which also removes the duplicate alerting (`fetchAlerts()` already does its own new-alert diff).
- **HTTP 4xx responses were retried** — the retry loop detected the status code by parsing the first whitespace-separated token of the error message, which is always the literal `HTTP`, so the "don't retry on 4xx" branch never ran. A misconfigured source (404, 401, 403) cost 3 retries and ~7s on every refresh; it now fails in ~10ms. The status code is carried by a typed `HttpStatusError` instead of being re-parsed from a string.
- **Per-source `timeout` was capped at 15s** — the shared HTTP client was built with a 15s global timeout, so a source configured with a longer `timeout` was cut off at 15s anyway. The client no longer sets a global timeout (each fetch is already wrapped in the source's own timeout); a 10s connect timeout is kept.
- **`error` severity sorted last** — `error` was not part of the built-in ranking, so alerts carrying it fell below `info` and `none`, got no filter chip and were rendered with the neutral grey style. It is now a first-class level between `critical` and `high`, with its own color in both normal and TV mode.
- **TV mode rows could hide every label** — the two inline label slots were sliced from the configured list *before* checking which labels the alert actually carries, so an alert missing the first two configured labels showed none inline and pushed the rest behind the `+N` toggle. Labels are now filtered by presence first.
- **`display:` section omitted lost every default** — `DisplayConfig` derived `Default`, which serde uses when the whole `display:` block is missing, bypassing the per-field defaults: no card labels and an empty timezone. `Default` now returns the documented values.

## [0.5.7] - 2026-09-04

### Fixed
- **404 on Alertmanager and Grafana sources** — the source `url` was used as the full API path, so a documented base URL (`http://127.0.0.1:9093`) was queried as `/alerts` instead of `/api/v2/alerts` and every fetch returned `HTTP 404`. The API path is built again from the source type: `{url}/api/v2` for Alertmanager, `{url}/api/alertmanager/grafana/api/v2` for Grafana. URLs that already point at the API (with or without a trailing `/alerts`) are kept as-is, so existing workarounds keep working, and a query string such as `?active=true` is preserved on the alerts request.

## [0.5.6] - 2026-06-23

### Changed
- **More visible trigger age** — the "for …" alert age is now rendered in amber (`--c-warning`) and semibold instead of the dim grey, in both normal and TV modes, so the time since the alert fired stands out.

## [0.5.5] - 2026-06-23

### Fixed
- **SSE connection counter leak (HTTP 429)** — the `/events` connection counter was only decremented when the broadcast channel closed, never on a normal client disconnect (tab close, reload, reconnect). The count leaked upward on every page load until it hit the limit and `/events` returned `429 Too Many Requests` permanently. The counter is now an atomic decremented by a RAII guard that runs whenever the stream is dropped, so disconnects can no longer leak slots. A lagging receiver also no longer terminates the stream — missed events are skipped and the connection is kept.

### Changed
- **TV mode: datasource on hover** — the datasource chip is no longer shown inline in TV mode rows; the datasource name now appears in the tooltip of the link button that opens Grafana/Alertmanager/Zabbix.
- **TV mode: label / age order** — labels are now shown before the "for …" trigger age in TV mode rows.

## [0.5.4] - 2026-06-23

### Reverted
- **TV mode row alignment (0.5.3)** — reverted the fixed-column layout of the trailing metadata in TV mode; rows are back to the previous flexible layout.

## [0.5.3] - 2026-06-23

### Changed
- **TV mode row alignment** — in TV mode, the trailing metadata (source, duration, labels, link) is now laid out in fixed columns so it lines up cleanly across rows. The duration column is right-aligned and the label slot is always reserved, so rows without labels no longer shift the source/duration out of alignment.

## [0.5.2] - 2026-06-23

### Added
- **TV mode in URL state** — the TV mode toggle is now reflected in the URL (`?tv=1`) like the search query and severity filter, so a TV view can be bookmarked or shared via a direct link. URL state takes precedence over the locally stored preference on load.

## [0.5.1] - 2026-06-23

### Fixed
- **Grafana/Alertmanager alert links** — alert links now point to the per-alert URL (`generatorURL`, or a configured `link_template`) instead of the static `dashboard_url`. Previously a `dashboard_url` set to the Grafana home page made every alert link to that page rather than the alert itself. The static `dashboard_url` is now only used as a fallback.

### Changed
- **Incremental alert rendering** — the alert list is now reconciled in place on each refresh: only added, removed or modified alerts touch the DOM, instead of rebuilding the whole list. This removes flicker, preserves scroll position, and keeps expanded groups open across refreshes.

## [0.5.0] - 2026-06-23

### Added
- **Configurable severity label per source** — each Alertmanager/Grafana source can set `severity_label` to choose which label is used to classify severity (defaults to `severity`). Useful when alerts carry the level under a different label name (e.g. `priority`).

### Changed
- Severity label lookup is now case-insensitive, so labels like `Severity` or `SEVERITY` are classified correctly.

## [0.4.6] - 2026-06-13

### Added
- **Progressive Web App (PWA) support** — AlertView is now installable on Android (Chrome), iOS (Safari) and desktop
  - Web app manifest (`/manifest.webmanifest`) with standalone display mode and app icons
  - Service worker (`/sw.js`) caching the static app shell; live data (`/api/*`, `/events`) is never cached
  - App icons (192px, 512px, maskable, apple-touch) embedded in the binary
  - Requires a secure context (HTTPS, or `localhost`) for installation
- Comprehensive documentation structure in `docs/` directory
- API documentation (`docs/api.md`)
- Troubleshooting guide (`docs/troubleshooting.md`)
- FAQ (`docs/faq.md`)
- Example configurations (`docs/examples/`)
- Development documentation (`docs/development/`)

### Changed
- Updated README with comprehensive documentation links

### Fixed
- Alertmanager sources no longer fail to load when the silences endpoint is unreachable; the error is logged and alerts are still served without silence comments

## [0.3.0] - 2024-01-15

### Added
- **Log format configuration**: Added `log_format` field to config file (supports "text" and "json")
  - Can be set via config file or `ALERTVIEW_LOG_FORMAT` environment variable
  - Priority: env var > config file > default ("text")
- **Enhanced README**: Added comprehensive documentation including:
  - Table of Contents
  - Configuration examples for each source type
  - Docker and Docker Compose examples
  - Kubernetes deployment guide
  - API documentation
  - Environment Variables section
  - Per-source configuration documentation
  - Link Template Variables documentation
  - Caching configuration section
  - Display Configuration section
  - Updated Features list
  - Tests section

### Changed
- **Configuration precedence**: Clarified that environment variables override config file settings
- **Documentation**: Updated all documentation to reflect new features

### Fixed
- **ConfigMap path reference**: Fixed Kubernetes manifests path in README (manifests are at root, not in `k8s/` directory)
- **API status values**: Fixed API status values documentation in README

## [0.2.0] - 2024-01-10

### Added
- **11 Major Enhancements**:
  1. **Environment Variables**: Added support for:
     - `ALERTVIEW_PORT`
     - `ALERTVIEW_REFRESH_INTERVAL`
     - `ALERTVIEW_CACHE_TTL`
     - `ALERTVIEW_LOG_FORMAT`
  2. **Per-source Timeout**: Added `timeout` field to `Source` struct (default: 15 seconds)
  3. **Structured Logs**: Added JSON log format support via `tracing-subscriber`
  4. **Gzip Compression**: Added compression support using `tower-http` with compression-gzip feature
  5. **Caching**: Added in-memory cache with configurable TTL (per-source and global)
  6. **Retry Logic**: Implemented exponential backoff with configurable:
     - `max_retries` (default: 3)
     - `initial_delay_ms` (default: 1000)
     - `max_delay_ms` (default: 10000)
  7. **Link Templates**: Added `link_template` field with variable substitution:
     - `{{.Labels.x}}` for label values
     - `{{.Annotations.x}}` for annotation values
     - `{{.Source}}` for source name
     - `{{.Id}}` for alert ID
  8. **Sound Notifications**: Added Web Audio API sound notifications in `static/app.js`
     - Configurable via `play_sounds` in display config
     - Different sounds for different severities
  9. **Customizable Theme**: Added `theme` field in `DisplayConfig`:
     - Values: "dark", "light", "auto", or custom CSS URL
  10. **Timezone Support**: Added `timezone` field in `DisplayConfig`:
      - Values: "UTC", "local", or any IANA timezone
  11. **Unit Tests**: Added 13 comprehensive tests:
      - 8 tests in `src/config.rs`
      - 5 tests in `src/alerts.rs`
- **Health Check Endpoint**: Added `/health` route for monitoring
- **Updated config.example**: Added comments and examples for all new configuration options

### Changed
- **Dependencies**: Added `notify`, `notify-debouncer-mini`, `tower-http`, `chrono` with serde feature
- **Dependencies**: Removed `backoff` crate (using custom retry implementation)
- **AppState**: Modified to include `cache` field
- **API Response**: Added `timezone`, `theme`, `play_sounds` to response

### Fixed
- All tests passing (13 tests)
- Configuration validation improved

## [0.1.0] - 2024-01-05

### Added
- **Core Functionality**:
  - Alert fetching from Alertmanager, Grafana, and Zabbix
  - Alert transformation to common format
  - Web UI for displaying alerts
  - Configuration file support (YAML)
  - Command line arguments
- **Auto-Reload Configuration**:
  - File watcher using `notify` and `notify-debouncer-mini`
  - 500ms debounce for file changes
  - Thread-safe config access using `Arc<RwLock<Config>>`
  - Async config loading with `load_async()` method
- **Background Tasks**:
  - Config file watcher (`start_config_watcher()`)
  - Graceful shutdown handling
- **Docker Support**:
  - Dockerfile for building images
  - Multi-arch support (amd64, arm64)
- **Kubernetes Support**:
  - Deployment, Service, and Ingress manifests
  - ConfigMap support
- **Initial Documentation**:
  - README.md with basic information
  - config.example with example configuration

### Changed
- **Architecture**: Moved from synchronous to asynchronous processing
- **Configuration**: Improved config structure with defaults

### Fixed
- Initial release - all core functionality working

## [0.0.1] - 2024-01-01

### Added
- Project initialization
- Basic Rust project structure
- Initial Cargo.toml with dependencies
- Placeholder files for main components

[Unreleased]: https://github.com/your-org/alertview/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/your-org/alertview/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/your-org/alertview/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/your-org/alertview/compare/v0.0.1...v0.1.0
[0.0.1]: https://github.com/your-org/alertview/releases/tag/v0.0.1
