# AlertView Architecture

This document describes the internal architecture of AlertView, helping you understand how it works under the hood.

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        AlertView                               │
├─────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐  │
│  │   Source 1  │    │   Source 2  │    │   Source N     │  │
│  │ (Alertmgr) │    │  (Grafana)  │    │   (Zabbix)     │  │
│  └──────┬──────┘    └──────┬──────┘    └────────┬────────┘  │
│         │                  │                   │            │
│         ▼                  ▼                   ▼            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    HTTP Client                        │   │
│  │  (with timeout, retry, caching)                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    Alert Processing                   │   │
│  │  - Normalization (across sources)                     │   │
│  │  - Severity mapping                                   │   │
│  │  - Status determination                               │   │
│  │  - Link generation                                     │   │
│  │  - Template rendering                                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                      Cache                              │   │
│  │  (optional, configurable TTL)                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    API Server                           │   │
│  │  - REST API (/api/alerts)                              │   │
│  │  - Web UI (/, /style.css, /app.js)                     │   │
│  │  - Health check (/health)                              │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    Web UI                               │   │
│  │  - React-like state management (vanilla JS)           │   │
│  │  - Real-time updates                                   │   │
│  │  - Filtering & search                                  │   │
│  │  - TV mode                                            │   │
│  │  - Sound notifications                                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Configuration Layer

**Location:** `src/config.rs`

**Responsibilities:**
- Load and parse YAML configuration files
- Provide default values
- Support environment variable overrides
- Automatic reload on file changes
- Validate configuration

**Key Structures:**
```rust
pub struct Config {
    pub port: u16,
    pub refresh_interval: u64,
    pub tls_insecure: bool,
    pub cache_ttl_seconds: u64,
    pub log_format: String,  // "text" or "json"
    pub sources: Vec<Source>,
    pub display: DisplayConfig,
}

pub struct Source {
    pub name: String,
    pub source_type: SourceType,  // Alertmanager, Grafana, Zabbix
    pub url: String,
    pub dashboard_url: Option<String>,
    pub link_template: Option<String>,
    pub basic_auth: Option<BasicAuth>,
    pub bearer_token: Option<String>,
    pub timeout: u64,
    pub retry_policy: RetryPolicy,
}
```

### 2. Alert Fetching Layer

**Location:** `src/alerts.rs`

**Responsibilities:**
- Fetch alerts from various sources
- Normalize alert data across sources
- Apply link templates
- Handle errors gracefully
- Support timeouts and retries

**Supported Sources:**
- **Alertmanager**: Standard Alertmanager API v2
- **Grafana**: Grafana's Alertmanager-compatible API
- **Zabbix**: Zabbix JSON-RPC API

**Fetch Process:**
1. Create HTTP client with source-specific timeout
2. Apply authentication (basic auth or bearer token)
3. Make request with timeout
4. Parse response
5. Normalize alert data
6. Apply link templates
7. Return normalized alerts

### 3. Caching Layer

**Implementation:** In-memory HashMap with TTL

**Features:**
- Configurable TTL (0 = disabled)
- Per-source caching
- Automatic cache invalidation
- Memory-efficient storage

**Cache Key:** Generated from source name, URL, and type

### 4. API Layer

**Framework:** Axum (Rust web framework)

**Endpoints:**
| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Web UI dashboard |
| GET | `/api/alerts` | JSON API - all alerts |
| GET | `/health` | Health check |
| GET | `/style.css` | Dashboard styles |
| GET | `/app.js` | Dashboard JavaScript |

**Middleware:**
- Gzip compression (tower-http)
- State management (Arc<AppState>)
- Error handling

### 5. Web UI Layer

**Technologies:** Vanilla HTML/CSS/JavaScript

**Features:**
- Single-page application
- Real-time updates via polling
- State persistence in localStorage
- URL-persisted filters
- Responsive design
- TV mode
- Sound notifications (Web Audio API)

**State Management:**
```javascript
const App = {
  data: null,           // Current alerts and sources
  knownFps: Set,        // Known alert fingerprints
  freshFps: Set,        // New alerts since last fetch
  searchQ: string,      // Search query
  sevFilter: string,    // Severity filter
  srcFilter: Set,       // Source filter
  showSilenced: bool,   // Show silenced alerts
  // ...
};
```

### 6. Config Watcher

**Library:** notify-debouncer-mini

**Features:**
- File system watching
- Debouncing (500ms delay)
- Automatic config reload
- Background task (tokio::spawn)
- Thread-safe config updates (RwLock)

## Data Flow

### Alert Fetch Flow

```
1. User requests /api/alerts
   │
2. Check cache (if enabled)
   │──► Cache hit? Return cached alerts
   │
3. For each source:
   │
   ├─► Check cache for this source
   │   │──► Cache hit? Use cached alerts
   │
   ├─► Create HTTP client with source timeout
   │
   ├─► Apply retry logic:
   │   │──► Attempt 1: Immediate
   │   │──► Attempt 2: After initial_delay_ms
   │   │──► Attempt 3: After 2 * initial_delay_ms
   │   │──► ... (up to max_retries)
   │   │──► Give up after max_delay_ms total
   │
   ├─► Fetch alerts from source
   │
   ├─► Normalize alert data
   │
   ├─► Apply link templates
   │
   └─► Cache results (if enabled)

4. Aggregate all alerts

5. Sort by severity and time

6. Return JSON response
```

### Config Reload Flow

```
1. File system detects change to config.yaml
   │
2. Debouncer waits 500ms (to avoid multiple reloads)
   │
3. If still changed after 500ms:
   │
   ├─► Load new config asynchronously
   │
   ├─► Acquire write lock on config
   │
   ├─► Replace old config with new config
   │
   ├─► Release lock
   │
   └─► Log success/failure

4. Next API request uses new config
```

## Performance Considerations

### Memory Usage
- **Alerts**: Stored in memory (Vec<Alert>)
- **Cache**: Optional, TTL-based, per-source
- **Config**: Single instance with RwLock
- **HTTP Client**: Connection pooling enabled

### CPU Usage
- **Fetching**: Parallel requests to all sources
- **Processing**: Minimal (normalization, sorting)
- **Rendering**: Client-side (no server-side rendering)

### Network Usage
- **Polling**: Configurable interval (default: 30s)
- **Compression**: Gzip for all responses
- **Caching**: Reduces requests to sources

## Security Considerations

### Data Flow
- **Read-Only**: AlertView only reads from sources, never writes
- **No Persistence**: No database, no data stored between restarts
- **In-Memory Only**: All data is in memory (RAM)

### Authentication
- **Source Auth**: Basic auth and bearer tokens supported
- **TLS**: Secure connections with configurable verification
- **No User Auth**: Currently no authentication for the UI

### Isolation
- **Per-Source**: Each source has its own configuration
- **Timeouts**: Prevents hanging requests
- **Retries**: Configurable per source

## Extensibility

### Current Architecture
- **Monolithic**: All functionality in main binary
- **Modular Code**: Separate modules for config, alerts, etc.
- **Public Modules**: `config` and `alerts` are public for testing

## Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Language | Rust | Core language |
| Runtime | Tokio | Async runtime |
| Web Framework | Axum | HTTP server |
| HTTP Client | Reqwest | HTTP requests |
| Config | Serde + serde_yaml | Configuration parsing |
| Logging | Tracing | Structured logging |
| Compression | Tower-HTTP | Gzip compression |
| File Watching | notify-debouncer-mini | Config file watching |
| UI | Vanilla HTML/CSS/JS | Frontend |
| Build | Cargo | Rust package manager |
| CI/CD | GitHub Actions | Continuous integration |

## Directory Structure

```
alertview/
├── Cargo.toml           # Project configuration
├── Cargo.lock           # Dependency lock file
├── src/
│   ├── main.rs          # Entry point, API server
│   ├── config.rs        # Configuration loading and parsing
│   ├── alerts.rs        # Alert fetching and processing
│   └── static/          # Static files (HTML, CSS, JS)
│       ├── index.html
│       ├── style.css
│       └── app.js
├── config.example       # Example configuration
├── 01-namespace.yaml    # Kubernetes namespace manifest
├── 02-configmap.yaml    # Kubernetes ConfigMap manifest
├── 03-deployment.yaml   # Kubernetes Deployment manifest
├── 04-service.yaml      # Kubernetes Service manifest
├── 05-ingress.yaml      # Kubernetes Ingress manifest
├── Dockerfile           # Docker build configuration
├── README.md           # Main readme
├── CHANGELOG.md        # Change log
└── docs/               # Documentation (this directory)
    ├── getting-started/
    ├── configuration/
    ├── deployment/
    ├── development/
    └── examples/
```

## Contributing

If you want to contribute to AlertView's architecture:

1. **Understand the Code**: Read through `src/main.rs`, `src/config.rs`, and `src/alerts.rs`
2. **Follow Patterns**: Use the existing patterns for consistency
3. **Add Tests**: Add tests for new functionality
4. **Update Docs**: Update this architecture document when making significant changes

See [Development Guide](../development/README.md) for more details.
