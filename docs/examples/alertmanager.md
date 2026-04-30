# Alertmanager Configuration

This guide covers configuring AlertView to work with Alertmanager, including advanced options and best practices.

## Basic Configuration

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093

port: 8080
```

## Complete Configuration

```yaml
sources:
  - name: production-alertmanager
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    
    # Connection settings
    timeout: 30
    
    # Retry settings
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    
    # Caching
    cache_ttl: 60
    
    # Link template for alert URLs
    link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
    
    # TLS settings (optional)
    tls:
      skip_verify: false
      # ca_certificate: /path/to/ca.crt
    
    # Authentication (optional)
    # basic_auth:
    #   username: user
    #   password: pass
    # bearer_token: "token"

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC
  play_sounds: true

port: 8080
log_format: json
```

## Connection Settings

### URL

The URL to your Alertmanager API endpoint:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
```

Alertmanager API endpoints:
- `/api/v2/alerts` - Get all alerts (default)
- `/api/v2/alerts?active=true` - Get active alerts only
- `/api/v2/alerts?silenced=true` - Get silenced alerts
- `/api/v2/alerts?inhibited=true` - Get inhibited alerts

### Timeout

Request timeout in seconds (default: 15):

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    timeout: 30  # 30 second timeout
```

## Authentication

### Basic Authentication

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    basic_auth:
      username: api-user
      password: api-password
```

Or via environment variables:

```bash
export ALERTMANAGER_BASIC_AUTH=api-user:api-password
```

### Bearer Token Authentication

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    bearer_token: "your-bearer-token"
```

Or via environment variables:

```bash
export ALERTMANAGER_BEARER_TOKEN=your-bearer-token
```

### TLS/HTTPS

#### Skip Certificate Verification (Not Recommended)

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    tls:
      skip_verify: true
```

#### Custom CA Certificate

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    tls:
      ca_certificate: /path/to/ca.crt
```

## Retry Policy

Configure how AlertView retries failed requests:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    retry_policy:
      max_retries: 5          # Maximum number of retries (default: 3)
      initial_delay_ms: 1000  # Initial delay in milliseconds (default: 1000)
      max_delay_ms: 30000    # Maximum delay in milliseconds (default: 10000)
```

**How it works:**
1. First retry: wait 1 second
2. Second retry: wait 2 seconds
3. Third retry: wait 4 seconds
4. Fourth retry: wait 8 seconds
5. Fifth retry: wait 16 seconds (capped at max_delay_ms = 30s)

## Caching

Enable caching to reduce load on Alertmanager:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    cache_ttl: 60  # Cache for 60 seconds

# Global cache TTL (applies to all sources without explicit cache_ttl)
cache_ttl: 30
```

**Note:** Caching is disabled by default (`cache_ttl: 0`).

## Link Templates

Customize the URL that opens when clicking an alert:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}/{{.Labels.panel}}?viewPanel={{.Labels.panel_id}}"
```

**Available template variables:**
- `{{.Labels.<name>}}` - Alert labels (e.g., `{{.Labels.alertname}}`, `{{.Labels.severity}}`)
- `{{.Annotations.<name>}}` - Alert annotations (e.g., `{{.Annotations.summary}}`, `{{.Annotations.description}}`)
- `{{.Source}}` - Source name
- `{{.Id}}` - Alert ID

**Examples:**

```yaml
# Simple dashboard link
link_template: "https://grafana.example.com/d/abc123"

# Dashboard with alert name
link_template: "https://grafana.example.com/d/abc123?var-alert={{.Labels.alertname}}"

# Direct link to alert in Alertmanager
link_template: "https://alertmanager.example.com/#/alerts?search={{.Labels.alertname}}"

# Multi-variable template
link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}/{{.Labels.panel}}?from=now-1h&to=now&var-service={{.Labels.service}}"
```

## Multiple Alertmanager Sources

Monitor multiple Alertmanager instances:

```yaml
sources:
  - name: production
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    
  - name: staging
    kind: alertmanager
    url: https://alertmanager.staging.example.com
    timeout: 15
    
  - name: development
    kind: alertmanager
    url: http://localhost:9093
    timeout: 10

display:
  refresh_interval: 30

port: 8080
```

## Filtering Alerts

Filter alerts at the source level (before they reach AlertView):

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093/api/v2/alerts?active=true  # Only active alerts
    
  - name: alertmanager-silenced
    kind: alertmanager
    url: http://localhost:9093/api/v2/alerts?silenced=true  # Only silenced alerts
```

**Note:** AlertView also supports client-side filtering via the `filters` option in the `display` section.

## Alertmanager API Version

AlertView works with Alertmanager API v2. If you're using an older version:

- **Alertmanager v0.21+**: Uses API v2 (recommended)
- **Alertmanager v0.15 - v0.20**: Uses API v1 (deprecated)

To use API v1 (not recommended):

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093/api/v1/alerts
```

## Alertmanager with Prometheus

If you're using Alertmanager with Prometheus, you can link directly to Prometheus queries:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://prometheus.example.com/graph?g0.range_input=1h&g0.expr={{.Annotations.promql}}&g0.tab=0"
```

## Alertmanager in Kubernetes

### Accessing Alertmanager in the Same Cluster

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://alertmanager-operated:9093
```

### Accessing Alertmanager with Service Account

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    bearer_token: "your-service-account-token"
```

Or use Kubernetes service account token:

```bash
# Mount the service account token
kubectl create secret generic alertview-token --from-file=token=/var/run/secrets/kubernetes.io/serviceaccount/token

# Then reference it in your deployment
```

## Best Practices

### Production Configuration

```yaml
sources:
  - name: production-alertmanager
    kind: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 30000
    cache_ttl: 60
    link_template: "https://grafana.prod.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
    tls:
      skip_verify: false
    bearer_token: "your-production-token"

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC
  play_sounds: true

port: 8080
log_format: json
```

### Development Configuration

```yaml
sources:
  - name: local-alertmanager
    kind: alertmanager
    url: http://localhost:9093
    timeout: 10
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 2000
    cache_ttl: 0  # Disable caching in development

display:
  refresh_interval: 10
  theme: light
  timezone: local
  play_sounds: false

port: 8080
log_format: text
```

### High Availability Configuration

```yaml
sources:
  - name: alertmanager-primary
    kind: alertmanager
    url: https://alertmanager-primary.example.com
    timeout: 30
    
  - name: alertmanager-secondary
    kind: alertmanager
    url: https://alertmanager-secondary.example.com
    timeout: 30

display:
  refresh_interval: 15

port: 8080
```

## Troubleshooting

### Connection Issues

**Symptom:** AlertView can't connect to Alertmanager

**Checks:**
1. Verify Alertmanager is running: `curl -I http://localhost:9093`
2. Check network connectivity from AlertView to Alertmanager
3. Verify authentication credentials
4. Check TLS/HTTPS configuration

**Debug:**

```bash
# Enable debug logging
RUST_LOG=debug alertview --config config.yaml

# Test connectivity manually
curl -v http://localhost:9093/api/v2/alerts
curl -u username:password http://localhost:9093/api/v2/alerts
```

### Authentication Issues

**Symptom:** 401 Unauthorized errors

**Checks:**
1. Verify username and password
2. Check if basic auth is enabled in Alertmanager
3. Verify bearer token is valid

**Alertmanager basic auth configuration:**

```yaml
# alertmanager.yml
route:
  receiver: 'default'

receivers:
- name: 'default'

# Enable basic auth
web:
  basic_auth_users:
    api-user: '$2y$10$hashed-password'
```

### Timeout Issues

**Symptom:** Requests timeout before completing

**Solutions:**
1. Increase the timeout value
2. Check Alertmanager performance
3. Reduce the number of alerts being fetched

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://slow-alertmanager.example.com
    timeout: 120  # 2 minute timeout
```

### SSL/TLS Issues

**Symptom:** SSL certificate errors

**Solutions:**
1. Use valid certificates
2. Configure custom CA if using self-signed certificates
3. Temporarily disable verification for testing (not recommended for production)

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: https://alertmanager.example.com
    tls:
      skip_verify: true  # Only for testing!
```

## Alertmanager-Specific Features

### Silenced Alerts

Alertmanager supports silencing alerts. AlertView will display silenced alerts with a special indicator.

To fetch only silenced alerts:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093/api/v2/alerts?silenced=true
```

### Inhibited Alerts

Alertmanager can inhibit certain alerts based on other alerts. AlertView displays inhibited alerts with a special indicator.

To fetch only inhibited alerts:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093/api/v2/alerts?inhibited=true
```

### Alert States

AlertView displays the following Alertmanager states:
- `firing`: Alert is currently active
- `resolved`: Alert has been resolved
- `silenced`: Alert has been silenced

### Alert Generator URL

AlertView includes the `generator_url` from Alertmanager, which links back to the source of the alert (typically Prometheus).

## Performance Considerations

### Reducing Load on Alertmanager

1. **Enable caching**: Set `cache_ttl` to a reasonable value
2. **Increase refresh interval**: Reduce how often AlertView fetches alerts
3. **Filter at the source**: Use Alertmanager API filters to reduce response size

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093/api/v2/alerts?active=true  # Only active alerts
    cache_ttl: 60

display:
  refresh_interval: 60
```

### Handling Large Numbers of Alerts

1. **Use pagination**: Alertmanager API supports pagination
2. **Filter alerts**: Only fetch alerts for specific teams or services
3. **Increase timeouts**: Give AlertView more time to process large responses

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093/api/v2/alerts?filter=team=frontend
    timeout: 60
```

## Integration with Other Tools

### Prometheus

Link AlertView alerts to Prometheus queries:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://prometheus.example.com/graph?g0.range_input=1h&g0.expr={{.Annotations.promql}}"
```

### Grafana

Link AlertView alerts to Grafana dashboards:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://grafana.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
```

### PagerDuty

If you use PagerDuty with Alertmanager, you can link to PagerDuty incidents:

```yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093
    link_template: "https://your-account.pagerduty.com/incidents/{{.Annotations.pagerduty_incident_id}}"
```

## Alertmanager Configuration for AlertView

To optimize Alertmanager for use with AlertView:

```yaml
# alertmanager.yml
route:
  receiver: 'alertview'
  
  # Group alerts for better display in AlertView
  group_by: ['alertname', 'severity']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 3h

receivers:
- name: 'alertview'
  # No need for webhook config - AlertView polls the API

# Enable API
web:
  expose: true
  
# Optional: Enable basic auth for API
web:
  basic_auth_users:
    alertview: '$2y$10$hashed-password'
```

## Additional Resources

- [Alertmanager Documentation](https://prometheus.io/docs/alerting/latest/alertmanager/)
- [Alertmanager API Documentation](https://prometheus.io/docs/alerting/latest/api/)
- [Prometheus Alerting Rules](https://prometheus.io/docs/prometheus/latest/configuration/alerting_rules/)
- [AlertView Configuration Reference](../configuration/config-file.md)
