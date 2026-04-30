# Advanced Configuration

This guide covers advanced configuration options for power users.

## Table of Contents

- [Caching](#caching)
- [Retry Policy](#retry-policy)
- [Timeout Configuration](#timeout-configuration)
- [Link Templates](#link-templates)
- [Performance Tuning](#performance-tuning)
- [Security Considerations](#security-considerations)

## Caching

AlertView supports caching of alert data to reduce load on your alert sources and improve performance.

### Configuration

```yaml
# Global cache TTL in seconds
cache_ttl_seconds: 60

# Or disable caching
cache_ttl_seconds: 0
```

### How Caching Works

1. **Per-Source Caching**: Each source is cached independently
2. **TTL-Based**: Cache entries expire after `cache_ttl_seconds`
3. **Automatic Invalidation**: Expired entries are automatically refreshed
4. **In-Memory**: Cache is stored in memory (RAM), not on disk

### Cache Key

The cache key is generated from:
- Source name
- Source URL
- Source type

This means if any of these change, a new cache entry will be created.

### When to Use Caching

**✅ Good for:**
- Sources with many alerts that don't change frequently
- Slow sources (e.g., Zabbix with high latency)
- Reducing load on your monitoring infrastructure
- Improving response times

**❌ Not recommended for:**
- Sources with rapidly changing alerts
- Sources where you need real-time data
- Low-traffic deployments (caching overhead may not be worth it)

### Cache Statistics

When caching is enabled, you'll see log messages like:

```
DEBUG Cache hit for source Alertmanager
DEBUG Fetching from source Grafana
```

### Example Configurations

**Conservative Caching (30 seconds):**
```yaml
cache_ttl_seconds: 30
```

**Moderate Caching (1 minute):**
```yaml
cache_ttl_seconds: 60
```

**Aggressive Caching (5 minutes):**
```yaml
cache_ttl_seconds: 300
```

### Cache Invalidation

The cache is automatically invalidated when:
1. The TTL expires
2. The configuration changes (via auto-reload)
3. AlertView restarts

There is currently no manual cache invalidation endpoint.

## Retry Policy

Configure how AlertView handles failed requests to alert sources.

### Configuration

```yaml
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager:9093"
    retry_policy:
      max_retries: 3        # Maximum number of retry attempts
      initial_delay_ms: 1000 # Delay before first retry (1 second)
      max_delay_ms: 30000   # Maximum delay between retries (30 seconds)
```

### How Retry Works

AlertView uses **exponential backoff** for retries:

1. **Attempt 1**: Immediate (no delay)
2. **Attempt 2**: After `initial_delay_ms` (1 second)
3. **Attempt 3**: After `2 * initial_delay_ms` (2 seconds)
4. **Attempt 4**: After `4 * initial_delay_ms` (4 seconds)
5. **...**: Doubling each time, up to `max_delay_ms`

### Retry Behavior

- **Transient Errors**: Retried (connection errors, timeouts)
- **Permanent Errors**: Not retried (4xx HTTP errors)
- **Timeout Errors**: Retried with backoff

### Example Scenarios

**Scenario 1: Temporary Network Issue**
```
Attempt 1: Immediate - Connection refused
Wait 1s
Attempt 2: Connection refused
Wait 2s
Attempt 3: Connection refused
Wait 4s
Attempt 4: Success! (returns alerts)
```

**Scenario 2: Authentication Error**
```
Attempt 1: Immediate - HTTP 401
No retry - permanent error
```

**Scenario 3: Server Overload**
```
Attempt 1: Immediate - HTTP 503
Wait 1s
Attempt 2: HTTP 503
Wait 2s
Attempt 3: HTTP 503
Wait 4s
Attempt 4: HTTP 200 (success)
```

### Recommended Retry Policies

| Source Type | max_retries | initial_delay_ms | max_delay_ms |
|-------------|-------------|------------------|--------------|
| Alertmanager | 3 | 1000 | 30000 |
| Grafana | 3 | 1000 | 30000 |
| Zabbix | 5 | 2000 | 60000 |
| Slow Network | 5 | 3000 | 60000 |
| Unstable Source | 10 | 1000 | 120000 |

### Disabling Retries

```yaml
retry_policy:
  max_retries: 0  # No retries
```

## Timeout Configuration

Each source can have its own timeout for HTTP requests.

### Configuration

```yaml
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager:9093"
    timeout: 15  # Seconds (default: 15)

  - name: "Zabbix"
    type: zabbix
    url: "https://zabbix.example.com/zabbix"
    timeout: 45  # Longer timeout for slower APIs
```

### How Timeouts Work

1. **Request Timeout**: The entire HTTP request must complete within `timeout` seconds
2. **Includes**: Connection time, TLS handshake, and response time
3. **Retry Timeout**: Each retry attempt has its own timeout
4. **Total Timeout**: Maximum total time is `timeout * (max_retries + 1)`

### Recommended Timeouts

| Source Type | Recommended Timeout | Notes |
|-------------|---------------------|-------|
| Alertmanager | 15s | Fast, local networks |
| Grafana | 15-30s | May vary based on load |
| Zabbix | 30-45s | JSON-RPC can be slow |
| Remote Sources | 30s | Account for network latency |
| Cloud Services | 30-60s | Higher latency, rate limits |

### Timeout Errors

When a timeout occurs:
1. The attempt fails
2. If retries are configured, the next retry is attempted
3. If no more retries, the source is marked as "error"
4. The error is logged with details

Example log message:
```
WARN Failed to fetch from Alertmanager: Timeout after 15s
```

## Link Templates

Create custom links for your alerts using template variables.

### Available Variables

**Labels:**
```
{{.Labels.<key>}}  # e.g., {{.Labels.namespace}}
```

**Annotations:**
```
{{.Annotations.<key>}}  # e.g., {{.Annotations.summary}}
```

**Alert Fields:**
```
{{.Id}}            # Unique alert fingerprint (alias for {{.Fingerprint}})
{{.Fingerprint}}   # Unique alert fingerprint
{{.Source}}       # Source name
{{.SourceType}}   # alertmanager, grafana, zabbix
{{.Status}}       # firing, silenced, pending
{{.Severity}}     # critical, high, warning, info, none
{{.Name}}         # Alert name
{{.StartsAt}}     # Start time (RFC3339)
{{.EndsAt}}       # End time (RFC3339, if resolved)
```

### Examples

**Basic Grafana Link:**
```yaml
link_template: "https://grafana.example.com/alerts?query={{.Labels.alertname}}"
```

**Grafana Dashboard with Variables:**
```yaml
link_template: "https://grafana.example.com/d/abc123?var-namespace={{.Labels.namespace}}&var-alert={{.Labels.alertname}}"
```

**Grafana Panel Link:**
```yaml
link_template: "https://grafana.example.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}"
```

**Zabbix Problem Link:**
```yaml
link_template: "https://zabbix.example.com/zabbix.php?action=problem.view&filter_eventid={{.Labels.eventid}}"
```

**Custom Alert URL:**
```yaml
link_template: "https://my-monitoring.example.com/alerts/{{.Fingerprint}}"
```

**Multi-Variable Link:**
```yaml
link_template: "https://example.com/alerts?source={{.Source}}&severity={{.Severity}}&name={{.Name}}"
```

### Priority Order

AlertView tries link generation methods in this order:

1. **dashboard_url** from config
2. **link_template** from config (with variables replaced)
3. **generator_url** from Alertmanager/Grafana response
4. **Default URL** for Zabbix

### Testing Templates

You can test your templates using the `apply_link_template` function in Rust:

```rust
use alertview::alerts::{Alert, apply_link_template};
use std::collections::HashMap;

let mut labels = HashMap::new();
labels.insert("namespace".to_string(), "production".to_string());
labels.insert("alertname".to_string(), "HighCPU".to_string());

let alert = Alert {
    fingerprint: "test:123".to_string(),
    source: "Alertmanager".to_string(),
    source_type: "alertmanager".to_string(),
    status: "firing".to_string(),
    severity: "critical".to_string(),
    name: "HighCPU".to_string(),
    labels,
    annotations: HashMap::new(),
    starts_at: "2024-01-01T00:00:00Z".to_string(),
    ends_at: None,
    link_url: None,
};

let template = "https://grafana.example.com/alerts?query={{.Labels.alertname}}";
let result = apply_link_template(template, &alert).unwrap();
// Result: "https://grafana.example.com/alerts?query=HighCPU"
```

### Common Use Cases

**1. Link to Grafana Dashboard:**
```yaml
link_template: "https://grafana.example.com/d/PROMETHEUS_ALERTS?var-alertname={{.Labels.alertname}}"
```

**2. Link to Prometheus Query:**
```yaml
link_template: "https://prometheus.example.com/graph?g0.range_input=1h&expr=ALERTS{{.Labels.alertname}}"
```

**3. Link to Runbook:**
```yaml
link_template: "https://wiki.example.com/runbooks/{{.Labels.alertname}}"
```

**4. Link with Multiple Variables:**
```yaml
link_template: "https://example.com/alerts?env={{.Labels.environment}}&app={{.Labels.app}}&severity={{.Severity}}"
```

## Performance Tuning

Optimize AlertView for your specific use case.

### For High-Traffic Deployments

```yaml
# Reduce refresh interval
refresh_interval: 15

# Enable caching
cache_ttl_seconds: 30

# Increase timeouts for slow sources
sources:
  - name: "Alertmanager"
    timeout: 30
    retry_policy:
      max_retries: 5
      initial_delay_ms: 1000
      max_delay_ms: 30000
```

### For Low-Traffic Deployments

```yaml
# Increase refresh interval
refresh_interval: 60

# Disable caching (not worth the overhead)
cache_ttl_seconds: 0

# Reduce retries
sources:
  - name: "Alertmanager"
    timeout: 15
    retry_policy:
      max_retries: 2
      initial_delay_ms: 1000
      max_delay_ms: 10000
```

### For Unstable Sources

```yaml
# Long timeouts
sources:
  - name: "Unstable Source"
    timeout: 60
    retry_policy:
      max_retries: 10
      initial_delay_ms: 2000
      max_delay_ms: 120000
```

### For Fast, Reliable Sources

```yaml
# Short timeouts, few retries
sources:
  - name: "Fast Source"
    timeout: 10
    retry_policy:
      max_retries: 1
      initial_delay_ms: 500
      max_delay_ms: 5000
```

## Security Considerations

### TLS/SSL

**Always use HTTPS in production:**

```yaml
sources:
  - name: "Alertmanager"
    url: "https://alertmanager.example.com:9093"  # Note: https
    # Only set tls_insecure if you have self-signed certs
    tls_insecure: false  # Default: false
```

**For development with self-signed certificates:**
```yaml
tls_insecure: true  # Only for development!
```

### Authentication

**Prefer Bearer Tokens:**
```yaml
bearer_token: "${TOKEN}"  # Set via environment variable
```

**Basic Auth (if needed):**
```yaml
basic_auth:
  username: "${USERNAME}"
  password: "${PASSWORD}"
```

### Secrets Management

**❌ Never do this:**
```yaml
bearer_token: "my-secret-token"  # Hardcoded in config!
```

**✅ Do this instead:**
```yaml
bearer_token: "${ALERTMANAGER_TOKEN}"  # From environment
```

**Best practices:**
1. Use environment variables for secrets
2. Use a secrets manager (Vault, AWS Secrets Manager, etc.)
3. Restrict file permissions: `chmod 600 config.yaml`
4. Never commit secrets to version control
5. Rotate tokens regularly

### Network Security

1. **Firewall Rules**: Restrict access to AlertView
2. **Network Policies**: Limit which pods can access AlertView in Kubernetes
3. **HTTPS**: Always use HTTPS in production
4. **Authentication**: Consider adding authentication to AlertView (planned feature)
5. **Rate Limiting**: Consider adding rate limiting (planned feature)

### Data Security

- **No Persistence**: AlertView doesn't store any data between restarts
- **In-Memory Only**: All data is in RAM, not on disk
- **Read-Only**: AlertView only reads from sources, never writes
- **No Logging of Sensitive Data**: Tokens and passwords are not logged

## Monitoring AlertView

### Health Check

```bash
curl http://alertview:8080/health
# Returns: OK
```

### Logs

**Text format (default):**
```bash
RUST_LOG=info cargo run -- config.yaml
```

**JSON format:**
```bash
RUST_LOG=info ALERTVIEW_LOG_FORMAT=json cargo run -- config.yaml
```

**Debug logging:**
```bash
RUST_LOG=debug cargo run -- config.yaml
```

### Metrics (Planned)

Future versions will include a `/metrics` endpoint with Prometheus metrics:
- `alertview_up` - Whether AlertView is running
- `alertview_alerts_total` - Total number of alerts
- `alertview_sources_up` - Number of healthy sources
- `alertview_fetch_duration_seconds` - Time to fetch from each source
- `alertview_cache_hits_total` - Number of cache hits

### AlertView as a Source

You can monitor AlertView itself by:
1. Setting up a health check in your monitoring system
2. Monitoring the `/health` endpoint
3. Parsing logs for errors
4. Checking response times

Example Prometheus alert rule:
```yaml
- alert: AlertViewDown
  expr: up{job="alertview"} == 0
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "AlertView is down"
    description: "AlertView has been down for 5 minutes"
```
