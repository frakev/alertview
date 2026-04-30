# Grafana Configuration

This guide covers configuring AlertView to work with Grafana's alerting system.

## Basic Configuration

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"

port: 8080
```

## Complete Configuration

```yaml
sources:
  - name: grafana-production
    kind: grafana
    url: https://grafana.prod.example.com
    api_key: "your-production-api-key"
    
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
    link_template: "https://grafana.example.com/d/{{.DashboardUID}}/{{.PanelID}}?viewPanel={{.PanelID}}&orgId={{.OrgID}}&from=now-1h&to=now"
    
    # TLS settings (optional)
    tls:
      skip_verify: false
    
    # Folder ID to fetch alerts from (optional)
    folder_id: 0  # 0 = root folder

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC
  play_sounds: true

port: 8080
log_format: json
```

## Authentication

Grafana requires an API key for authentication. You can create one in Grafana:

1. Go to Configuration > API Keys
2. Click "Add API Key"
3. Enter a name (e.g., "AlertView")
4. Select the "Admin" role (or a role with read access to alerts)
5. Set an optional expiration date
6. Click "Add"

### Using API Key in Configuration

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
```

### Using API Key via Environment Variable

```bash
export GRAFANA_API_KEY=your-api-key
```

Then in your config:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    # api_key will be read from environment variable
```

## Grafana Alerting API

AlertView uses Grafana's HTTP API to fetch alerts:

- **Grafana v8+**: `/api/v1/alerts` (Unified alerting)
- **Grafana v7**: `/api/alerts` (Legacy alerting)

AlertView automatically detects which API version to use based on the Grafana version.

### Unified Alerting (Grafana v8+)

Grafana v8 introduced a new unified alerting system. AlertView supports both:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    # Uses /api/v1/alerts for Grafana v8+
```

### Legacy Alerting (Grafana v7)

For Grafana v7, AlertView uses the legacy API:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    # Uses /api/alerts for Grafana v7
```

## Folder Support

Grafana organizes dashboards and alerts into folders. You can specify which folder to fetch alerts from:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    folder_id: 123  # Specific folder ID
```

To get folder IDs:

1. Go to Dashboards in Grafana
2. Click on a folder
3. The URL will contain the folder ID: `/dashboards/folder/<folder-id>`

Or use the API:

```bash
curl -H "Authorization: Bearer your-api-key" \
  https://grafana.example.com/api/folders
```

### Multiple Folders

To monitor alerts from multiple folders, create multiple Grafana sources:

```yaml
sources:
  - name: grafana-frontend
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    folder_id: 1  # Frontend folder
    
  - name: grafana-backend
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    folder_id: 2  # Backend folder
```

## Link Templates

Customize the URL that opens when clicking a Grafana alert:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    link_template: "https://grafana.example.com/d/{{.DashboardUID}}/{{.PanelID}}?viewPanel={{.PanelID}}&orgId={{.OrgID}}&from=now-1h&to=now"
```

**Available template variables for Grafana alerts:**
- `{{.DashboardUID}}` - Dashboard UID
- `{{.DashboardName}}` - Dashboard name
- `{{.PanelID}}` - Panel ID
- `{{.PanelTitle}}` - Panel title
- `{{.OrgID}}` - Organization ID
- `{{.RuleName}}` - Alert rule name
- `{{.RuleUID}}` - Alert rule UID
- `{{.FolderUID}}` - Folder UID
- `{{.FolderName}}` - Folder name
- `{{.Labels.<name>}}` - Alert labels
- `{{.Annotations.<name>}}` - Alert annotations

**Examples:**

```yaml
# Simple dashboard link
link_template: "https://grafana.example.com/d/{{.DashboardUID}}"

# Dashboard with time range
link_template: "https://grafana.example.com/d/{{.DashboardUID}}?from=now-1h&to=now"

# Direct link to panel
link_template: "https://grafana.example.com/d/{{.DashboardUID}}/{{.PanelID}}?viewPanel={{.PanelID}}"

# With organization
link_template: "https://grafana.example.com/d/{{.DashboardUID}}?orgId={{.OrgID}}"
```

## Alert States

Grafana alerts can have the following states:
- `normal` - Alert is not firing
- `alerting` - Alert is firing
- `paused` - Alert is paused
- `unknown` - Alert state is unknown

AlertView maps these to:
- `firing` - For `alerting` state
- `resolved` - For `normal` state
- `paused` - For `paused` state
- `unknown` - For `unknown` state

## Retry Policy

Configure how AlertView retries failed requests to Grafana:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 30000
```

## Caching

Enable caching to reduce load on Grafana:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    cache_ttl: 60  # Cache for 60 seconds

# Global cache TTL
cache_ttl: 30
```

## Timeout

Set request timeout for Grafana API calls:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    timeout: 30  # 30 second timeout
```

## TLS/HTTPS

### Skip Certificate Verification (Not Recommended)

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    tls:
      skip_verify: true
```

### Custom CA Certificate

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    tls:
      ca_certificate: /path/to/ca.crt
```

## Multiple Grafana Sources

Monitor multiple Grafana instances:

```yaml
sources:
  - name: grafana-production
    kind: grafana
    url: https://grafana.prod.example.com
    api_key: "prod-api-key"
    timeout: 30
    
  - name: grafana-staging
    kind: grafana
    url: https://grafana.staging.example.com
    api_key: "staging-api-key"
    timeout: 15
    
  - name: grafana-local
    kind: grafana
    url: http://localhost:3000
    api_key: "local-api-key"
    timeout: 10

display:
  refresh_interval: 30

port: 8080
```

## Grafana Cloud

AlertView works with Grafana Cloud:

```yaml
sources:
  - name: grafana-cloud
    kind: grafana
    url: https://your-instance.grafana.net
    api_key: "your-cloud-api-key"
    timeout: 30
```

**Note:** Grafana Cloud URLs follow the pattern `https://<your-org>.grafana.net`.

## Best Practices

### Production Configuration

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.prod.example.com
    api_key: "your-production-api-key"
    timeout: 30
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 30000
    cache_ttl: 60
    link_template: "https://grafana.prod.example.com/d/{{.DashboardUID}}/{{.PanelID}}?viewPanel={{.PanelID}}&from=now-1h&to=now"

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC

port: 8080
log_format: json
```

### Development Configuration

```yaml
sources:
  - name: grafana
    kind: grafana
    url: http://localhost:3000
    api_key: "local-api-key"
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

port: 8080
log_format: text
```

## Troubleshooting

### Connection Issues

**Symptom:** AlertView can't connect to Grafana

**Checks:**
1. Verify Grafana is running: `curl -I https://grafana.example.com`
2. Check network connectivity from AlertView to Grafana
3. Verify API key is valid
4. Check if the API key has the correct permissions

**Debug:**

```bash
# Enable debug logging
RUST_LOG=debug alertview --config config.yaml

# Test connectivity manually
curl -H "Authorization: Bearer your-api-key" \
  https://grafana.example.com/api/health

# Test alerts API (Grafana v8+)
curl -H "Authorization: Bearer your-api-key" \
  https://grafana.example.com/api/v1/alerts

# Test alerts API (Grafana v7)
curl -H "Authorization: Bearer your-api-key" \
  https://grafana.example.com/api/alerts
```

### Authentication Issues

**Symptom:** 401 or 403 errors

**Checks:**
1. Verify API key is correct
2. Check if API key has expired
3. Verify API key has the correct role (Admin or Editor)
4. Check if the organization is correct

**Solution:**

Create a new API key with the correct permissions:

1. Go to Configuration > API Keys
2. Click "Add API Key"
3. Select "Admin" role
4. Set expiration if needed
5. Use the new API key

### API Version Issues

**Symptom:** Unexpected response format or errors

**Checks:**
1. Check Grafana version: `curl -H "Authorization: Bearer your-api-key" https://grafana.example.com/api/health`
2. Verify which API version is available

**Solution:**

AlertView automatically detects the Grafana version and uses the appropriate API. If you're having issues:

1. Update Grafana to the latest version
2. Update AlertView to the latest version
3. Check the Grafana API documentation for your version

### Timeout Issues

**Symptom:** Requests timeout before completing

**Solutions:**
1. Increase the timeout value
2. Check Grafana performance
3. Reduce the number of alerts being fetched

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://slow-grafana.example.com
    api_key: "your-api-key"
    timeout: 120  # 2 minute timeout
```

## Performance Considerations

### Reducing Load on Grafana

1. **Enable caching**: Set `cache_ttl` to a reasonable value
2. **Increase refresh interval**: Reduce how often AlertView fetches alerts
3. **Filter by folder**: Only fetch alerts from specific folders

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    folder_id: 123  # Specific folder
    cache_ttl: 60

display:
  refresh_interval: 60
```

### Handling Large Numbers of Alerts

1. **Use folder filtering**: Only fetch alerts from specific folders
2. **Increase timeouts**: Give AlertView more time to process large responses
3. **Enable caching**: Reduce API calls

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    folder_id: 1  # Only one folder
    timeout: 60
    cache_ttl: 120
```

## Integration with Other Tools

### Prometheus

Link AlertView alerts to Prometheus queries (if Grafana alerts are based on Prometheus):

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    link_template: "https://prometheus.example.com/graph?g0.range_input=1h&g0.expr={{.Annotations.promql}}"
```

### Alertmanager

If you use both Grafana and Alertmanager, you can link them together:

```yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"
    link_template: "https://alertmanager.example.com/#/alerts?search={{.RuleName}}"
```

## Grafana Alerting Best Practices

### Organizing Alerts

1. **Use folders**: Organize alerts by team, service, or environment
2. **Use labels**: Add labels to alerts for better filtering and grouping
3. **Use annotations**: Add descriptive annotations to alerts

### Alert Naming

Use descriptive names for alerts:
- `HighCPUUsage` instead of `CPU Alert`
- `ServiceDown` instead of `Alert 1`
- `DatabaseConnectionFailed` instead of `DB Alert`

### Alert Grouping

Group related alerts together in Grafana:
- Group by service
- Group by team
- Group by environment

## Additional Resources

- [Grafana Documentation](https://grafana.com/docs/)
- [Grafana Alerting Documentation](https://grafana.com/docs/grafana/latest/alerting/)
- [Grafana HTTP API](https://grafana.com/docs/grafana/latest/developers/http_api/)
- [Grafana API Keys](https://grafana.com/docs/grafana/latest/administration/api-keys/)
- [AlertView Configuration Reference](../configuration/config-file.md)
