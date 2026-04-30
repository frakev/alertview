# AlertView Features

AlertView is a lightweight, real-time alert dashboard designed for monitoring and managing alerts from multiple sources. Here's a comprehensive list of all features:

## Core Features

### 📡 Alert Aggregation
- **Multi-Source Support**: Aggregate alerts from Alertmanager, Grafana, and Zabbix
- **Unified View**: All alerts displayed in a single, consistent interface
- **Real-time Updates**: Automatic refresh with configurable interval
- **Server-Sent Events (SSE)**: Real-time push notifications for new alerts via `/events` endpoint
- **Desktop Notifications**: Browser-based notifications for new alerts (requires permission)
- **Source Status**: Visual indication of each source's health

### 🎨 User Interface
- **Severity Coloring**: Color-coded cards (critical, high, warning, info)
- **Responsive Design**: Works on desktop, tablet, and mobile
- **Dark/Light Theme**: Built-in themes with custom CSS support
- **TV Mode**: Full-screen display for wall monitors
- **Sound Notifications**: Audio alerts for new alerts (configurable per severity)

### 🔍 Filtering & Search
- **Severity Filter**: Filter by critical, high, warning, info, or none
- **Status Filter**: Filter by firing, silenced, or pending
- **Source Filter**: Filter by specific alert sources
- **Text Search**: Search across all alert fields
- **URL-Persisted Filters**: Filters are preserved in the URL for sharing

### 🔗 Alert Links
- **Direct Links**: One-click access to alerts in their source systems
- **Custom Templates**: Configure custom link URLs with template variables
- **Dashboard Links**: Links to your monitoring dashboards
- **Smart Fallbacks**: Multiple fallback options for link generation

## Advanced Features

### ⚙️ Configuration
- **YAML Configuration**: Simple, human-readable configuration files
- **Environment Variables**: All major settings can be set via environment variables
- **Auto-Reload**: Configuration files are automatically reloaded when changed
- **Per-Source Settings**: Individual timeout, retry, and link settings per source
- **Display Customization**: Configure which labels to display
### 🔄 Resilience
- **Retry Logic**: Exponential backoff for failed requests
- **Timeout Configuration**: Configurable timeouts per source
- **Error Handling**: Graceful degradation when sources are unavailable
- **Caching**: Optional response caching to reduce load on sources
- **Health Checks**: `/health` endpoint for monitoring AlertView itself

### 📊 Observability
- **Structured Logging**: JSON or text log format
- **Log Levels**: Configurable log verbosity
- **Debug Mode**: Detailed logging for troubleshooting

### 🎯 Alert Management

#### Alert Grouping ✅
- Group alerts by label (namespace, job, etc.)
- Collapsible groups for better organization
- Configure via `group_by` in display configuration

## Technical Features

### Performance
- **Rust-Based**: High performance, low memory usage
- **Async I/O**: Non-blocking requests to alert sources
- **Parallel Fetching**: Simultaneous requests to all sources
- **Gzip Compression**: Compressed API responses
- **Efficient Caching**: Memory-efficient caching system

### Security
- **No Database**: No external dependencies, no data persistence
- **Read-Only**: Only reads from alert sources, never writes
- **TLS Support**: Secure connections to alert sources
- **Configurable TLS**: Option to skip TLS verification for self-signed certs

### Deployment
- **Single Binary**: Easy to deploy anywhere
- **Docker Support**: Official Docker images available
- **Kubernetes Ready**: Manifests included for easy deployment
- **Multi-Arch**: Support for x86_64, arm64, and more
- **No Dependencies**: Zero runtime dependencies

## Comparison with Alternatives

| Feature | AlertView | Grafana | Alertmanager UI |
|---------|-----------|---------|-----------------|
| Multi-Source | ✅ Yes | ✅ Yes | ❌ No |
| Real-time | ✅ Yes | ✅ Yes | ✅ Yes |
| Customizable | ✅ Highly | ✅ Medium | ❌ No |
| Lightweight | ✅ Very | ❌ Heavy | ✅ Very |
| No Database | ✅ Yes | ❌ No | ✅ Yes |
| TV Mode | ✅ Yes | ❌ No | ❌ No |
| Sound Notifications | ✅ Yes | ❌ No | ❌ No |
| Plugin System | ❌ No | ✅ Yes | ❌ No |
| Alert Grouping | ✅ Yes | ✅ Yes | ❌ No |
| Alert Actions | ❌ No | ✅ Yes | ❌ No |

## Use Cases

### 🏠 Home Lab
- Monitor your personal servers and services
- Simple setup with Docker
- No complex infrastructure required

### 🏢 Small Team
- Centralized alert viewing for multiple services
- Easy to configure and maintain
- Affordable (no licensing costs)

### 🏭 Enterprise
- Wall-mounted display for NOC/SOC
- Complement to existing monitoring tools
- Customizable for specific workflows

### 🚀 Development
- Local development of alerting systems
- Testing alert configurations
- Debugging alert rules

## Contributing

Check out our [GitHub Issues](https://github.com/frakev/alertview/issues) for feature requests and bug reports.
Contributions are welcome! See [Contributing Guide](../development/contributing.md) for details.
