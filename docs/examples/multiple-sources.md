# Multiple Sources Configuration

This guide covers configuring AlertView to aggregate alerts from multiple monitoring systems.

## Basic Multiple Sources

```yaml
sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    
  - name: grafana
    type: grafana
    url: https://grafana.example.com
    bearer_token: "your-api-key"
    
  - name: zabbix
    type: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password

port: 8080
```

## Production Multi-Source Setup

```yaml
sources:
  # Production Alertmanager
  - name: prod-alertmanager
    type: alertmanager
    url: https://alertmanager.prod.example.com
    timeout: 30
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    cache_ttl: 60
    link_template: "https://grafana.prod.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
    
  # Production Grafana
  - name: prod-grafana
    type: grafana
    url: https://grafana.prod.example.com
    bearer_token: "prod-api-key"
    timeout: 30
    cache_ttl: 60
    link_template: "https://grafana.prod.example.com/d/{{.DashboardUID}}/{{.PanelID}}?viewPanel={{.PanelID}}"
    
  # Staging Alertmanager
  - name: staging-alertmanager
    type: alertmanager
    url: https://alertmanager.staging.example.com
    timeout: 15
    cache_ttl: 30
    
  # Development Zabbix
  - name: dev-zabbix
    type: zabbix
    url: http://zabbix.dev.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    timeout: 10
    cache_ttl: 30

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC
  play_sounds: true
  
  # Default filters (applied to all sources)
  filters:
    state: firing
  
  # Default sort
  sort:
    by: starts_at
    order: desc

port: 8080
log_format: json
```

## Organizing Sources

### By Environment

```yaml
sources:
  - name: production
    type: alertmanager
    url: https://alertmanager.prod.example.com
    
  - name: staging
    type: alertmanager
    url: https://alertmanager.staging.example.com
    
  - name: development
    type: alertmanager
    url: http://localhost:9093
```

### By Team

```yaml
sources:
  - name: frontend
    type: alertmanager
    url: https://alertmanager.example.com
    link_template: "https://grafana.example.com/d/frontend-dashboard?var-team=frontend"
    
  - name: backend
    type: alertmanager
    url: https://alertmanager.example.com
    link_template: "https://grafana.example.com/d/backend-dashboard?var-team=backend"
    
  - name: infrastructure
    type: alertmanager
    url: https://alertmanager.example.com
    link_template: "https://grafana.example.com/d/infra-dashboard?var-team=infra"
```

### By Service

```yaml
sources:
  - name: web-service
    type: alertmanager
    url: https://alertmanager.example.com
    
  - name: api-service
    type: alertmanager
    url: https://alertmanager.example.com
    
  - name: database
    type: alertmanager
    url: https://alertmanager.example.com
```

## Source-Specific Configuration

Each source can have its own configuration:

```yaml
sources:
  - name: fast-source
    type: alertmanager
    url: http://localhost:9093
    timeout: 5
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 2000
    cache_ttl: 10
    
  - name: slow-source
    type: alertmanager
    url: https://remote.example.com:9093
    timeout: 60
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 30000
    cache_ttl: 120
    
  - name: unreliable-source
    type: alertmanager
    url: https://unreliable.example.com:9093
    timeout: 30
    retry_policy:
      max_retries: 10
      initial_delay_ms: 1000
      max_delay_ms: 60000
    cache_ttl: 30
```

## Link Templates for Multiple Sources

Customize link templates for each source:

```yaml
sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    link_template: "https://grafana.example.com/d/alertmanager-dashboard?var-alert={{.Labels.alertname}}"
    
  - name: grafana
    type: grafana
    url: https://grafana.example.com
    bearer_token: "your-api-key"
    link_template: "https://grafana.example.com/d/{{.DashboardUID}}/{{.PanelID}}?viewPanel={{.PanelID}}"
    
  - name: zabbix
    type: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    link_template: "https://zabbix.example.com/monitoring.php?triggerid={{.TriggerID}}"
```

## Display Configuration for Multiple Sources

```yaml
sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    
  - name: grafana
    type: grafana
    url: https://grafana.example.com
    bearer_token: "your-api-key"

display:
  # Refresh interval in seconds
  refresh_interval: 30
  
  # Theme: dark, light, or custom CSS URL
  theme: dark
  
  # Timezone: UTC, local, or IANA timezone
  timezone: America/New_York
  
  # Enable sound notifications
  play_sounds: true
  
  # Default filters (applied to all sources)
  filters:
    severity: [critical, warning]
    state: firing
  
  # Default sort
  sort:
    by: starts_at  # or: starts_at, severity, source, state
    order: desc    # or: asc
  
  # Compact mode for smaller displays
  compact_mode: false
  
  # Hide header and footer
  hide_header: false
  hide_footer: false
```

## Grouping Alerts from Multiple Sources

AlertView can group alerts by various fields:

```yaml
sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    
  - name: grafana
    type: grafana
    url: https://grafana.example.com
    bearer_token: "your-api-key"

display:
  # Group alerts by these fields
  group_by:
    - alertname
    - service
    - team
  
  # Sort groups by
  group_sort:
    by: severity
    order: desc
```

## Filtering Alerts from Multiple Sources

Apply filters globally or per-source:

```yaml
sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    # Source-specific filter (if supported by the source)
    
  - name: grafana
    type: grafana
    url: https://grafana.example.com
    bearer_token: "your-api-key"

display:
  # Global filters (applied to all sources after fetching)
  filters:
    severity: [critical, warning]
    state: firing
    team: [frontend, backend]
```

## Performance Considerations

### Parallel Fetching

AlertView fetches alerts from all sources in parallel for maximum performance.

### Caching Strategy

```yaml
# Global cache TTL (applies to sources without explicit cache_ttl)
cache_ttl: 60

sources:
  - name: fast-source
    type: alertmanager
    url: http://localhost:9093
    cache_ttl: 30  # Override global
    
  - name: slow-source
    type: alertmanager
    url: https://remote.example.com:9093
    cache_ttl: 120  # Override global
```

### Refresh Interval

```yaml
# Global refresh interval
display:
  refresh_interval: 30

# Sources with different update frequencies
sources:
  - name: critical-source
    type: alertmanager
    url: https://critical.example.com:9093
    # Will be refreshed every 30 seconds (global)
    
  - name: less-important-source
    type: alertmanager
    url: https://less-important.example.com:9093
    # Will also be refreshed every 30 seconds
```

**Note:** All sources are refreshed at the same interval defined in `display.refresh_interval`.

### Reducing Load

To reduce load on your monitoring systems:

1. **Enable caching**: Set appropriate `cache_ttl` values
2. **Increase refresh interval**: Reduce how often alerts are fetched
3. **Filter at source**: Use source-specific filters to reduce response size
4. **Use timeouts**: Set appropriate timeouts for each source

```yaml
cache_ttl: 120  # Global cache

display:
  refresh_interval: 60  # Refresh every minute

sources:
  - name: alertmanager
    type: alertmanager
    url: http://localhost:9093
    timeout: 30
    cache_ttl: 120
    
  - name: grafana
    type: grafana
    url: https://grafana.example.com
    bearer_token: "your-api-key"
    timeout: 30
    cache_ttl: 120
```

## High Availability Setup

Monitor multiple instances of the same type for redundancy:

```yaml
sources:
  # Primary Alertmanager
  - name: alertmanager-primary
    type: alertmanager
    url: https://alertmanager-primary.example.com
    timeout: 30
    
  # Secondary Alertmanager
  - name: alertmanager-secondary
    type: alertmanager
    url: https://alertmanager-secondary.example.com
    timeout: 30
    
  # Tertiary Alertmanager
  - name: alertmanager-tertiary
    type: alertmanager
    url: https://alertmanager-tertiary.example.com
    timeout: 30

display:
  refresh_interval: 15
  theme: dark

port: 8080
```

## Multi-Region Setup

Monitor alerts from different regions:

```yaml
sources:
  - name: us-east-1
    type: alertmanager
    url: https://alertmanager.us-east-1.example.com
    link_template: "https://grafana.us-east-1.example.com/d/{{.Labels.dashboard}}?var-region=us-east-1"
    
  - name: us-west-2
    type: alertmanager
    url: https://alertmanager.us-west-2.example.com
    link_template: "https://grafana.us-west-2.example.com/d/{{.Labels.dashboard}}?var-region=us-west-2"
    
  - name: eu-west-1
    type: alertmanager
    url: https://alertmanager.eu-west-1.example.com
    link_template: "https://grafana.eu-west-1.example.com/d/{{.Labels.dashboard}}?var-region=eu-west-1"

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC

port: 8080
```

## Troubleshooting Multiple Sources

### Check Which Sources Are Working

```bash
# Enable debug logging to see which sources are being fetched
RUST_LOG=debug alertview --config config.yaml
```

### Test Individual Sources

```bash
# Test Alertmanager
curl -v http://localhost:9093/api/v2/alerts

# Test Grafana
curl -H "Authorization: Bearer your-api-key" \
  https://grafana.example.com/api/v1/alerts

# Test Zabbix
curl -v -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"user.login","params":{"user":"api-user","password":"api-password"},"id":1}' \
  https://zabbix.example.com/api_jsonrpc.php
```

### Common Issues

**Issue: Some sources are slow**
- Solution: Increase timeout for slow sources
- Solution: Enable caching for slow sources

**Issue: Some sources are unreliable**
- Solution: Increase retry count for unreliable sources
- Solution: Increase retry delays

**Issue: Too many alerts**
- Solution: Enable filtering at the source level
- Solution: Use global filters in display configuration

**Issue: Sources have different alert formats**
- Solution: AlertView normalizes all alerts to a common format
- Solution: Use link templates to customize URLs for each source

## Best Practices

### Naming Conventions

Use clear, descriptive names for sources:

```yaml
sources:
  - name: prod-us-east-1-alertmanager
    type: alertmanager
    url: https://alertmanager.prod.us-east-1.example.com
    
  - name: staging-eu-west-1-grafana
    type: grafana
    url: https://grafana.staging.eu-west-1.example.com
    bearer_token: "staging-api-key"
```

### Source Organization

Group related sources together in the configuration:

```yaml
sources:
  # Production sources
  - name: prod-alertmanager
    type: alertmanager
    url: https://alertmanager.prod.example.com
    
  - name: prod-grafana
    type: grafana
    url: https://grafana.prod.example.com
    bearer_token: "prod-api-key"
  
  # Staging sources
  - name: staging-alertmanager
    type: alertmanager
    url: https://alertmanager.staging.example.com
    
  - name: staging-grafana
    type: grafana
    url: https://grafana.staging.example.com
    bearer_token: "staging-api-key"
```

### Configuration Management

For complex setups with many sources:

1. **Use environment variables** for sensitive data
2. **Use config includes** (if supported) to split configuration
3. **Use templates** to generate configuration
4. **Document your configuration** with comments

```yaml
# Production Alertmanager - Primary region
- name: prod-primary-alertmanager
  type: alertmanager
  url: https://alertmanager.prod.primary.example.com
  timeout: 30
  retry_policy:
    max_retries: 3
  cache_ttl: 60
  # API key is set via ALERTMANAGER_PROD_PRIMARY_API_KEY environment variable

# Production Alertmanager - Secondary region
- name: prod-secondary-alertmanager
  type: alertmanager
  url: https://alertmanager.prod.secondary.example.com
  timeout: 30
  retry_policy:
    max_retries: 3
  cache_ttl: 60
  # API key is set via ALERTMANAGER_PROD_SECONDARY_API_KEY environment variable
```

## Additional Resources

- [Alertmanager Configuration](alertmanager.md)
- [Grafana Configuration](grafana.md)
- [Zabbix Configuration](zabbix.md)
- [Configuration Reference](../configuration/config-file.md)
