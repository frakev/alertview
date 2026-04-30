# Examples

This directory contains example configurations and use cases for AlertView.

## Available Examples

- **[Minimal Configuration](minimal.md)** - Basic configuration to get started
- **[Alertmanager](alertmanager.md)** - Complete Alertmanager integration
- **[Grafana](grafana.md)** - Complete Grafana integration
- **[Zabbix](zabbix.md)** - Complete Zabbix integration
- **[Multiple Sources](multiple-sources.md)** - Aggregating alerts from multiple sources
- **[Advanced Configuration](advanced.md)** - All configuration options in use

## Quick Start Examples

### Minimal Alertmanager Setup

```yaml
# config.yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093

port: 8080
```

Run AlertView:

```bash
alertview --config config.yaml
```

Then visit http://localhost:8080

### Docker Compose Example

```yaml
# docker-compose.yml
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

  alertmanager:
    image: prom/alertmanager:latest
    ports:
      - "9093:9093"
```

### Kubernetes Example

```yaml
# k8s/alertview.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: alertview
spec:
  replicas: 1
  selector:
    matchLabels:
      app: alertview
  template:
    metadata:
      labels:
        app: alertview
    spec:
      containers:
      - name: alertview
        image: ghcr.io/your-org/alertview:latest
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: config
          mountPath: /etc/alertview/config.yaml
          subPath: config.yaml
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: alertview-config
---
apiVersion: v1
kind: Service
metadata:
  name: alertview
spec:
  ports:
  - port: 80
    targetPort: 8080
  selector:
    app: alertview
```

## Use Case Examples

### Centralized Alert Dashboard

Aggregate alerts from multiple monitoring systems:

```yaml
sources:
  - name: production-alertmanager
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    
  - name: staging-grafana
    kind: grafana
    url: https://grafana.staging.example.com
    api_key: "your-api-key"
    
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password

display:
  refresh_interval: 30
  theme: dark
  timezone: America/New_York

port: 8080
```

### Team-Specific Dashboard

Create a dashboard for a specific team:

```yaml
sources:
  - name: frontend-alerts
    kind: alertmanager
    url: https://alertmanager.example.com
    link_template: "https://grafana.example.com/d/abc123?var-team=frontend&var-alert={{.Labels.alertname}}"
    
  - name: backend-alerts
    kind: alertmanager
    url: https://alertmanager.example.com
    link_template: "https://grafana.example.com/d/def456?var-team=backend&var-alert={{.Labels.alertname}}"

display:
  refresh_interval: 15
  filters:
    team: frontend
  
port: 8080
```

### On-Call Dashboard

Create a dashboard for on-call engineers:

```yaml
sources:
  - name: pagerduty
    kind: alertmanager
    url: https://alertmanager.example.com
    
  - name: opsgenie
    kind: alertmanager
    url: https://alertmanager.example.com

display:
  refresh_interval: 10
  play_sounds: true
  theme: dark
  
  # Only show firing alerts
  filters:
    state: firing
    
  # Sort by severity (critical first)
  sort:
    by: severity
    order: desc

port: 8080
```

### Embedded Dashboard

Embed AlertView in another application:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093

display:
  # Disable auto-refresh when embedded
  refresh_interval: 0
  
  # Compact display for embedding
  compact_mode: true
  
  # Hide header and footer
  hide_header: true
  hide_footer: true

port: 8080
```

Then embed in your application:

```html
<iframe 
  src="http://localhost:8080" 
  width="100%" 
  height="600px" 
  frameborder="0"
></iframe>
```

## Configuration Examples

### Caching Configuration

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    cache_ttl: 60  # Cache for 60 seconds

# Global cache TTL
cache_ttl: 30
```

### Retry Configuration

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    retry_policy:
      max_retries: 5
      initial_delay_ms: 1000  # 1 second
      max_delay_ms: 10000    # 10 seconds
```

### Timeout Configuration

```yaml
sources:
  - name: slow-source
    kind: alertmanager
    url: http://slow-monitor.example.com
    timeout: 60  # 60 second timeout
    
  - name: fast-source
    kind: alertmanager
    url: http://fast-monitor.example.com
    timeout: 10  # 10 second timeout
```

### Link Templates

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}/{{.Labels.panel}}?viewPanel={{.Labels.panel_id}}"
    
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    link_template: "https://grafana.example.com/d/{{.DashboardUID}}/{{.PanelID}}?orgId={{.OrgID}}"
```

### Display Configuration

```yaml
display:
  # Theme
  theme: dark  # or "light" or a custom CSS URL
  
  # Timezone
  timezone: America/New_York
  
  # Refresh interval (seconds)
  refresh_interval: 30
  
  # Enable sound notifications
  play_sounds: true
  
  # Default filters
  filters:
    severity: critical
    state: firing
  
  # Default sort
  sort:
    by: starts_at
    order: desc
  
  # Compact mode
  compact_mode: false
  
  # Hide elements
  hide_header: false
  hide_footer: false
```

### Environment Variables

```bash
# Set environment variables
export ALERTVIEW_PORT=9090
export ALERTVIEW_REFRESH_INTERVAL=60
export ALERTVIEW_CACHE_TTL=120
export ALERTVIEW_LOG_FORMAT=json
export RUST_LOG=info

# Run AlertView
alertview --config config.yaml
```

Or in Docker:

```yaml
# docker-compose.yml
environment:
  - ALERTVIEW_PORT=9090
  - ALERTVIEW_REFRESH_INTERVAL=60
  - ALERTVIEW_CACHE_TTL=120
  - ALERTVIEW_LOG_FORMAT=json
  - RUST_LOG=info
```

## Real-World Scenarios

### Scenario 1: SRE Team Dashboard

**Requirements:**
- Monitor production Alertmanager
- Show only critical and warning alerts
- Auto-refresh every 15 seconds
- Dark theme for 24/7 monitoring
- Sound notifications for new critical alerts

**Configuration:**

```yaml
sources:
  - name: production
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000

display:
  refresh_interval: 15
  theme: dark
  play_sounds: true
  timezone: UTC
  filters:
    severity: [critical, warning]
    state: firing
  sort:
    by: starts_at
    order: desc

port: 8080
log_format: json
```

### Scenario 2: Development Team Dashboard

**Requirements:**
- Monitor staging environment
- Show alerts from multiple sources
- Link to Grafana dashboards
- Light theme
- No sound notifications

**Configuration:**

```yaml
sources:
  - name: staging-alertmanager
    kind: alertmanager
    url: https://alertmanager.staging.example.com
    link_template: "https://grafana.staging.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
    
  - name: staging-grafana
    kind: grafana
    url: https://grafana.staging.example.com
    api_key: "staging-api-key"
    link_template: "https://grafana.staging.example.com/d/{{.DashboardUID}}"

display:
  refresh_interval: 30
  theme: light
  play_sounds: false
  timezone: America/New_York

port: 8080
```

### Scenario 3: NOC Dashboard

**Requirements:**
- Large screen display
- Multiple sources
- Group alerts by service
- High visibility for critical alerts
- Minimal UI chrome

**Configuration:**

```yaml
sources:
  - name: network
    kind: alertmanager
    url: https://alertmanager.noc.example.com
    
  - name: servers
    kind: alertmanager
    url: https://alertmanager.noc.example.com
    
  - name: applications
    kind: alertmanager
    url: https://alertmanager.noc.example.com

display:
  refresh_interval: 10
  theme: dark
  play_sounds: true
  timezone: UTC
  hide_header: true
  hide_footer: true
  compact_mode: false
  
  # Group by service label
  group_by: [service]
  
  # Highlight critical alerts
  severity_colors:
    critical: "#ff0000"
    warning: "#ffcc00"
    info: "#00ccff"

port: 8080
```

### Scenario 4: Embedded in Grafana

**Requirements:**
- Embed in Grafana dashboard
- Show alerts for specific dashboard
- Match Grafana theme
- No auto-refresh (Grafana handles it)

**Configuration:**

```yaml
sources:
  - name: grafana-alerts
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    dashboard_uid: "abc123"

display:
  refresh_interval: 0  # Disable auto-refresh
  theme: auto  # Match browser theme
  play_sounds: false
  compact_mode: true
  hide_header: true
  hide_footer: true

port: 8080
```

Grafana panel HTML:

```html
<div style="width: 100%; height: 500px;">
  <iframe 
    src="http://alertview.example.com" 
    width="100%" 
    height="100%" 
    frameborder="0"
    style="border: none;"
  ></iframe>
</div>
```

## Troubleshooting Examples

### Example: Debugging Configuration

```bash
# Enable debug logging
RUST_LOG=debug alertview --config config.yaml

# Check config is loaded correctly
RUST_LOG=debug,alertview::config=trace alertview --config config.yaml
```

### Example: Testing Connectivity

```bash
# Test if AlertView can reach Alertmanager
curl -v http://localhost:9093/api/v2/alerts

# Test from within AlertView container
docker exec -it alertview curl -v http://alertmanager:9093/api/v2/alerts
```

### Example: Validating Configuration

```bash
# Check YAML syntax
python3 -c "import yaml, sys; yaml.safe_load(open(sys.argv[1]))" config.yaml

# Or use yamllint
yamllint config.yaml
```

## Performance Tuning Examples

### High-Volume Setup

```yaml
# Increase timeouts and retries for unreliable sources
sources:
  - name: unreliable-source
    kind: alertmanager
    url: http://unreliable.example.com
    timeout: 120
    retry_policy:
      max_retries: 10
      initial_delay_ms: 2000
      max_delay_ms: 30000

# Enable caching to reduce load
cache_ttl: 60

# Use JSON logging for better performance
log_format: json

# Increase refresh interval
refresh_interval: 60
```

### Low-Latency Setup

```yaml
# Short timeouts for fast response
sources:
  - name: fast-source
    kind: alertmanager
    url: http://localhost:9093
    timeout: 5
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 2000

# Frequent refresh
refresh_interval: 5

# Disable caching for real-time updates
cache_ttl: 0
```

## Security Examples

### Secure Configuration

```yaml
# Use environment variables for sensitive data
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    # Don't put credentials in config file!
    # Use environment variables instead

# In environment:
# ALERTMANAGER_BASIC_AUTH=username:password
```

### TLS Configuration

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    # AlertView automatically validates TLS certificates
    # For self-signed certificates, use:
    tls:
      skip_verify: false  # Default is false (verify)
      # ca_certificate: /path/to/ca.crt  # Custom CA
```

### Network Isolation

```yaml
# Use a reverse proxy for additional security
# AlertView runs on localhost, proxy handles external access

# In AlertView config:
port: 8080

# In Nginx config:
server {
    listen 443 ssl;
    server_name alertview.example.com;
    
    ssl_certificate /etc/letsencrypt/live/alertview.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/alertview.example.com/privkey.pem;
    
    location / {
        auth_basic "AlertView";
        auth_basic_user_file /etc/nginx/.htpasswd;
        
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
    }
}
```

## Migration Examples

### Migrating from v0.1 to v0.2

**Changes:**
- `refresh_interval` is now in `display` section
- `timeout` is now per-source
- New `retry_policy` configuration

**Old config (v0.1):**

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093

refresh_interval: 30
timeout: 10
port: 8080
```

**New config (v0.2):**

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    timeout: 10

display:
  refresh_interval: 30

port: 8080
```

### Migrating from v0.2 to v0.3

**Changes:**
- New `link_template` configuration
- New `cache_ttl` configuration
- New `log_format` configuration

**Old config (v0.2):**

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093

display:
  refresh_interval: 30

port: 8080
```

**New config (v0.3):**

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"

cache_ttl: 60
log_format: text

display:
  refresh_interval: 30

port: 8080
```

## Additional Resources

- [Configuration Reference](../configuration/config-file.md)
- [Deployment Guide](../deployment/README.md)
- [Troubleshooting Guide](../troubleshooting.md)
- [FAQ](../faq.md)
