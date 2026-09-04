# Changelog

All notable changes to AlertView are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.1] - 2026-09-04

### Changed
- **Severity and status badges come before the alert text** — they used to follow it, so their position depended on the length of the name; they now sit in a fixed column right after the prefix labels, in TV rows and on cards alike, and severity reads straight down a column.

## [0.8.0] - 2026-09-04

### Changed
- **TV rows line up in columns** — each row was its own flex line, so every column started wherever the previous element happened to end: with hostnames of different lengths the message, the badges and the labels were ragged from one row to the next. The alert list is now a CSS grid and each row a `subgrid` of it, so a column is as wide as the widest cell across all rows and the tracks size to their content, capped with `fit-content()` so one very long hostname cannot starve the message. No hardcoded widths — unlike the 0.5.3 attempt (`7rem 4.5rem 10rem`) that was reverted in 0.5.4. The age is right-aligned so `for 4m` and `for 28m` line up on their digits. Every row emits the same nine slots even when empty, which is what keeps the columns in step; a browser without `subgrid` support falls back to the previous flex layout untouched.

### Documentation
- **`config.example` rewritten as a complete, organised reference** — options had been appended as they were added, leaving the file unsorted and partly stale: a "Global timeout for all sources" comment sitting above `tls_insecure` (there is no such setting), a Zabbix `link_template` still using the `filter_eventid` parameter dropped back in 0.4, and `show_labels` / `show_alert_name` descriptions predating the reveal button. It is now grouped into Server / Sources / Display sections, every per-source option is documented on the first source, and the file ends with the full list of environment variables and of the URL parameters. All 42 configuration fields are covered.
- The two `filter_eventid` mentions in the README were replaced by how Zabbix links are actually built, and `docs/configuration/config-file.md` gained the per-source options missing from its inline schema.

## [0.7.1] - 2026-09-04

### Added
- **A button to reveal what the config hides** — `display.show_labels: false` and `display.show_alert_name: false` no longer make things simply disappear: each alert carries a small `+N` button bringing back its labels and, when the summary took its place, its name as an `alertname=…` chip. One button covers both, on cards as well as TV rows, and it reuses the toggle TV rows already had for the labels they have no space for. The open state is now kept per alert rather than in the DOM, so it survives the auto-refresh.

### Fixed
- **TV rows were laid out with two gaps** — with `display.show_alert_name: false` the summary took the place of the alert name and both it and the (now empty) `.row-summary` grew to fill the row, so the free space was split in two and the severity badges ended up stranded in the middle. The summary now shrinks but never grows, leaving a single spacer, and the badges sit right after the text as they do in every other layout.
- **Prefix labels were truncated too early** — the prefix was capped at 22 characters, which cut a row like `top1-mon-1 / coreiaas / top1-…` short even on a wide screen. TV rows show it in full and let it shrink (before the alert text does) only when the row actually runs out of room; cards keep a cap, raised to 40 characters. Its font size also matches the alert text now.

## [0.7.0] - 2026-09-04

### Changed
- **Sources are fetched concurrently** — `/api/alerts` used to take the sum of every source's latency; it now takes the slowest one (8 in flight at a time, results kept in config order). Measured with three sources answering in 2s: 2.0s instead of ~6s.
- **The config lock is no longer held across the network I/O** — the handler snapshots the config and releases the read guard before fetching, so a config reload no longer waits for the slowest source.
- **Groups no longer duplicate the alert payload** — `AlertGroup.alerts` repeated every alert already present in `alerts`, doubling the response when `group_by` was set. The frontend never read it: it picks the members out of the main list.
- **Groups are ordered by severity** — most severe group first instead of alphabetically by key, so on a wall display the team with a critical is at the top.
- **Sorting does less work** — the severity rank is computed once per alert instead of on every comparison, on both sides.
- **`display.source_link` and the SSE payload agree** — new alerts are broadcast after the config-wide link settings are applied, so an SSE payload carries the same alert as `/api/alerts`.
- **URL parameters no longer stick** — `?tv=1`, `?sev=`, `?src=` and `?silenced=` used to be written to `localStorage`, so opening a shared link once pinned that setting in the visitor's browser for good. They now apply to that visit only.
- **CI** — a workflow now runs `cargo clippy -- -D warnings`, `cargo test` and a syntax check of the static assets on every push and pull request; nothing but tag builds ran before. Two integration tests were added against a stub Alertmanager, covering the failures that actually happened: the API path being dropped from the source URL, `silencedBy` not deserialising, a `javascript:` generator URL reaching the frontend, and a 404 surfacing as a typed status error. `release.yml` no longer uses the archived `actions-rs/toolchain`.

### Fixed
- **The theme picked by the user was overwritten 30s later** — `display.theme` was re-applied on the next poll because the "the user chose this" flag was computed once at startup and never updated when the theme button was clicked.
- **Grouping hid alerts** — an alert missing the grouping label was placed in a `<missing>` group the frontend could never match, and a label value containing `,` or `=` scrambled the group key when it was re-parsed. Membership now comes from the labels the server sends. Measured on 6 alerts grouped by `team`: 2 were invisible and 2 were shown twice; all 6 now land in the right group.
- **A blocked storage made the page blank** — `localStorage` raises rather than returning null in private browsing or with site data blocked, and it was read unguarded at startup, taking the whole script down. All access goes through guarded helpers.
- **Clearing the search left `?q=` in the URL** — with label filters in the box, a reload silently brought the filter back.
- **Exponential backoff was never capped** — `max_delay_ms` was applied to the multiplier instead of the delay.
- **408 and 429 are retried again** — the "don't retry 4xx" rule covered them, although both mean "try again later" rather than "you are misconfigured".
- **`ALERTVIEW_CONFIG` is read** — it was documented in `--help` but never looked at. `--config <path>` is accepted as well.
- **Duplicate source names are rejected at startup** — they silently shared a cache key, an announced-fingerprint entry and a filter chip.
- **Inhibited alerts are labelled as such** — an alert suppressed by another alert (`inhibitedBy`) showed up as silenced with no comment, indistinguishable from a real silence.
- **HTML injection through the `severity` label** — the severity was interpolated raw into a CSS class, a chip's text and an inline `onclick`, so an alert carrying a crafted `severity` label (`x" onmouseover="alert(1)`) broke out of the attribute and injected a handler. Severity now goes through `esc()` for display and through a slug for the CSS class token. Same defect for source names and group keys, which broke the inline handler on any value containing a quote: the chips and group headers no longer carry inline `onclick` attributes at all, the values travel in `data-*` and one delegated listener per container handles the click.
- **Silence comments were never displayed** — `AmStatus.silenced_by` was missing its `#[serde(rename = "silencedBy")]`, so the field Alertmanager actually sends never deserialized and the silence lookup always came up empty. Silenced alerts now show the silence comment again.
- **A long summary broke the TV row layout** — with `show_alert_name: false` the summary landed in `.alert-name`, which has no flex or truncation, and pushed the trailing metadata out of the row. It now takes the free space and truncates with an ellipsis.

### Added
- **Label filters in the search box** — the search now accepts comma-separated `key=value` filters alongside free text: `team=sre, hostname~web`, `severity=critical, team!=dba`, or mixed with a plain search (`team=dba, slow queries`). `=` is an exact case-insensitive match, `!=` excludes (and matches alerts without the label), `~` means "contains". Keys are looked up in the alert's labels then its annotations, with `source`, `status`, `name`/`alertname` and `severity` also usable. Anything that is not a `key<op>value` pair stays free text, so the previous behaviour is unchanged. Filters live in `?q=` like before, so a per-team view can be bookmarked or put on a wall display.
- **`display.show_alert_name`** — set to false, the `summary` annotation takes the place of the `alertname` in the card title and the TV row, and is not repeated below. An alert without a summary keeps its name so a row is never blank.
- **`display.show_labels`** — hides the label chips (and the TV `+N` toggle) without having to empty `display.labels`. Prefix labels are unaffected.
- **`display.critical_icon`** — an icon (🔥 by default, `""` to disable) shown right before the name of critical alerts.
- **Version in TV mode** — the running version sits next to the TV clock, dimmed to 35% so it stays discreet on a wall display, and becomes fully readable when the HUD bar is hovered.

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
