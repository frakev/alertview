# Advanced Configuration

This example demonstrates all AlertView configuration options in a comprehensive setup.

## Complete Configuration File

```yaml
# Server configuration
port: 8080
refresh_interval: 30
log_format: json
tls_insecure: false

# Global cache TTL (applies to sources without explicit cache_ttl)
cache_ttl_seconds: 30

# Alert sources
sources:
  # Alertmanager source
  - name: production-alertmanager
    type: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    
    # Authentication
    basic_auth:
      username: api-user
      password: api-password
    # Or use bearer token
    # bearer_token: "your-token"
    
    # Retry policy
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    
    # Caching (overrides global)
    # cache_ttl_seconds: 60
    
    # Link template with available variables:
    # {{.Labels.<key>}}, {{.Annotations.<key>}}, {{.Fingerprint}}, {{.Id}},
    # {{.Source}}, {{.SourceType}}, {{.Status}}, {{.Severity}}, {{.Name}},
    # {{.StartsAt}}, {{.EndsAt}}
    link_template: "https://grafana.prod.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}&from=now-1h&to=now"
    
    # Dashboard URL (fallback if link_template doesn't match)
    dashboard_url: "https://grafana.prod.example.com/alerting/list"
    
  # Grafana source
  - name: production-grafana
    type: grafana
    url: https://grafana.prod.example.com
    timeout: 30
    
    # Authentication (Grafana Service Account token)
    bearer_token: "your-grafana-token"
    
    # Or use basic auth
    # basic_auth:
    #   username: api-user
    #   password: api-password
    
    # Retry policy
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    
    # Caching
    # cache_ttl_seconds: 60
    
    # Link template
    link_template: "https://grafana.prod.example.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}&from=now-1h&to=now"
    
    # Dashboard URL
    dashboard_url: "https://grafana.prod.example.com/"
    
  # Zabbix source
  - name: production-zabbix
    type: zabbix
    url: https://zabbix.prod.example.com/zabbix
    timeout: 45
    
    # Authentication (Zabbix API token)
    bearer_token: "your-zabbix-token"
    
    # Retry policy
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 60000
    
    # Caching
    # cache_ttl_seconds: 60
    
    # Link template - use standard alert fields
    link_template: "https://zabbix.prod.example.com/zabbix.php?action=problem.view&filter_eventid={{.Labels.eventid}}"
    
    # Dashboard URL
    dashboard_url: "https://zabbix.prod.example.com/zabbix/zabbix.php?action=problem.view"

# Display configuration
display:
  # Labels to display on alert cards
  labels:
    - namespace
    - job
    - instance
    - cluster
    - node
    - pod
    - host
    - hostgroup
  
  # Theme: dark, light, or custom CSS URL
  theme: dark
  
  # Timezone: local, UTC, or IANA timezone (e.g., Europe/Paris)
  timezone: local
  
  # Enable sound notifications
  play_sounds: false
  
  # Group alerts by labels
  group_by: [namespace, job]

# Health check endpoint
# Accessible at /health
# Returns: OK
```

## Configuration by Use Case

### 24/7 Monitoring Dashboard

```yaml
port: 8080
refresh_interval: 15
cache_ttl_seconds: 60

sources:
  - name: production-alertmanager
    type: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    retry_policy:
      max_retries: 5
      initial_delay_ms: 1000
      max_delay_ms: 30000
    link_template: "https://grafana.prod.example.com/d/alerts?query={{.Labels.alertname}}"
    dashboard_url: "https://grafana.prod.example.com/alerting/list"

display:
  labels: [namespace, job, instance, severity]
  theme: dark
  timezone: UTC
  play_sounds: true
  group_by: [namespace]
```

### Development Environment

```yaml
port: 8080
refresh_interval: 30
tls_insecure: true  # For self-signed certificates in dev

sources:
  - name: local-alertmanager
    type: alertmanager
    url: http://localhost:9093
    timeout: 15
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 5000

display:
  labels: [alertname, namespace, job]
  theme: light
  timezone: local
  play_sounds: false
```

### Multi-Cluster Setup

```yaml
port: 8080
refresh_interval: 30
cache_ttl_seconds: 120

sources:
  - name: cluster-us-east
    type: alertmanager
    url: https://alertmanager.us-east.example.com
    timeout: 30
    link_template: "https://grafana.example.com/d/cluster-us-east?var-alert={{.Labels.alertname}}"
    
  - name: cluster-us-west
    type: alertmanager
    url: https://alertmanager.us-west.example.com
    timeout: 30
    link_template: "https://grafana.example.com/d/cluster-us-west?var-alert={{.Labels.alertname}}"
    
  - name: cluster-eu
    type: alertmanager
    url: https://alertmanager.eu.example.com
    timeout: 30
    link_template: "https://grafana.example.com/d/cluster-eu?var-alert={{.Labels.alertname}}"

display:
  labels: [namespace, job, cluster, instance]
  theme: dark
  timezone: UTC
  group_by: [cluster, namespace]
```

### High-Latency Sources

```yaml
port: 8080
refresh_interval: 60

sources:
  - name: remote-zabbix
    type: zabbix
    url: https://zabbix.remote.example.com/zabbix
    timeout: 60
    bearer_token: "your-zabbix-token"
    retry_policy:
      max_retries: 10
      initial_delay_ms: 2000
      max_delay_ms: 120000
    cache_ttl_seconds: 300

display:
  labels: [host, hostgroup, severity]
  theme: dark
  timezone: local
```

### Minimal Configuration

```yaml
port: 8080
refresh_interval: 30

sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093

display:
  labels: [namespace, job]
```

### No Caching

```yaml
port: 8080
refresh_interval: 30
cache_ttl_seconds: 0  # Disable caching

sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    timeout: 15
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 5000
```

## Link Template Examples

### Grafana

```yaml
link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
```

### Alertmanager

```yaml
link_template: "https://alertmanager.example.com/#/alerts?query={{.Labels.alertname}}"
```

### Zabbix

```yaml
link_template: "https://zabbix.example.com/zabbix.php?action=problem.view&filter_eventid={{.Labels.eventid}}"
```

### Custom with Multiple Variables

```yaml
link_template: "https://my-monitoring.example.com/alerts?source={{.Source}}&severity={{.Severity}}&name={{.Name}}"
```

## Display Configuration Examples

### Group by Namespace and Job

```yaml
display:
  labels: [namespace, job, instance, pod]
  theme: dark
  timezone: UTC
  play_sounds: true
  group_by: [namespace, job]
```

### Group by Service and Team

```yaml
display:
  labels: [service, team, environment, severity]
  theme: light
  timezone: America/New_York
  play_sounds: false
  group_by: [service, team, environment]
```

### Custom Theme

```yaml
display:
  labels: [namespace, job]
  theme: "https://example.com/custom-alertview-theme.css"
  timezone: local
```

## Environment Variable Examples

### Using Environment Variables

```bash
# Set via environment
export ALERTVIEW_PORT=8080
export ALERTVIEW_REFRESH_INTERVAL=30
export ALERTVIEW_CACHE_TTL=60
export ALERTVIEW_LOG_FORMAT=json

# Then run AlertView
cargo run -- config.yaml
```

### Docker with Environment Variables

```bash
docker run -p 8080:8080 \
  -e ALERTVIEW_PORT=8080 \
  -e ALERTVIEW_REFRESH_INTERVAL=30 \
  -e ALERTVIEW_LOG_FORMAT=json \
  -v $(pwd)/config.yaml:/config/config.yaml:rw \
  alertview
```

## Logging Configuration

### Text Format (Default)

```yaml
log_format: text
```

### JSON Format

```yaml
log_format: json
```

Or via environment variable:

```bash
ALERTVIEW_LOG_FORMAT=json cargo run -- config.yaml
```

## Retry Policy Examples

### Conservative Retry

```yaml
retry_policy:
  max_retries: 2
  initial_delay_ms: 500
  max_delay_ms: 5000
```

### Aggressive Retry for Unstable Sources

```yaml
retry_policy:
  max_retries: 10
  initial_delay_ms: 2000
  max_delay_ms: 120000
```

### No Retry

```yaml
retry_policy:
  max_retries: 0
```

## Timeout Examples

### Fast Sources (Local Network)

```yaml
timeout: 10
```

### Slow Sources (Remote/Cloud)

```yaml
timeout: 60
```

## Kubernetes Deployment Example

```yaml
# 02-configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: alertview-config
  namespace: alertview
data:
  config.yaml: |
    port: 8080
    refresh_interval: 30
    cache_ttl_seconds: 60
    
    sources:
      - name: alertmanager
        type: alertmanager
        url: http://alertmanager.alertview.svc.cluster.local:9093
        timeout: 15
        retry_policy:
          max_retries: 3
          initial_delay_ms: 1000
          max_delay_ms: 10000
    
    display:
      labels: [namespace, job, instance]
      theme: dark
      timezone: UTC
      group_by: [namespace]
```

## Docker Compose Example

```yaml
version: '3.8'

services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/config/config.yaml:rw
    environment:
      - ALERTVIEW_PORT=8080
      - ALERTVIEW_REFRESH_INTERVAL=30
      - ALERTVIEW_LOG_FORMAT=json
    restart: unless-stopped
```

## Debugging Configuration

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run -- config.yaml
```

### Validate Configuration

```bash
# AlertView will error on invalid YAML
cargo run -- config.yaml

# Or use yamllint
yamllint config.yaml
```

## Configuration Examples by Use Case

### 24/7 Monitoring Dashboard

```yaml
sources:
  - name: production-alertmanager
    type: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    retry_policy:
      max_retries: 5
      initial_delay_ms: 1000
      max_delay_ms: 30000
    link_template: "https://grafana.prod.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}&from=now-1h&to=now"
    dashboard_url: "https://grafana.prod.example.com/alerting/list"

display:
  labels: [namespace, job, instance, severity]
  theme: dark
  timezone: UTC
  play_sounds: true
  group_by: [namespace]
```

### Small Team Setup

```yaml
port: 8080
refresh_interval: 60
cache_ttl_seconds: 0

sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    timeout: 15
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 5000

display:
  labels: [alertname, namespace]
  theme: dark
  timezone: local
  group_by: [namespace]
```
