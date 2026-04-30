# Project Structure

This document describes the structure and organization of the AlertView project.

## Directory Layout

```
alertview/
├── src/                          # Rust source code
│   ├── main.rs                   # Application entry point and HTTP server
│   ├── config.rs                 # Configuration loading, parsing, and validation
│   ├── alerts.rs                 # Alert fetching, processing, and transformation
│   ├── cache.rs                  # In-memory caching implementation
│   └── lib.rs                    # Library exports (if applicable)
│
├── static/                       # Static web assets
│   ├── index.html                # Main HTML page
│   ├── app.js                    # Frontend JavaScript application
│   └── style.css                 # Stylesheet
│
├── docs/                         # Documentation
│   ├── README.md                 # Documentation table of contents
│   ├── getting-started/          # Getting started guides
│   ├── configuration/            # Configuration documentation
│   ├── deployment/               # Deployment guides
│   └── development/              # Development documentation
│
├── k8s/                         # Kubernetes manifests
│   ├── deployment.yaml           # Deployment configuration
│   ├── service.yaml              # Service configuration
│   └── ingress.yaml              # Ingress configuration
│
├── .github/                      # GitHub configuration
│   └── workflows/                # GitHub Actions workflows
│       └── ci.yml                # CI/CD pipeline
│
├── Cargo.toml                    # Rust package manifest
├── Cargo.lock                    # Dependency lock file
├── config.example                # Example configuration file
├── README.md                     # Project README
└── .gitignore                    # Git ignore patterns
```

## Source Code Structure

### main.rs

The main entry point contains:

- **Application initialization**: Logging setup, configuration loading
- **HTTP server setup**: Axum router configuration, middleware
- **State management**: `AppState` struct with shared configuration and cache
- **API endpoints**: Route handlers for `/api/alerts`, `/health`, etc.
- **Background tasks**: Config file watcher, cache cleanup
- **Main function**: Entry point that starts the server

**Key components:**

```rust
// AppState holds shared application state
pub struct AppState {
    pub config: SharedConfig,      // Arc<RwLock<Config>>
    pub cache: Cache,              // Alert cache
}

// Router configuration
let app = Router::new()
    .route("/api/alerts", get(get_alerts))
    .route("/health", get(health_check))
    .route("/", get(serve_index))
    .with_state(state)
    .layer(Extension(state.config.clone()))
    .layer(CompressionLayer::new());
```

### config.rs

Handles all configuration-related functionality:

- **Config struct**: Main configuration structure with all fields
- **Source struct**: Datasource configuration (Alertmanager, Grafana, Zabbix)
- **DisplayConfig struct**: UI-related configuration
- **RetryPolicy struct**: Retry logic configuration
- **Loading functions**: `load()`, `load_async()`, `load_from_env()`
- **Validation**: Config validation and default values
- **File watching**: Automatic reload on file changes
- **Tests**: Configuration parsing and validation tests

**Key structs:**

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub sources: Vec<Source>,
    pub display: DisplayConfig,
    pub cache_ttl: Option<u64>,
    pub log_format: String,
    pub refresh_interval: Option<u64>,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Source {
    pub name: String,
    pub kind: SourceKind,          // alertmanager, grafana, zabbix
    pub url: String,
    pub timeout: Option<u64>,
    pub link_template: Option<String>,
    pub retry_policy: Option<RetryPolicy>,
    // ... kind-specific fields
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}
```

### alerts.rs

Handles alert fetching and processing:

- **Alert struct**: Represents a single alert
- **AlertGroup struct**: Group of related alerts
- **Fetching functions**: `fetch_source_alerts()`, `fetch_with_retry()`
- **Transformation**: Convert source-specific formats to common Alert format
- **Link templates**: `apply_link_template()` for custom alert links
- **Grouping**: Group alerts by labels (future enhancement)
- **Tests**: Alert processing and template tests

**Key functions:**

```rust
// Fetch alerts from a source with retry logic
pub async fn fetch_source_alerts_with_retry(
    source: &Source,
    client: &Client,
) -> Result<Vec<Alert>, AlertError> { ... }

// Apply link template to an alert
pub fn apply_link_template(template: &str, alert: &Alert) -> String { ... }

// Transform Alertmanager alerts to common format
pub fn transform_alertmanager_alerts(alerts: &[AlertmanagerAlert]) -> Vec<Alert> { ... }
```

### cache.rs

Implements in-memory caching:

- **Cache struct**: Thread-safe alert cache
- **CacheEntry struct**: Cached data with expiration
- **Operations**: `get()`, `set()`, `invalidate()`, `cleanup()`
- **TTL handling**: Automatic expiration based on TTL
- **Background cleanup**: Periodic cleanup of expired entries

**Key implementation:**

```rust
#[derive(Debug, Clone)]
pub struct Cache {
    inner: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: Vec<Alert>,
    pub expires_at: DateTime<Utc>,
}

impl Cache {
    pub fn get(&self, key: &str) -> Option<Vec<Alert>> { ... }
    pub fn set(&self, key: String, data: Vec<Alert>, ttl: u64) { ... }
    pub fn invalidate(&self, key: &str) { ... }
    pub fn cleanup(&self) { ... }
}
```

## Frontend Structure

### index.html

The main HTML page contains:

- **Meta tags**: Viewport, charset, title
- **CSS imports**: Internal styles and external CDN links
- **Body structure**: Header, main content area, footer
- **Root element**: `<div id="app">` for JavaScript mounting
- **Script imports**: `app.js` and dependencies

### app.js

The frontend application handles:

- **State management**: `TV` global object with alerts, config, filters
- **Data fetching**: `fetchAlerts()` with error handling
- **UI updates**: `updateUI()` to refresh the display
- **Notifications**: `sendNotif()` for browser notifications
- **Sound alerts**: `playSoundForAlerts()` with Web Audio API
- **Time handling**: `absTime()` with timezone support
- **Filtering**: Filter alerts by severity, source, text
- **Sorting**: Sort alerts by time, severity, etc.
- **Auto-refresh**: Periodic refresh based on config

**Key objects:**

```javascript
// TV - Main application state
const TV = {
    alerts: [],           // Current alerts
    config: {},           // Current configuration
    filters: {},          // Current filters
    sort: {},             // Current sort options
    // ... methods
};

// SOUND_PRESETS - Sound configurations for different severities
const SOUND_PRESETS = {
    critical: { frequency: 800, duration: 0.5 },
    warning: { frequency: 400, duration: 0.3 },
    // ...
};
```

### style.css

Contains all styles for the application:

- **Variables**: CSS custom properties for theming
- **Reset**: Basic CSS reset
- **Layout**: Grid and flexbox layouts
- **Components**: Alert cards, filters, header, etc.
- **Responsive**: Media queries for different screen sizes
- **Animations**: Transitions and animations
- **Themes**: Dark and light theme styles

## Configuration Files

### Cargo.toml

Rust package manifest with:

- **Package metadata**: Name, version, authors, description
- **Dependencies**: All Rust crates with versions and features
- **Features**: Optional features (e.g., `gzip`, `json-logs`)
- **Build configuration**: Targets, profiles

**Key sections:**

```toml
[package]
name = "alertview"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
reqwest = { version = "0.11", features = ["json"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
notify = "6.1"
notify-debouncer-mini = "0.4"
tower-http = { version = "0.5", features = ["compression-gzip"] }
chrono = { version = "0.4", features = ["serde"] }

[features]
default = []
gzip = ["tower-http/compression-gzip"]
```

### config.example

Example configuration file showing all available options with comments. Used as:
- Documentation for users
- Template for creating new configs
- Reference for all config fields

## Build Output

### target/debug/

Debug build artifacts:
- `alertview`: Debug binary with symbols
- `alertview.d`: Debug information
- `.pdb` files on Windows

### target/release/

Release build artifacts:
- `alertview`: Optimized binary
- Stripped of debug symbols
- Smaller file size

### target/doc/

Generated Rust documentation:
- HTML documentation for all crates
- Generated by `cargo doc`

## GitHub Workflows

### .github/workflows/ci.yml

CI/CD pipeline with:

- **Build**: Check compilation on multiple platforms
- **Test**: Run all tests
- **Lint**: Run clippy for code quality
- **Format**: Check code formatting
- **Release**: Build and publish release artifacts

**Typical workflow:**

```yaml
name: CI

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
      - run: cargo build
      - run: cargo test
      - run: cargo clippy
      - run: cargo fmt --check

  release:
    needs: build
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
      - run: cargo build --release
      - uses: actions/upload-artifact@v3
        with:
          name: alertview-release
          path: target/release/alertview
```

## Module Dependencies

```
┌─────────────────────────────────────────────────────────┐
│                      main.rs                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  config.rs  │  │  alerts.rs  │  │      cache.rs        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────┐
│                    External Dependencies                   │
│  axum, tokio, serde, reqwest, tracing, notify, tower-http  │
└─────────────────────────────────────────────────────────┘
```

- `main.rs` imports and uses functions from `config.rs`, `alerts.rs`, and `cache.rs`
- Each module is self-contained with its own tests
- Shared types are defined in the module that uses them most

## Data Flow

```
┌──────────┐     ┌──────────────┐     ┌──────────────┐
│  Config  │────▶│   Sources    │────▶│   Alerts     │
│  File    │     │  Definition  │     │  Fetching    │
└──────────┘     └──────────────┘     └──────────────┘
                                                       │
                                                       ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Transform   │────▶│    Cache     │────▶│   API        │
│  to Common   │     │  (Optional)  │     │  Response    │
└──────────────┘     └──────────────┘     └──────────────┘
                                                       │
                                                       ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   HTTP       │────▶│   Frontend    │────▶│   Display    │
│   Server     │     │   (Browser)  │     │   Alerts     │
└──────────────┘     └──────────────┘     └──────────────┘
```

1. Configuration is loaded from file or environment
2. Sources are defined in the config
3. Alerts are fetched from each source
4. Alerts are transformed to a common format
5. Alerts are optionally cached
6. Alerts are returned via API
7. Frontend fetches and displays alerts

## File Size Guidelines

| File | Recommended Max Size | Current Size |
|------|---------------------|--------------|
| main.rs | 500 lines | ~400 lines |
| config.rs | 800 lines | ~600 lines |
| alerts.rs | 800 lines | ~500 lines |
| cache.rs | 300 lines | ~200 lines |
| app.js | 1500 lines | ~1200 lines |
| style.css | 1000 lines | ~800 lines |

If a file grows beyond these guidelines, consider splitting it into smaller modules.

## Naming Conventions

### Rust

- **Types**: `PascalCase` (e.g., `Alert`, `Config`, `AppState`)
- **Functions**: `snake_case` (e.g., `fetch_alerts`, `load_config`)
- **Variables**: `snake_case` (e.g., `config_path`, `alert_list`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_PORT`, `MAX_RETRIES`)
- **Modules**: `snake_case` (e.g., `config.rs`, `alerts.rs`)

### JavaScript

- **Variables**: `camelCase` (e.g., `currentAlerts`, `configData`)
- **Functions**: `camelCase` (e.g., `fetchAlerts`, `updateUI`)
- **Constants**: `UPPER_CASE` (e.g., `SOUND_PRESETS`, `DEFAULT_CONFIG`)
- **Objects**: `PascalCase` for constructors (e.g., `Alert`, `Source`)

### CSS

- **Classes**: `kebab-case` (e.g., `.alert-card`, `.severity-critical`)
- **IDs**: `kebab-case` (e.g., `#app`, `#alerts-container`)
- **Variables**: `--kebab-case` (e.g., `--primary-color`, `--alert-bg`)

## Error Handling

- Use Rust's `Result` and `Option` types appropriately
- Return meaningful error types (not just `String`)
- Log errors with appropriate levels (debug, info, warn, error)
- Provide user-friendly error messages in the UI

## Testing

- Unit tests in the same file as the code (module-level)
- Integration tests in the `tests/` directory
- Test both happy paths and error cases
- Use descriptive test names

## Documentation

- Use Rustdoc comments (`///`) for public items
- Document all public functions, structs, and enums
- Include examples in documentation where helpful
- Keep documentation in sync with code changes
