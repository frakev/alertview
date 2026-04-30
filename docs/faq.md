# Frequently Asked Questions

## General Questions

### What is AlertView?

AlertView is a lightweight, web-based dashboard for viewing and managing alerts from multiple monitoring systems including Alertmanager, Grafana, and Zabbix.

### Why use AlertView?

- **Unified view**: See alerts from all your monitoring systems in one place
- **Lightweight**: Minimal resource usage, fast to deploy
- **Customizable**: Configure display, filtering, grouping, and more
- **Real-time**: Auto-refresh and sound notifications for new alerts
- **Easy to use**: Simple configuration, no database required

### Who is AlertView for?

AlertView is designed for:
- DevOps engineers
- SRE teams
- System administrators
- Anyone who needs to monitor multiple alerting systems

### What monitoring systems does AlertView support?

Currently supported:
- **Alertmanager** (Prometheus Alertmanager)
- **Grafana** (Grafana Alerting)
- **Zabbix** (Zabbix triggers)



### Is AlertView a replacement for Alertmanager/Grafana?

No. AlertView is a **viewer** for alerts, not a replacement for alerting systems. It:
- Reads alerts from existing systems
- Displays them in a unified interface
- Does not manage or route alerts

### Does AlertView store alerts?

No. AlertView:
- Fetches alerts from your monitoring systems on demand
- Optionally caches alerts in memory (configurable)
- Does not persist alerts to disk or database

For persistence, AlertView relies on your existing monitoring systems.

## Installation Questions

### How do I install AlertView?

See the [Getting Started](getting-started/quick-start.md) guide for installation options:
- **Binary**: Download pre-built binaries
- **Docker**: Use the Docker image
- **Cargo**: Install from source with `cargo install`
- **Kubernetes**: Deploy using the provided manifests

### What are the system requirements?

**Minimum:**
- 64-bit CPU
- 64 MB RAM
- 10 MB disk space

**Recommended:**
- Modern CPU
- 256 MB RAM
- 100 MB disk space

**Supported OS:**
- Linux (x86_64, ARM64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

### Do I need Rust to run AlertView?

No. You only need Rust if you want to:
- Build AlertView from source
- Contribute to the project

Pre-built binaries and Docker images are available for users.

### What Rust version is required to build AlertView?

AlertView requires Rust 1.75 or later.

Check your Rust version:
```bash
rustc --version
```

Update Rust:
```bash
rustup update stable
```

### How do I update AlertView?

**Binary:**
1. Download the new version
2. Replace the old binary
3. Restart AlertView

**Docker:**
```bash
docker pull ghcr.io/your-org/alertview:latest
docker stop alertview
docker rm alertview
docker run -d --name alertview ...
```

**Kubernetes:**
```bash
kubectl set image deployment/alertview alertview=ghcr.io/your-org/alertview:latest -n alertview
```

**Cargo:**
```bash
cargo install --git https://github.com/your-org/alertview.git --tag v1.0.0
```

## Configuration Questions

### Where should I put the configuration file?

Common locations:
- `/etc/alertview/config.yaml` (system-wide)
- `~/alertview/config.yaml` (user-specific)
- `./config.yaml` (current directory)

AlertView looks for configuration in this order:
1. Command line argument (`--config`)
2. Environment variable (`ALERTVIEW_CONFIG_PATH`)
3. Default locations (`/etc/alertview/config.yaml`, `./config.yaml`)

### Can I use environment variables for configuration?

Yes! Most configuration options can be set via environment variables:

```bash
# Server settings
export ALERTVIEW_PORT=8080
export ALERTVIEW_LOG_FORMAT=json

# Display settings
export ALERTVIEW_REFRESH_INTERVAL=30
export ALERTVIEW_THEME=dark
export ALERTVIEW_TIMEZONE=UTC

# Cache settings
export ALERTVIEW_CACHE_TTL=60

# Source-specific settings
export ALERTMANAGER_URL=http://localhost:9093
export GRAFANA_API_KEY=your-api-key
```

See [Environment Variables](configuration/environment-variables.md) for a complete list.

### Can I have multiple configuration files?

Not directly, but you can:
- Use environment variables to override specific settings
- Use command line arguments for the config file path
- Use symlinks to point to different config files

### How do I configure multiple alert sources?

Add multiple entries to the `sources` array:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
```

See [Multiple Sources](examples/multiple-sources.md) for more details.

### How do I filter alerts?

Use the `filters` option in the `display` section:

```yaml
display:
  filters:
    severity: [critical, warning]  # Only critical and warning
    state: [firing]               # Only firing alerts
    source: [alertmanager]        # Only from alertmanager
    team: [frontend, backend]     # Custom label filter
```

### How do I change the refresh interval?

Set the `refresh_interval` in the `display` section:

```yaml
display:
  refresh_interval: 30  # Refresh every 30 seconds
```

### How do I enable dark mode?

Set the `theme` in the `display` section:

```yaml
display:
  theme: dark  # or "light" or "auto"
```

### How do I change the timezone?

Set the `timezone` in the `display` section:

```yaml
display:
  timezone: America/New_York  # IANA timezone
  # or
  timezone: UTC
  # or
  timezone: local  # Use browser's timezone
```

### How do I enable sound notifications?

Set `play_sounds` to true:

```yaml
display:
  play_sounds: true
```

### How do I customize the alert link URLs?

Use `link_template` in each source configuration:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
```

See [Link Templates](configuration/config-file.md#link-templates) for more details.

### How do I configure caching?

Set `cache_ttl` globally or per-source:

```yaml
# Global cache (applies to all sources without explicit cache_ttl)
cache_ttl: 60  # Cache for 60 seconds

sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    cache_ttl: 30  # Override global for this source
```

### How do I configure retry logic?

Set `retry_policy` per-source:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    retry_policy:
      max_retries: 3          # Maximum number of retries
      initial_delay_ms: 1000  # Initial delay in milliseconds
      max_delay_ms: 10000    # Maximum delay in milliseconds
```

### How do I set timeouts?

Set `timeout` per-source:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    timeout: 30  # 30 second timeout
```

### How do I configure authentication?

**Alertmanager (Basic Auth):**
```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    basic_auth:
      username: api-user
      password: api-password
```

**Grafana (API Key):**
```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
```

**Zabbix (Username/Password):**
```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
```

### How do I configure TLS/HTTPS?

**Skip certificate verification (not recommended for production):**
```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    tls:
      skip_verify: true
```

**Custom CA certificate:**
```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    tls:
      ca_certificate: /path/to/ca.crt
```

## Usage Questions

### How do I run AlertView?

```bash
# With default config
alertview

# With custom config
alertview --config /path/to/config.yaml

# With custom port
alertview --port 9090

# With debug logging
RUST_LOG=debug alertview
```

### How do I access the web UI?

After starting AlertView, open your browser to:
```
http://localhost:8080
```

Or whatever port you configured.

### How do I view alerts from a specific source?

Use the source filter in the UI or via the API:

```bash
# Via API
curl "http://localhost:8080/api/alerts?source=alertmanager"
```

### How do I search for alerts?

Use the search box in the UI to search by:
- Alert name
- Labels
- Annotations
- Source

### How do I sort alerts?

Click on column headers to sort by that column. Configure default sort in the config:

```yaml
display:
  sort:
    by: starts_at  # or: severity, state, source, alertname
    order: desc    # or: asc
```

### How do I group alerts?

Configure grouping in the config:

```yaml
display:
  group_by: [alertname, service, team]
```

### How do I acknowledge alerts?

Acknowledgment is not currently supported in AlertView. You need to:
- Acknowledge in your monitoring system (Alertmanager, Grafana, Zabbix)
- Use the link template to open the alert in the source system

### How do I silence alerts?

Silencing is not currently supported in AlertView. You need to:
- Silence in your monitoring system
- Use the link template to open the alert in the source system

### How do I view alert history?

AlertView only shows current alerts (firing and recently resolved). For history:
- Use your monitoring system's history features
- Configure Alertmanager to keep resolved alerts
- Use Grafana's alert history

### How do I export alerts?

Use the API to export alerts:

```bash
# Get all alerts as JSON
curl http://localhost:8080/api/alerts > alerts.json

# Get alerts from specific source
curl "http://localhost:8080/api/alerts?source=alertmanager" > alertmanager-alerts.json
```

### How do I import alerts?

AlertView does not support importing alerts. It only reads from configured sources.

### How do I customize the UI?

You can customize:
- Theme (dark/light/auto)
- Timezone
- Columns (visibility and order)
- Colors (severity, state)
- Grouping
- Sorting
- Filters

For deeper customization, you can:
- Use custom CSS via the `theme` option
- Modify the static files and rebuild

### How do I disable auto-refresh?

Set `refresh_interval` to 0:

```yaml
display:
  refresh_interval: 0  # Disable auto-refresh
```

### How do I enable compact mode?

Set `compact_mode` to true:

```yaml
display:
  compact_mode: true
```

### How do I hide the header/footer?

```yaml
display:
  hide_header: true
  hide_footer: true
```

### How do I embed AlertView in another page?

Use an iframe:

```html
<iframe 
  src="http://alertview.example.com" 
  width="100%" 
  height="600px" 
  frameborder="0"
  style="border: none;"
></iframe>
```

Configure AlertView for embedding:

```yaml
display:
  refresh_interval: 0  # Disable auto-refresh (let parent page handle it)
  compact_mode: true
  hide_header: true
  hide_footer: true
```

## Deployment Questions

### How do I deploy AlertView with Docker?

See [Docker Deployment](deployment/docker.md) for details.

**Simple:**
```bash
docker run -d \
  --name alertview \
  -p 8080:8080 \
  -v /path/to/config.yaml:/etc/alertview/config.yaml:ro \
  ghcr.io/your-org/alertview:latest
```

**Docker Compose:**
```yaml
version: '3'

services:
  alertview:
    image: ghcr.io/your-org/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/alertview/config.yaml:ro
    environment:
      - RUST_LOG=info
```

### How do I deploy AlertView on Kubernetes?

See [Kubernetes Deployment](deployment/kubernetes.md) for details.

**Quick Start:**
```bash
# Create ConfigMap
kubectl create configmap alertview-config --from-file=config.yaml -n alertview

# Deploy
kubectl apply -f 01-namespace.yaml -f 02-configmap.yaml -f 03-deployment.yaml -f 04-service.yaml -f 05-ingress.yaml -n alertview
```

### How do I deploy AlertView behind a reverse proxy?

See [Reverse Proxy Configuration](deployment/reverse-proxy.md) for details.

**Nginx Example:**
```nginx
server {
    listen 80;
    server_name alertview.example.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### How do I secure AlertView?

Options for securing AlertView:

1. **Reverse proxy with authentication:**
   - Nginx basic auth
   - Apache basic auth
   - Traefik authentication

2. **Network restrictions:**
   - Firewall rules
   - Security groups
   - Network policies (Kubernetes)

3. **TLS/HTTPS:**
   - Use a reverse proxy with TLS
   - Use Let's Encrypt for certificates

4. **IP whitelisting:**
   - Restrict access to specific IPs

See [Reverse Proxy Configuration](deployment/reverse-proxy.md) for authentication examples.

### How do I run AlertView as a systemd service?

Create a systemd service file:

```ini
# /etc/systemd/system/alertview.service
[Unit]
Description=AlertView Alert Dashboard
After=network.target

[Service]
Type=simple
User=alertview
Group=alertview
ExecStart=/usr/local/bin/alertview --config /etc/alertview/config.yaml
Restart=on-failure
RestartSec=5s

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/etc/alertview /var/lib/alertview

[Install]
WantedBy=multi-user.target
```

Then:
```bash
sudo systemctl daemon-reload
sudo systemctl enable alertview
sudo systemctl start alertview
```

### How do I scale AlertView?

AlertView is stateless and can be scaled horizontally:

**Docker Compose:**
```yaml
services:
  alertview:
    image: ghcr.io/your-org/alertview:latest
    deploy:
      replicas: 3
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/alertview/config.yaml:ro
```

**Kubernetes:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: alertview
spec:
  replicas: 3
  # ... rest of configuration
```

**Note:** Each instance fetches alerts independently. Consider:
- Reducing refresh interval
- Using a load balancer

### How do I monitor AlertView?

**Health Check:**
```bash
curl http://localhost:8080/health
```

**Logging:**
```bash
# View logs
journalctl -u alertview -f

# Or with Docker
docker logs alertview -f
```

## Troubleshooting Questions

### AlertView won't start. What should I do?

1. Check for errors:
   ```bash
   alertview --config config.yaml 2>&1
   ```

2. Validate your configuration:
   ```bash
   python3 -c "import yaml; yaml.safe_load(open('config.yaml'))"
   ```

3. Enable debug logging:
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   ```

4. Check file permissions:
   ```bash
   ls -la config.yaml
   ```

See [Troubleshooting Guide](troubleshooting.md) for more details.

### No alerts are showing up. What's wrong?

1. Check source connectivity:
   ```bash
   curl -v http://alertmanager.example.com:9093/api/v2/alerts
   ```

2. Check AlertView logs:
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   ```

3. Check source status:
   ```bash
   curl http://localhost:8080/api/sources
   ```

4. Verify your monitoring systems have alerts

See [Troubleshooting Guide](troubleshooting.md) for more details.

### Alerts are not updating. Why?

1. Check refresh interval:
   ```yaml
   display:
     refresh_interval: 30  # Should be > 0
   ```

2. Check caching:
   ```yaml
   cache_ttl: 60  # If too high, alerts may seem stale
   ```

3. Check for errors in logs:
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   ```

See [Troubleshooting Guide](troubleshooting.md) for more details.

### I'm getting authentication errors. How do I fix them?

1. Verify your credentials:
   - Username and password
   - API keys
   - Bearer tokens

2. Test authentication manually:
   ```bash
   curl -u username:password http://alertmanager.example.com:9093/api/v2/alerts
   ```

3. Check permissions:
   - Verify the user/API key has read access to alerts

See [Troubleshooting Guide](troubleshooting.md) for more details.

### I'm getting SSL errors. How do I fix them?

1. **For testing only**, skip verification:
   ```yaml
   sources:
     - name: alertmanager
       kind: alertmanager
       url: https://alertmanager.example.com
       tls:
         skip_verify: true
   ```

2. **For production**, use a valid certificate or configure a custom CA:
   ```yaml
   sources:
     - name: alertmanager
       kind: alertmanager
       url: https://alertmanager.example.com
       tls:
         ca_certificate: /path/to/ca.crt
   ```

See [Troubleshooting Guide](troubleshooting.md) for more details.

### AlertView is using too much memory. How do I reduce it?

1. Reduce cache size:
   ```yaml
   cache_ttl: 30  # Lower value
   ```

2. Reduce refresh interval:
   ```yaml
   display:
     refresh_interval: 60  # Higher value
   ```

3. Filter alerts:
   ```yaml
   display:
     filters:
       severity: [critical]  # Only critical alerts
   ```

4. Monitor fewer sources

### AlertView is slow. How do I make it faster?

1. Enable caching:
   ```yaml
   cache_ttl: 60  # Cache for 60 seconds
   ```

2. Reduce refresh interval:
   ```yaml
   display:
     refresh_interval: 30  # Refresh every 30 seconds
   ```

3. Filter alerts at the source:
   ```yaml
   sources:
     - name: alertmanager
       kind: alertmanager
       url: http://alertmanager.example.com/api/v2/alerts?active=true
   ```

4. Increase timeouts:
   ```yaml
   sources:
     - name: alertmanager
       kind: alertmanager
       url: http://alertmanager.example.com
       timeout: 60  # 60 second timeout
   ```

## Development Questions

### How do I build AlertView from source?

See [Building AlertView](development/building.md) for details.

**Quick Start:**
```bash
# Clone the repository
git clone https://github.com/your-org/alertview.git
cd alertview

# Build
cargo build --release

# Run
./target/release/alertview --config config.example
```

### How do I run tests?

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific tests
cargo test test_name
```

See [Testing AlertView](development/testing.md) for more details.

### How do I contribute to AlertView?

See [Contributing Guide](development/contributing.md) for details.

**Quick Start:**
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Run linter: `cargo clippy`
6. Format code: `cargo fmt`
7. Commit your changes
8. Push to your fork
9. Open a pull request

### How do I report a bug?

1. Check if the bug has already been reported
2. Collect information:
   - AlertView version
   - Configuration
   - Logs
   - Environment details
3. Open a new issue with the collected information

See [Contributing Guide](development/contributing.md) for more details.

### How do I request a feature?

1. Check if the feature has already been requested
2. Open a new issue with:
   - Description of the feature
   - Use case
   - Any relevant details
3. Discuss the feature with maintainers

## Comparison Questions

### How is AlertView different from Grafana?

| Feature | AlertView | Grafana |
|---------|-----------|---------|
| Purpose | Alert viewing | Visualization and monitoring |
| Scope | Alerts only | Metrics, logs, traces, alerts |
| Data sources | Alertmanager, Grafana, Zabbix | Many (Prometheus, Elasticsearch, etc.) |
| Complexity | Lightweight, simple | Feature-rich, complex |
| Use case | Quick alert overview | Comprehensive monitoring |

**Use AlertView if:**
- You want a simple, lightweight alert dashboard
- You need to aggregate alerts from multiple systems
- You want a dedicated alert viewing interface

**Use Grafana if:**
- You need to visualize metrics
- You need dashboards with graphs and charts
- You need a comprehensive monitoring solution

### How is AlertView different from Alertmanager?

| Feature | AlertView | Alertmanager |
|---------|-----------|--------------|
| Purpose | Alert viewing | Alert routing and management |
| Function | Reads alerts | Routes, groups, silences alerts |
| Data source | Alertmanager, Grafana, Zabbix | Prometheus (primarily) |
| UI | Web UI | Web UI (limited) |
| Use case | View alerts | Manage alert routing |

**Use AlertView if:**
- You want a better UI for viewing alerts
- You need to see alerts from multiple systems
- You want a dedicated alert dashboard

**Use Alertmanager if:**
- You need to route alerts to different receivers
- You need to group and silence alerts
- You need to manage alert notifications

### Can I use AlertView with Alertmanager?

Yes! AlertView is designed to work with Alertmanager. Simply configure Alertmanager as a source:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
```

### Can I use AlertView instead of Alertmanager?

No. AlertView is a **viewer** for alerts, not a replacement for Alertmanager. AlertView:
- Reads alerts from Alertmanager
- Displays them in a UI
- Does not route, group, or silence alerts

You still need Alertmanager (or another alerting system) to manage your alerts.

### Can I use AlertView with Prometheus?

AlertView does not directly support Prometheus alerts. However, you can:
1. Use Alertmanager with Prometheus (recommended)
2. Configure Alertmanager as a source in AlertView

**Recommended setup:**
```
Prometheus -> Alertmanager -> AlertView
```

## Roadmap Questions

### What features are planned for the future?

See the [GitHub Issues](https://github.com/your-org/alertview/issues) for planned features.

**Some planned features:**
- Authentication
- Metrics endpoint

- More source types (Datadog, New Relic, etc.)
- Alert actions (acknowledge, silence, resolve)


### When will the next version be released?

There is no fixed release schedule. Releases are made when significant features or bug fixes are ready.

### How can I stay updated on new releases?

- Watch the repository on GitHub
- Subscribe to release notifications
- Follow the project on social media (if available)

## Support Questions

### Where can I get help?

1. **Documentation:** Check this FAQ and the documentation
2. **Issues:** Search and open issues on GitHub
3. **Discussions:** Ask questions in GitHub Discussions
4. **Community:** Ask in relevant communities (e.g., Rust, Prometheus, Grafana)

### How do I contact the maintainers?

- Open an issue on GitHub
- Start a discussion on GitHub
- Email the maintainers (if email is provided)

### Is commercial support available?

Not currently. AlertView is an open-source project maintained by volunteers.

### Can I pay for support?

Not currently. If you're interested in commercial support, contact the maintainers to discuss options.

## License Questions

### What license is AlertView released under?

AlertView is released under the MIT License.

### Can I use AlertView in production?

Yes! AlertView is designed for production use.

### Can I modify AlertView?

Yes! The MIT License allows you to modify AlertView for your own use.

### Can I distribute modified versions of AlertView?

Yes! The MIT License allows you to distribute modified versions, as long as you include the original license and copyright notice.

### Do I need to contribute back my changes?

No. While contributions are welcome, you are not required to contribute your changes back to the project.

## Additional Resources

- [Documentation](README.md)
- [Getting Started](getting-started/README.md)
- [Configuration](configuration/README.md)
- [Deployment](deployment/README.md)
- [Development](development/README.md)
- [Examples](examples/README.md)
- [API Documentation](api.md)
- [Troubleshooting](troubleshooting.md)
- [GitHub Repository](https://github.com/your-org/alertview)
