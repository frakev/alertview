# Configuration File Reference

This document provides a complete reference for the AlertView configuration file.

## Complete Configuration Schema

```yaml
# ========== Server Configuration ==========
port: 8080                      # Port to listen on (default: 8080)
refresh_interval: 30           # Seconds between auto-refreshes (default: 30)
tls_insecure: false            # Skip TLS certificate verification (default: false)
cache_ttl_seconds: 0          # Cache TTL in seconds (0 = disabled, default: 0)
log_format: "text"             # Log format: "text" or "json" (default: "text")

# ========== Alert Sources ==========
sources:
  - name: "Alertmanager"        # Required: Unique name for this source
    type: alertmanager          # Required: Source type (alertmanager, grafana, zabbix)
    url: "http://localhost:9093" # Required: Base URL for the source
    
    # === Optional Settings ===
    dashboard_url: "https://grafana.example.com/alerting/list"  # Fallback link for the ↗ button
    link_template: "https://example.com/alerts?query={{.Labels.alertname}}"  # Template for the ↗ button
    alert_link_template: "https://wiki.example.com/runbook/{{.Labels.alertname}}"  # Makes the whole alert clickable
    source_link: true           # Show the ↗ button for this source
    severity_label: "severity"  # Label carrying the severity (case-insensitive)
    timeout: 15                 # Request timeout in seconds (default: 15)
    
    # === Authentication (choose one) ===
    basic_auth:                 # HTTP Basic Authentication
      username: "user"
      password: "pass"
    
    bearer_token: "your-token-here"  # Bearer token authentication
    
    # === Retry Policy ===
    retry_policy:
      max_retries: 3           # Maximum number of retry attempts (default: 3)
      initial_delay_ms: 1000  # Initial delay between retries in ms (default: 1000)
      max_delay_ms: 30000     # Maximum delay between retries in ms (default: 30000)

  - name: "Grafana"
    type: grafana
    url: "http://grafana:3000"
    bearer_token: "glsa_xxxxx"  # Grafana service account token
    timeout: 20
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 60000

  - name: "Zabbix"
    type: zabbix
    url: "https://zabbix.example.com/zabbix"
    bearer_token: "zabbix-api-token"
    dashboard_url: "https://zabbix.example.com/zabbix/zabbix.php?action=problem.view"

# ========== Display Configuration ==========
display:
  # Labels to display on each alert card
  labels:
    - namespace
    - job
    - instance
    - cluster
    - node
    - pod
    - host
    - hostgroup
    - severity  # Note: severity is always shown as a badge
    - alertname # Note: alertname is always shown as a badge
  
  # Theme settings
  theme: "auto"              # "auto" (follow the OS), "dark" or "light" (default: "auto")
  custom_css: ""             # URL of an extra stylesheet layered on the theme

  # Timezone settings
  timezone: "local"          # "local", "UTC", or IANA timezone (default: "local")

  # Sound notifications
  play_sounds: false         # Enable sound notifications for new alerts (default: false)

  # Alert grouping
  group_by: []              # Labels to group alerts by (e.g., ["namespace", "job"]) (default: [])

  # Severity ranking, most severe first. Drives sorting, filter chips and sounds
  severity_order: ["critical", "error", "high", "warning", "info", "none"]

  # Labels shown in front of the alert name, in both normal and TV mode
  prefix_labels: ["hostname"]
  prefix_separator: " / "

  # Alert body
  show_alert_name: true      # false = show the summary annotation instead
  show_labels: true          # false = hide the label chips
  critical_icon: "🔥"         # marker on critical alerts, "" to disable

  # Links
  alert_link_template: ""    # makes the whole alert clickable, built from its labels
  source_link: true          # show the ↗ "open in the source" button
  link_new_tab: true         # false = open links in the same tab (kiosk)

  # TV mode
  tv_mode_default: false     # start in TV mode (a stored choice or ?tv= wins)
```

## Configuration Sections

### Server Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `port` | u16 | 8080 | Port number to listen on |
| `refresh_interval` | u64 | 30 | Seconds between auto-refreshes |
| `tls_insecure` | bool | false | Skip TLS certificate verification |
| `cache_ttl_seconds` | u64 | 0 | Cache TTL in seconds (0 = disabled). One entry per source |
| `log_format` | string | "text" | Log format: "text" or "json" |
| `config_watch_method` | string | "polling" | Method to watch config file: "inotify" (native) or "polling" (default, works everywhere) |
| `config_poll_interval` | u64 | 10 | Polling interval in seconds (only used with polling method) |

### Sources Configuration

Each source must have:
- `name`: Unique identifier for the source
- `type`: One of: `alertmanager`, `grafana`, `zabbix`
- `url`: Base URL for the source API

#### Source Types

**Alertmanager:**
```yaml
- name: "Alertmanager"
  type: alertmanager
  url: "http://alertmanager:9093"
```
- API endpoint: `{url}/api/v2/alerts`
- Supports: basic_auth, bearer_token

**Grafana:**
```yaml
- name: "Grafana"
  type: grafana
  url: "http://grafana:3000"
```
- API endpoint: `{url}/api/alertmanager/grafana/api/v2/alerts`
- Requires: bearer_token (Service Account token recommended)
- Also supports: basic_auth

**Zabbix:**
```yaml
- name: "Zabbix"
  type: zabbix
  url: "https://zabbix.example.com/zabbix"
```
- API endpoint: `{url}/api_jsonrpc.php`
- Requires: bearer_token (Zabbix API token)

#### Source Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `dashboard_url` | string | null | URL to link to from alert cards |
| `severity_label` | string | "severity" | Label carrying the severity, matched case-insensitively (Alertmanager/Grafana) |
| `link_template` | string | null | Template for the ↗ source link |
| `alert_link_template` | string | null | Template making the whole alert clickable (overrides `display.alert_link_template`) |
| `source_link` | bool | null | Show the ↗ source link for this source (overrides `display.source_link`) |
| `timeout` | u64 | 15 | Request timeout in seconds |
| `basic_auth` | object | null | HTTP Basic Authentication |
| `bearer_token` | string | null | Bearer token authentication |
| `retry_policy` | object | defaults | Retry configuration |

#### Retry Policy

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_retries` | usize | 3 | Maximum retry attempts |
| `initial_delay_ms` | u64 | 1000 | Initial delay in milliseconds |
| `max_delay_ms` | u64 | 30000 | Cap on the delay between attempts |

A failed attempt is retried after `initial_delay_ms × 2^(attempt-1)`, capped at
`max_delay_ms`. HTTP 4xx responses are **not** retried — a 404, 401 or 403 is a
configuration problem, and retrying it only delays the error — except `408` and
`429`, which mean "try again later". 5xx and network errors are always retried.

The retry delay follows an exponential backoff pattern:
- Attempt 1: Immediate
- Attempt 2: After `initial_delay_ms`
- Attempt 3: After `2 * initial_delay_ms`
- Attempt 4: After `4 * initial_delay_ms`
- ... up to `max_delay_ms`

### Display Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `labels` | array | see below | Labels shown as chips on alert cards |
| `theme` | string | "auto" | "auto" (follow the OS), "dark" or "light". A URL is still accepted and treated as `custom_css` |
| `custom_css` | string | null | URL of an extra stylesheet layered on top of the theme |
| `timezone` | string | "local" | Timezone for date display |
| `play_sounds` | bool | false | Enable sound notifications |
| `group_by` | array | [] | Labels to group alerts by (e.g., ["namespace", "job"]) |
| `severity_order` | array | critical, error, high, warning, info, none | Severity ranking, most severe first. Unlisted severities sort last |
| `prefix_labels` | array | ["hostname"] | Labels shown in front of the alert name, in both modes |
| `prefix_separator` | string | " / " | Separator between prefix labels |
| `show_alert_name` | bool | true | false shows the `summary` annotation instead of the alert name |
| `show_labels` | bool | true | false hides the label chips |
| `critical_icon` | string | "🔥" | Marker on critical alerts, `""` to disable |
| `alert_link_template` | string | null | Makes the whole alert clickable (see Link Templates) |
| `source_link` | bool | true | Show the ↗ "open in the source" button |
| `link_new_tab` | bool | true | false opens links in the same tab |
| `tv_mode_default` | bool | false | Start in TV mode when the browser has no stored choice |

Every one of these is documented in more detail, with examples, in
[Display Options](display-options.md).

#### Default Labels

If not specified, these labels are shown:
- namespace
- job
- instance
- cluster
- node

## Link Templates

Link templates allow you to create custom URLs for your alerts using placeholders.

### Available Placeholders

**Labels:**
- `{{.Labels.<key>}}` - Any label value (e.g., `{{.Labels.namespace}}`)

**Annotations:**
- `{{.Annotations.<key>}}` - Any annotation value (e.g., `{{.Annotations.summary}}`)

**Alert Fields:**
- `{{.Fingerprint}}` - Unique alert fingerprint
- `{{.Source}}` - Source name
- `{{.SourceType}}` - Source type (alertmanager, grafana, zabbix)
- `{{.Status}}` - Alert status (firing, silenced, pending)
- `{{.Severity}}` - Alert severity
- `{{.Name}}` - Alert name
- `{{.StartsAt}}` - Alert start time (RFC3339)
- `{{.EndsAt}}` - Alert end time (RFC3339, if resolved)

### Examples

**Basic:**
```yaml
link_template: "https://grafana.example.com/alerts?query={{.Labels.alertname}}"
```

**With namespace:**
```yaml
link_template: "https://grafana.example.com/d/abc123?var-namespace={{.Labels.namespace}}&var-alert={{.Labels.alertname}}"
```

**Grafana dashboard link:**
```yaml
link_template: "https://grafana.example.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}&var-alertname={{.Name}}"
```

**Zabbix with event ID:**
```yaml
link_template: "https://zabbix.example.com/zabbix.php?action=problem.view&triggerids[]={{.Labels.triggerid}}"
```

### Two independent destinations

An alert can carry two links, and either can be left out:

| | Declared by | Rendered as |
|---|---|---|
| Alert link | `alert_link_template` (source, then `display`) | the whole card or row is clickable |
| Source link | `link_template`, then the alert's generator URL, then `dashboard_url` | the ↗ button on the right |

For Zabbix, the source link falls back to a `problem.view` URL built from the
trigger id when nothing else applies. `display.source_link: false` hides the ↗
button; a source can opt back in with `source_link: true`.

### Rules

- Substituted values are **percent-encoded**, so a label containing a space,
  `&` or `/` cannot change the shape of the URL. A value therefore cannot be
  used as a path separator.
- If the alert does not carry a label the template asks for, the template is
  **not used**: the source link falls back to the next candidate, and the alert
  link is dropped. No URL is ever emitted with `{{.Labels.foo}}` left in it.
- Only `http` and `https` links are rendered. Anything else — including a
  `javascript:` generator URL coming from a source — is ignored.

## Complete Examples

### Minimal Configuration

```yaml
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://localhost:9093"
```

### Production Configuration

```yaml
# Server settings
port: 8080
refresh_interval: 60
cache_ttl_seconds: 30
log_format: "json"
tls_insecure: false

# Alert sources
sources:
  - name: "Production Alertmanager"
    type: alertmanager
    url: "https://alertmanager.prod.svc.cluster.local:9093"
    timeout: 30
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 30000
    dashboard_url: "https://grafana.prod.example.com/alerting/list"

  - name: "Staging Grafana"
    type: grafana
    url: "https://grafana.staging.example.com"
    bearer_token: "${GRAFANA_TOKEN}"  # Set via environment variable
    timeout: 20
    link_template: "https://grafana.staging.example.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}"

  - name: "Zabbix"
    type: zabbix
    url: "https://zabbix.example.com/zabbix"
    bearer_token: "${ZABBIX_TOKEN}"
    timeout: 45

# Display settings
display:
  labels:
    - namespace
    - job
    - instance
    - host
    - hostgroup
  theme: "dark"
  timezone: "Europe/Paris"
  play_sounds: true
```

### Development Configuration

```yaml
port: 3000
refresh_interval: 10
log_format: "json"

sources:
  - name: "Local Alertmanager"
    type: alertmanager
    url: "http://localhost:9093"
    tls_insecure: true  # For local development with self-signed certs

display:
  labels:
    - namespace
    - job
    - instance
  theme: "light"
  play_sounds: false
```

## Validation

You can validate your configuration file using:

```bash
# Using yamllint (recommended)
yamllint config.yaml

# Using Python
python3 -c 'import yaml, sys; yaml.safe_load(open(sys.argv[1]))' config.yaml

# Using Rust (if you have serde_yaml)
cargo run --example validate-config -- config.yaml
```

## Migration Guide

### From v0.1.0 to v0.2.0

New fields added (all optional with sensible defaults):
- `cache_ttl_seconds` - Set to 0 to disable caching
- `log_format` - Set to "text" or "json"
- `display.theme` - Set to "dark" or "light"
- `display.timezone` - Set to "local" or your timezone
- `display.play_sounds` - Set to false by default
- `source.timeout` - Defaults to 15 seconds
- `source.retry_policy` - Defaults to 3 retries with exponential backoff
- `source.link_template` - Optional custom link template

Existing configurations will continue to work without changes.
