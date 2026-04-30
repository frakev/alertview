# Advanced Configuration

This example demonstrates all AlertView configuration options in a comprehensive setup.

## Complete Configuration File

```yaml
# Global settings
global:
  # Log format: text or json
  log_format: json
  
  # Log level: error, warn, info, debug, trace
  # (Set via RUST_LOG environment variable)

# Alert sources
sources:
  # Alertmanager source
  - name: production-alertmanager
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    
    # Authentication
    basic_auth:
      username: api-user
      password: api-password
    # Or use bearer token
    # bearer_token: "your-token"
    
    # TLS settings
    tls:
      skip_verify: false
      # ca_certificate: /path/to/ca.crt
    
    # Retry policy
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    
    # Caching
    cache_ttl: 60
    
    # Link template
    link_template: "https://grafana.prod.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}&from=now-1h&to=now"
    
    # Custom headers
    headers:
      X-Custom-Header: "custom-value"
    
    # Query parameters
    query_params:
      filter: "severity=critical"
  
  # Grafana source
  - name: production-grafana
    kind: grafana
    url: https://grafana.prod.example.com
    api_key: "your-api-key"
    timeout: 30
    
    # Folder to fetch alerts from
    folder_id: 123
    
    # Retry policy
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    
    # Caching
    cache_ttl: 60
    
    # Link template
    link_template: "https://grafana.prod.example.com/d/{{.DashboardUID}}/{{.PanelID}}?viewPanel={{.PanelID}}&orgId={{.OrgID}}&from=now-1h&to=now"
    
    # Only fetch alerts from specific dashboard
    dashboard_uid: "abc123"
  
  # Zabbix source
  - name: production-zabbix
    kind: zabbix
    url: https://zabbix.prod.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    timeout: 30
    
    # Retry policy
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    
    # Caching
    cache_ttl: 60
    
    # Link template
    link_template: "https://zabbix.prod.example.com/monitoring.php?triggerid={{.TriggerID}}&hostid={{.HostID}}"
    
    # Filtering
    only_active: true
    min_severity: 2  # Warning and above (0-5)
    host_group: "Production Servers"

# Global cache TTL (applies to sources without explicit cache_ttl)
cache_ttl: 30

# Display configuration
display:
  # Port to listen on
  port: 8080
  
  # Refresh interval in seconds
  refresh_interval: 30
  
  # Theme: dark, light, auto, or custom CSS URL
  theme: dark
  
  # Custom CSS URL (overrides theme)
  # custom_css: "https://example.com/custom.css"
  
  # Timezone: UTC, local, or IANA timezone
  timezone: America/New_York
  
  # Enable sound notifications
  play_sounds: true
  
  # Sound configuration
  sounds:
    enabled: true
    critical:
      frequency: 800
      duration: 0.5
      type: sine
    warning:
      frequency: 400
      duration: 0.3
      type: square
    info:
      frequency: 200
      duration: 0.2
      type: sawtooth
  
  # Default filters
  filters:
    severity: [critical, warning]
    state: [firing]
    source: [production-alertmanager, production-grafana]
    # Custom label filters
    team: [frontend, backend]
    service: [web, api, database]
  
  # Default sort
  sort:
    by: starts_at  # or: starts_at, severity, source, state, alertname
    order: desc    # or: asc
  
  # Group alerts
  group_by: [alertname, service]
  
  # Sort groups
  group_sort:
    by: severity
    order: desc
  
  # Compact mode
  compact_mode: false
  
  # Hide elements
  hide_header: false
  hide_footer: false
  hide_filters: false
  hide_sort: false
  
  # Column visibility
  columns:
    severity: true
    state: true
    starts_at: true
    ends_at: true
    source: true
    labels: true
    annotations: true
  
  # Column order
  column_order:
    - severity
    - state
    - alertname
    - starts_at
    - source
    - labels
    - annotations
  
  # Severity colors
  severity_colors:
    critical: "#ff0000"
    warning: "#ffcc00"
    info: "#00ccff"
    unknown: "#cccccc"
  
  # State colors
  state_colors:
    firing: "#ff0000"
    resolved: "#00ff00"
    silenced: "#cccc00"
    paused: "#cccccc"
  
  # Date/time format
  date_format: "YYYY-MM-DD HH:mm:ss"
  relative_time: true
  
  # Alert age thresholds
  age_thresholds:
    new: 300      # 5 minutes
    recent: 3600  # 1 hour
    old: 86400    # 1 day
  
  # Alert age colors
  age_colors:
    new: "#00ff00"
    recent: "#ffff00"
    old: "#ff0000"

# Health check endpoint
health:
  enabled: true
  path: /health
  port: 8080

# API configuration
api:
  # Enable CORS
  cors:
    enabled: true
    origins: ["*"]
    methods: [GET, POST, OPTIONS]
    headers: [Content-Type, Authorization]
  

```

## Environment Variables

All configuration options can also be set via environment variables:

```bash
# Global settings
export ALERTVIEW_LOG_FORMAT=json
export RUST_LOG=info

# Server settings
export ALERTVIEW_PORT=8080

# Display settings
export ALERTVIEW_REFRESH_INTERVAL=30
export ALERTVIEW_THEME=dark
export ALERTVIEW_TIMEZONE=America/New_York
export ALERTVIEW_PLAY_SOUNDS=true

# Cache settings
export ALERTVIEW_CACHE_TTL=60

# Source-specific settings
export ALERTMANAGER_URL=https://alertmanager.example.com
export ALERTMANAGER_TIMEOUT=30
export GRAFANA_URL=https://grafana.example.com
export GRAFANA_API_KEY=your-api-key
```

## Command Line Arguments

```bash
# Specify config file
alertview --config /path/to/config.yaml

# Specify port
alertview --port 9090

# Enable debug logging
RUST_LOG=debug alertview --config config.yaml

# Combine with environment variables
ALERTVIEW_PORT=9090 ALERTVIEW_LOG_FORMAT=json alertview --config config.yaml
```

## Configuration Precedence

AlertView uses the following precedence order (highest to lowest):

1. **Command line arguments** - `--port`, `--config`
2. **Environment variables** - `ALERTVIEW_*`, `RUST_LOG`
3. **Configuration file** - `config.yaml`
4. **Default values** - Built-in defaults

## Advanced Use Cases

### Custom Severity Mapping

```yaml
display:
  severity_mapping:
    # Map custom severity values to standard levels
    custom-critical: critical
    custom-warning: warning
    custom-info: info
    
  # Custom severity colors
  severity_colors:
    custom-critical: "#ff0000"
    custom-warning: "#ffcc00"
    custom-info: "#00ccff"
```

### Custom State Mapping

```yaml
display:
  state_mapping:
    # Map custom state values to standard states
    open: firing
    closed: resolved
    acknowledged: silenced
    
  # Custom state colors
  state_colors:
    open: "#ff0000"
    closed: "#00ff00"
    acknowledged: "#cccc00"
```

### Custom Filters

```yaml
display:
  filters:
    # Standard filters
    severity: [critical, warning]
    state: [firing]
    
    # Custom label filters
    custom_label: [value1, value2]
    
    # Regex filters
    regex_filters:
      alertname: ".*production.*"
      service: "^(web|api|database)$"
    
    # Numeric filters
    numeric_filters:
      cpu_usage: ">90"
      memory_usage: ">80"
```

### Custom Sorting

```yaml
display:
  sort:
    by: [severity, starts_at]  # Multi-field sort
    order: [desc, desc]       # Order for each field
```

### Custom Grouping

```yaml
display:
  group_by: [service, team, environment]
  group_sort:
    by: [severity, starts_at]
    order: [desc, desc]
  
  # Group display options
  group_display:
    collapsed_by_default: false
    show_count: true
    show_severity: true
    show_source: true
```

### Custom Columns

```yaml
display:
  columns:
    # Standard columns
    severity: true
    state: true
    alertname: true
    starts_at: true
    
    # Custom columns (from labels/annotations)
    custom_columns:
      - name: team
        label: Team
        field: Labels.team
        width: 100
        
      - name: service
        label: Service
        field: Labels.service
        width: 100
        
      - name: description
        label: Description
        field: Annotations.description
        width: 300
```

### Custom Actions

```yaml
display:
  actions:
    # Enable actions
    enabled: true
    
    # Action buttons
    buttons:
      - name: acknowledge
        label: Acknowledge
        icon: check
        action: acknowledge
        
      - name: snooze
        label: Snooze
        icon: pause
        action: snooze
        duration: 3600  # 1 hour
        
      - name: resolve
        label: Resolve
        icon: times
        action: resolve
```

## Performance Tuning

### High-Volume Configuration

```yaml
# Global settings
cache_ttl: 120

display:
  refresh_interval: 60

sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    timeout: 60
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 30000
    cache_ttl: 120
    
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    timeout: 60
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 30000
    cache_ttl: 120
```

### Low-Latency Configuration

```yaml
# Global settings
cache_ttl: 0  # Disable caching

display:
  refresh_interval: 5

sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    timeout: 5
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 2000
    cache_ttl: 0
```

### Balanced Configuration

```yaml
# Global settings
cache_ttl: 60

display:
  refresh_interval: 30

sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    timeout: 15
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    cache_ttl: 60
```

## Security Configuration

### TLS/HTTPS

```yaml
# In server configuration (not in AlertView config)
# AlertView uses the system's TLS configuration

# For source connections
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    tls:
      skip_verify: false  # Verify certificates
      ca_certificate: /path/to/ca.crt
```

### Authentication

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    basic_auth:
      username: api-user
      password: api-password
    
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

### Network Security

```yaml
# Use a reverse proxy for additional security
# AlertView runs on localhost, proxy handles external access

# In AlertView config
port: 8080

# In Nginx config
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
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Monitoring and Observability

### Health Check

```yaml
health:
  enabled: true
  path: /health
  port: 8080
```

Test health endpoint:

```bash
curl -I http://localhost:8080/health
curl http://localhost:8080/health
```

### Logging

```yaml
# In config
global:
  log_format: json

# Via environment variable
RUST_LOG=info
RUST_LOG=debug,alertview::config=trace
```

## Docker Configuration

### Docker Compose

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
      - ALERTVIEW_LOG_FORMAT=json
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### Docker Run

```bash
docker run -d \
  --name alertview \
  -p 8080:8080 \
  -v /path/to/config.yaml:/etc/alertview/config.yaml:ro \
  -e RUST_LOG=info \
  -e ALERTVIEW_LOG_FORMAT=json \
  --restart unless-stopped \
  ghcr.io/your-org/alertview:latest
```

## Kubernetes Configuration

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: alertview
  labels:
    app: alertview
spec:
  replicas: 2
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
        env:
        - name: RUST_LOG
          value: "info"
        - name: ALERTVIEW_LOG_FORMAT
          value: "json"
        resources:
          requests:
            memory: "64Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: config
        configMap:
          name: alertview-config
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: alertview
  labels:
    app: alertview
spec:
  type: ClusterIP
  ports:
  - port: 80
    targetPort: 8080
  selector:
    app: alertview
```

### Ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: alertview
  labels:
    app: alertview
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    nginx.ingress.kubernetes.io/auth-type: basic
    nginx.ingress.kubernetes.io/auth-secret: alertview-auth
    nginx.ingress.kubernetes.io/auth-realm: "Authentication Required"
spec:
  rules:
  - host: alertview.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: alertview
            port:
              number: 80
```

## Configuration Validation

### Validate YAML Syntax

```bash
# Using Python
python3 -c "import yaml, sys; yaml.safe_load(open(sys.argv[1]))" config.yaml

# Using yamllint
yamllint config.yaml
```

### Validate Configuration

```bash
# AlertView will validate config on startup
alertview --config config.yaml

# With debug logging to see validation details
RUST_LOG=debug alertview --config config.yaml
```

## Configuration Examples by Use Case

### 24/7 Monitoring Dashboard

```yaml
sources:
  - name: production-alertmanager
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    cache_ttl: 60

display:
  refresh_interval: 15
  theme: dark
  timezone: UTC
  play_sounds: true
  sounds:
    critical:
      frequency: 800
      duration: 0.5
    warning:
      frequency: 400
      duration: 0.3
  filters:
    severity: [critical, warning]
    state: [firing]
  sort:
    by: starts_at
    order: desc

port: 8080
log_format: json
```

### Team Dashboard

```yaml
sources:
  - name: team-alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    link_template: "https://grafana.example.com/d/team-dashboard?var-team=myteam&var-alert={{.Labels.alertname}}"

display:
  refresh_interval: 30
  theme: light
  timezone: America/New_York
  filters:
    team: myteam
  group_by: [service]

port: 8080
```

### Embedded Dashboard

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093

display:
  refresh_interval: 0  # Disable auto-refresh
  theme: auto
  compact_mode: true
  hide_header: true
  hide_footer: true

port: 8080
```

### Centralized Alert Aggregation

```yaml
sources:
  - name: prod-alertmanager
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    cache_ttl: 60
    
  - name: staging-alertmanager
    kind: alertmanager
    url: https://alertmanager.staging.example.com
    timeout: 15
    cache_ttl: 30
    
  - name: prod-grafana
    kind: grafana
    url: https://grafana.prod.example.com
    api_key: "prod-api-key"
    timeout: 30
    cache_ttl: 60
    
  - name: staging-grafana
    kind: grafana
    url: https://grafana.staging.example.com
    api_key: "staging-api-key"
    timeout: 15
    cache_ttl: 30

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC
  group_by: [environment, service]

cache_ttl: 30
port: 8080
log_format: json
```

## Additional Resources

- [Configuration Reference](../configuration/config-file.md)
- [Environment Variables](../configuration/environment-variables.md)
- [Alertmanager Configuration](alertmanager.md)
- [Grafana Configuration](grafana.md)
- [Zabbix Configuration](zabbix.md)
- [Multiple Sources Configuration](multiple-sources.md)
