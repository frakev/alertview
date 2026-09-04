# Source Types

AlertView supports three types of alert sources: Alertmanager, Grafana, and Zabbix. This guide explains how to configure each type.

## Common Configuration

All source types share these common fields:

```yaml
- name: "Source Name"        # Required: Unique identifier
  type: alertmanager         # Required: Source type
  url: "http://host:port"    # Required: Base URL
  timeout: 15               # Optional: Request timeout in seconds (default: 15)
  dashboard_url: "..."       # Optional: Link for alert cards
  link_template: "..."      # Optional: Template for the ↗ source link
  alert_link_template: "..." # Optional: Turns the severity dot into a link
  source_link: true         # Optional: Show the ↗ source link for this source
  retry_policy:             # Optional: Retry configuration
    max_retries: 3
    initial_delay_ms: 1000
    max_delay_ms: 30000
```

## Alertmanager

Alertmanager is the most common alert source and is natively supported by AlertView.

### Configuration

```yaml
- name: "Alertmanager"
  type: alertmanager
  url: "http://alertmanager.example.com:9093"
  
  # Optional: Authentication
  basic_auth:
    username: "user"
    password: "password"
  
  # or
  bearer_token: "your-token"
  
  # Optional: Display settings
  dashboard_url: "https://grafana.example.com/alerting/list"
  link_template: "https://grafana.example.com/alerts?query={{.Labels.alertname}}"
  
  # Optional: Performance settings
  timeout: 30
  retry_policy:
    max_retries: 5
    initial_delay_ms: 2000
    max_delay_ms: 60000
```

### API Details

- **Endpoint**: `{url}/api/v2/alerts`
- **Method**: GET
- **Authentication**: Basic Auth or Bearer Token
- **Response Format**: Alertmanager API v2

### Example Alert

```json
{
  "labels": {
    "alertname": "HighCPU",
    "severity": "critical",
    "namespace": "production",
    "job": "node-exporter"
  },
  "annotations": {
    "summary": "High CPU usage",
    "description": "CPU usage is above 90% for 5 minutes"
  },
  "state": "active",
  "startsAt": "2024-01-01T10:00:00Z",
  "endsAt": "0001-01-01T00:00:00Z",
  "generatorURL": "http://prometheus:9090/graph?g0.range_input=1h..."
}
```

### Silence Comments

When alerts are silenced in Alertmanager, AlertView fetches the silence information and includes the comment in the annotations:
- `silence_comment`: The comment/message from the silence that silenced this alert
- `silence_created_by`: Who created that silence (the silence's `createdBy`)

**Malformed alerts.** Alerts are parsed one at a time: an entry AlertView
cannot read is logged and skipped, and the rest of the payload is shown. Only
`fingerprint` is required; everything else falls back to a default, so a
minimal alert still appears rather than taking the whole source down.

If the silence cannot be fetched or has no comment, a default message "Silenced in Alertmanager" is used.

### Troubleshooting

**Connection refused:**
- Verify Alertmanager is running
- Check the URL and port
- Test with: `curl http://alertmanager:9093/api/v2/alerts`

**Authentication failed:**
- Verify username/password or token
- Check if authentication is required
- Test with: `curl -u user:pass http://alertmanager:9093/api/v2/alerts`

**No alerts returned:**
- Check if there are active alerts in Alertmanager
- Verify filters in Alertmanager
- Test with: `curl http://alertmanager:9093/api/v2/alerts | jq`

## Grafana

Grafana can act as an alert source if it has the Alertmanager API enabled (which is the default in Grafana 7.0+).

### Configuration

```yaml
- name: "Grafana"
  type: grafana
  url: "http://grafana.example.com:3000"
  
  # Required: Bearer token (Service Account recommended)
  bearer_token: "glsa_xxxxx"
  
  # Optional: Basic auth (alternative to bearer token)
  basic_auth:
    username: "admin"
    password: "secret"
  
  # Optional: Display settings
  dashboard_url: "https://grafana.example.com/"
  link_template: "https://grafana.example.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}"
  
  # Optional: Performance settings
  timeout: 30
```

### API Details

- **Endpoint**: `{url}/api/alertmanager/grafana/api/v2/alerts`
- **Method**: GET
- **Authentication**: Bearer Token (recommended) or Basic Auth
- **Response Format**: Alertmanager API v2 (same as Alertmanager)

### Setting Up Grafana

1. **Create a Service Account** (recommended):
   - Go to Configuration → Service Accounts
   - Create a new service account
   - Assign the "Admin" role (or create a custom role with read access to alerts)
   - Copy the token

2. **Enable Alertmanager API** (usually enabled by default):
   - Go to Configuration → Settings → Alerting
   - Ensure "Alertmanager" is enabled

3. **Verify the API**:
   ```bash
   curl -H "Authorization: Bearer YOUR_TOKEN" \
     http://grafana:3000/api/alertmanager/grafana/api/v2/alerts | jq
   ```

### Grafana-Specific Features

Grafana alerts include additional annotations that you can use in link templates:
- `dashboardUid` - The dashboard UID
- `panelId` - The panel ID
- `ruleName` - The alert rule name
- `ruleUrl` - URL to the alert rule

**Silence Comments:** Grafana uses Alertmanager API internally, so silenced alerts also include the `silence_comment` and `silence_created_by` annotations.

### Example Link Template

```yaml
link_template: "https://grafana.example.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}&var-alert={{.Labels.alertname}}"
```

## Zabbix

Zabbix integration uses the Zabbix JSON-RPC API to fetch problems (triggered events).

### Configuration

```yaml
- name: "Zabbix"
  type: zabbix
  url: "https://zabbix.example.com/zabbix"
  
  # Required: Zabbix API token
  bearer_token: "your-zabbix-api-token"
  
  # Optional: Display settings
  dashboard_url: "https://zabbix.example.com/zabbix/zabbix.php?action=problem.view"
  link_template: "https://zabbix.example.com/zabbix/zabbix.php?action=problem.view&triggerids[]={{.Labels.triggerid}}"
  
  # Optional: Performance settings
  timeout: 45  # Zabbix API can be slower
```

### API Details

- **Endpoint**: `{url}/api_jsonrpc.php`
- **Method**: POST
- **Authentication**: Bearer Token (Zabbix API token)
- **Response Format**: Zabbix JSON-RPC

### Setting Up Zabbix

1. **Create an API Token**:
   - Go to User Settings → API Tokens
   - Create a new token
   - Copy the token

2. **Verify the API**:
   ```bash
   curl -X POST \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"user.login","params":{"user":"Admin","password":"zabbix"},"id":1}' \
     http://zabbix:80/api_jsonrpc.php | jq
   ```

3. **Test fetching problems**:
   ```bash
   curl -X POST \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -d '{"jsonrpc":"2.0","method":"problem.get","params":{"output":"extend","selectTags":"extend"},"id":1}' \
     http://zabbix:80/api_jsonrpc.php | jq
   ```

### Zabbix-Specific Features

Zabbix alerts include these special fields:
- `alertname` - The problem name, under the same key Alertmanager and Grafana use, so one link template works for every source
- `eventid` - Unique event ID (used in links)
- `host` - Host name
- `hostgroup` - Host group
- `severity` - Zabbix severity (0-5, mapped to: none, info, warning, high, critical)
- `acknowledged` - Whether the alert is acknowledged in Zabbix ("0" or "1")
- `acknowledgement` - Acknowledgment message/comment from Zabbix (if available)
- `acknowledged_by` - User who acknowledged the alert
- `acknowledged_at` - Timestamp when the alert was acknowledged

**Alert Status Handling:**
- Alerts with `suppressed: "1"` (manually silenced in Zabbix) are displayed as **silenced**
- Alerts with `acknowledged: "1"` (ACK'd in Zabbix) are also displayed as **silenced**
- The acknowledgment message is available in the `acknowledgement` annotation
- User and timestamp are available in `acknowledged_by` and `acknowledged_at` labels

**Who acknowledged an alert.** An acknowledgement carries a `userid`, not a
name, so AlertView resolves it with one `user.get` per poll. An API token
without permission to read users simply leaves the author out — the message,
the timestamp and the alert itself are unaffected.

### Zabbix Version Compatibility

Tested against Zabbix **6.0 through 7.x**. Zabbix renamed two API parameters
along the way, and AlertView probes for them rather than pinning a version:

| API call | Modern name | Older name | Renamed in |
|---|---|---|---|
| `trigger.get` | `selectHostGroups` | `selectGroups` | 6.4 (deprecated), **removed in 7.0** |
| `problem.get` | `selectAcknowledgements` | `selectAcknowledges` | 6.0 |

The modern name is tried first. If the server answers "invalid params" — that
is, it does not know the parameter — AlertView retries with the older one and
remembers what worked for that source, so the extra round-trip happens once,
not on every poll. Any other error (authentication, permissions, HTTP,
network) is reported straight away instead of being retried under a different
name.

No configuration is needed for this: it applies to whatever version each
source turns out to be running.

### Severity Mapping

Zabbix severities are mapped to AlertView severities:

| Zabbix Severity | Zabbix Value | AlertView Severity |
|-----------------|--------------|-------------------|
| Not classified  | 0            | none              |
| Information     | 1            | info              |
| Warning         | 2            | warning           |
| Average         | 3            | warning           |
| High            | 4            | high              |
| Disaster        | 5            | critical          |

### Example Link Template

```yaml
# Link directly to the problem in Zabbix
link_template: "https://zabbix.example.com/zabbix.php?action=problem.view&triggerids[]={{.Labels.triggerid}}"
```

## Comparison Table

| Feature | Alertmanager | Grafana | Zabbix |
|---------|--------------|---------|--------|
| API Type | REST | REST | JSON-RPC |
| Auth Methods | Basic, Bearer | Basic, Bearer | Bearer |
| Default Port | 9093 | 3000 | 80/443 |
| Alert Format | Alertmanager v2 | Alertmanager v2 | Zabbix-specific |
| Severity | From labels | From labels | From priority |
| Status | From state | From state | From status |
| Links | generatorURL | generatorURL | problem.view URL built from the trigger id |
| Timeout Recommendation | 15s | 15s | 30-45s |

## Multiple Sources Example

```yaml
sources:
  # Alertmanager for Prometheus alerts
  - name: "Prometheus Alertmanager"
    type: alertmanager
    url: "http://alertmanager.prometheus:9093"
    timeout: 15

  # Grafana for dashboard alerts
  - name: "Grafana Alerts"
    type: grafana
    url: "http://grafana:3000"
    bearer_token: "${GRAFANA_TOKEN}"
    timeout: 20

  # Zabbix for infrastructure monitoring
  - name: "Zabbix"
    type: zabbix
    url: "https://zabbix.example.com/zabbix"
    bearer_token: "${ZABBIX_TOKEN}"
    timeout: 45
```

## Best Practices

1. **Use meaningful names**: Name your sources descriptively (e.g., "Production Alertmanager", "Staging Grafana")

2. **Set appropriate timeouts**:
   - Alertmanager/Grafana: 15-30 seconds
   - Zabbix: 30-45 seconds (Zabbix API can be slower)

3. **Use bearer tokens**: Prefer bearer tokens over basic auth when possible

4. **Configure retries**: Use exponential backoff for resilience

5. **Set dashboard URLs**: Provide links to your monitoring dashboards for easy navigation

6. **Use link templates**: Create direct links to specific alerts in your monitoring tools

7. **Test connectivity**: Verify each source is accessible before relying on it
