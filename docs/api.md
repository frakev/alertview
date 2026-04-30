# API Documentation

AlertView provides a RESTful API for fetching alerts and configuration.

## Base URL

```
http://localhost:8080
```

Or whatever port you've configured AlertView to use.

## Endpoints

### GET /api/alerts

Fetch all alerts from all configured sources.

**Request:**

```
GET /api/alerts HTTP/1.1
Host: localhost:8080
Accept: application/json
```

**Query Parameters:**

| Parameter | Type | Description | Default |
|-----------|------|-------------|---------|
| `source` | string | Filter by source name | All sources |
| `severity` | string | Filter by severity (critical, warning, info) | All severities |
| `state` | string | Filter by state (firing, resolved, silenced, paused) | All states |
| `limit` | integer | Maximum number of alerts to return | 1000 |
| `offset` | integer | Offset for pagination | 0 |

**Response:**

```json
{
  "alerts": [
    {
      "id": "abc123",
      "labels": {
        "alertname": "HighCPUUsage",
        "severity": "critical",
        "instance": "server-01",
        "service": "web"
      },
      "annotations": {
        "summary": "High CPU usage on server-01",
        "description": "CPU usage has been above 90% for 5 minutes"
      },
      "starts_at": "2024-01-15T10:30:00Z",
      "ends_at": null,
      "state": "firing",
      "severity": "critical",
      "source": "alertmanager",
      "generator_url": "http://prometheus.example.com/graph?g0.expr=..."
    },
    {
      "id": "def456",
      "labels": {
        "alertname": "DiskSpaceLow",
        "severity": "warning",
        "instance": "server-02",
        "service": "database"
      },
      "annotations": {
        "summary": "Low disk space on server-02",
        "description": "Disk usage is at 85%"
      },
      "starts_at": "2024-01-15T09:15:00Z",
      "ends_at": null,
      "state": "firing",
      "severity": "warning",
      "source": "alertmanager",
      "generator_url": "http://prometheus.example.com/graph?g0.expr=..."
    }
  ],
  "total": 2,
  "config": {
    "refresh_interval": 30,
    "theme": "dark",
    "timezone": "UTC",
    "play_sounds": true
  },
  "sources": [
    {
      "name": "alertmanager",
      "kind": "alertmanager",
      "url": "http://localhost:9093",
      "status": "healthy",
      "last_fetch": "2024-01-15T10:35:00Z",
      "alert_count": 2
    }
  ]
}
```

**Response Fields:**

- `alerts`: Array of alert objects
- `total`: Total number of alerts (before pagination)
- `config`: Current display configuration
- `sources`: Information about configured sources

**Alert Object Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique alert identifier |
| `labels` | object | Alert labels (key-value pairs) |
| `annotations` | object | Alert annotations (key-value pairs) |
| `starts_at` | string | When the alert started (ISO 8601) |
| `ends_at` | string | When the alert ended (ISO 8601), null if still firing |
| `state` | string | Alert state: firing, resolved, silenced, paused |
| `severity` | string | Alert severity: critical, warning, info, unknown |
| `source` | string | Source name from configuration |
| `generator_url` | string | URL to the alert generator (e.g., Prometheus query) |

**Status Codes:**

- `200 OK`: Success
- `500 Internal Server Error`: Error fetching alerts

### GET /api/alerts/{source}

Fetch alerts from a specific source.

**Request:**

```
GET /api/alerts/alertmanager HTTP/1.1
Host: localhost:8080
Accept: application/json
```

**Response:**

Same format as GET /api/alerts, but only alerts from the specified source.

**Status Codes:**

- `200 OK`: Success
- `404 Not Found`: Source not found
- `500 Internal Server Error`: Error fetching alerts

### GET /api/sources

Get information about all configured sources.

**Request:**

```
GET /api/sources HTTP/1.1
Host: localhost:8080
Accept: application/json
```

**Response:**

```json
{
  "sources": [
    {
      "name": "alertmanager",
      "kind": "alertmanager",
      "url": "http://localhost:9093",
      "status": "healthy",
      "last_fetch": "2024-01-15T10:35:00Z",
      "last_success": "2024-01-15T10:35:00Z",
      "last_error": null,
      "alert_count": 2,
      "config": {
        "timeout": 15,
        "cache_ttl": 60,
        "retry_policy": {
          "max_retries": 3,
          "initial_delay_ms": 1000,
          "max_delay_ms": 10000
        }
      }
    },
    {
      "name": "grafana",
      "kind": "grafana",
      "url": "https://grafana.example.com",
      "status": "healthy",
      "last_fetch": "2024-01-15T10:35:00Z",
      "last_success": "2024-01-15T10:35:00Z",
      "last_error": null,
      "alert_count": 5,
      "config": {
        "timeout": 30,
        "cache_ttl": 60,
        "folder_id": 123
      }
    }
  ]
}
```

**Source Object Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Source name |
| `kind` | string | Source kind (alertmanager, grafana, zabbix) |
| `url` | string | Source URL |
| `status` | string | Source status (healthy, unhealthy, unknown) |
| `last_fetch` | string | When the source was last fetched (ISO 8601) |
| `last_success` | string | When the source was last successfully fetched (ISO 8601) |
| `last_error` | string | Last error message, if any |
| `alert_count` | integer | Number of alerts from this source |
| `config` | object | Source configuration |

**Status Codes:**

- `200 OK`: Success

### GET /api/sources/{name}

Get information about a specific source.

**Request:**

```
GET /api/sources/alertmanager HTTP/1.1
Host: localhost:8080
Accept: application/json
```

**Response:**

Same format as GET /api/sources, but only the specified source.

**Status Codes:**

- `200 OK`: Success
- `404 Not Found`: Source not found

### GET /api/config

Get the current configuration.

**Request:**

```
GET /api/config HTTP/1.1
Host: localhost:8080
Accept: application/json
```

**Response:**

```json
{
  "sources": [
    {
      "name": "alertmanager",
      "kind": "alertmanager",
      "url": "http://localhost:9093",
      "timeout": 15,
      "cache_ttl": 60,
      "retry_policy": {
        "max_retries": 3,
        "initial_delay_ms": 1000,
        "max_delay_ms": 10000
      },
      "link_template": "https://grafana.example.com/d/{{.Labels.dashboard}}?var-alert={{.Labels.alertname}}"
    }
  ],
  "display": {
    "refresh_interval": 30,
    "theme": "dark",
    "timezone": "UTC",
    "play_sounds": true,
    "filters": {
      "severity": ["critical", "warning"],
      "state": ["firing"]
    },
    "sort": {
      "by": "starts_at",
      "order": "desc"
    },
    "group_by": ["alertname"],
    "columns": {
      "severity": true,
      "state": true,
      "starts_at": true,
      "source": true
    }
  },
  "cache_ttl": 60,
  "log_format": "json",
  "port": 8080
}
```

**Status Codes:**

- `200 OK`: Success

### GET /health

Health check endpoint.

**Request:**

```
GET /health HTTP/1.1
Host: localhost:8080
```

**Response:**

```
OK
```

Or with details:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime": "2024-01-15T10:35:00Z",
  "sources": [
    {
      "name": "alertmanager",
      "status": "healthy"
    },
    {
      "name": "grafana",
      "status": "healthy"
    }
  ]
}
```

**Status Codes:**

- `200 OK`: Healthy
- `503 Service Unavailable`: Unhealthy

### GET /

Serve the main HTML page.

**Request:**

```
GET / HTTP/1.1
Host: localhost:8080
Accept: text/html
```

**Response:**

HTML page with the AlertView UI.

**Status Codes:**

- `200 OK`: Success

## WebSocket API (Future Feature)

### /ws

WebSocket endpoint for real-time updates.

**Connection:**

```javascript
const socket = new WebSocket('ws://localhost:8080/ws');

socket.onopen = function(e) {
  console.log('Connected');
};

socket.onmessage = function(event) {
  const data = JSON.parse(event.data);
  console.log('Update:', data);
};

socket.onclose = function(event) {
  console.log('Disconnected');
};
```

**Messages:**

- **Alert Update**: Sent when alerts change
  ```json
  {
    "type": "alerts",
    "alerts": [...],
    "total": 10
  }
  ```

- **Config Update**: Sent when configuration changes
  ```json
  {
    "type": "config",
    "config": {...}
  }
  ```

- **Source Status Update**: Sent when source status changes
  ```json
  {
    "type": "source_status",
    "source": {
      "name": "alertmanager",
      "status": "healthy"
    }
  }
  ```

## Response Format

All JSON responses follow a consistent format:

```json
{
  "data": {...},      // Response data
  "error": null,      // Error message (if any)
  "status": "success" // Status: success, error
}
```

For error responses:

```json
{
  "data": null,
  "error": "Source not found",
  "status": "error"
}
```

## Error Handling

AlertView returns appropriate HTTP status codes:

| Status Code | Description |
|-------------|-------------|
| 200 | Success |
| 400 | Bad Request |
| 404 | Not Found |
| 500 | Internal Server Error |
| 502 | Bad Gateway (source error) |
| 503 | Service Unavailable |
| 504 | Gateway Timeout |

## Rate Limiting (Future Feature)

Rate limiting may be added in the future:

- Default: 60 requests per minute
- Configurable via `api.rate_limit` in configuration

## Authentication (Future Feature)

Authentication may be added in the future:

- Basic authentication
- Bearer token authentication
- API key authentication

## CORS

CORS is enabled by default for the API:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization
```

This can be configured in the `api.cors` section of the configuration.

## Examples

### Fetch All Alerts

```bash
curl http://localhost:8080/api/alerts
```

### Fetch Alerts from Specific Source

```bash
curl http://localhost:8080/api/alerts/alertmanager
```

### Filter Alerts

```bash
# By severity
curl "http://localhost:8080/api/alerts?severity=critical"

# By state
curl "http://localhost:8080/api/alerts?state=firing"

# By source
curl "http://localhost:8080/api/alerts?source=alertmanager"

# Combined filters
curl "http://localhost:8080/api/alerts?severity=critical&state=firing"
```

### Pagination

```bash
# First page
curl "http://localhost:8080/api/alerts?limit=100&offset=0"

# Second page
curl "http://localhost:8080/api/alerts?limit=100&offset=100"
```

### Get Source Information

```bash
# All sources
curl http://localhost:8080/api/sources

# Specific source
curl http://localhost:8080/api/sources/alertmanager
```

### Get Configuration

```bash
curl http://localhost:8080/api/config
```

### Health Check

```bash
curl http://localhost:8080/health

# With details
curl -H "Accept: application/json" http://localhost:8080/health
```

## API Versioning

The current API version is v1. Future versions may be added:

- `/api/v1/alerts` - Current version
- `/api/v2/alerts` - Future version (when released)

## OpenAPI Specification (Future Feature)

An OpenAPI/Swagger specification may be added in the future for:

- Automatic API documentation
- Client code generation
- API testing
- Interactive API exploration

## Additional Resources

- [Configuration Reference](configuration/config-file.md)
- [Examples](examples/README.md)
- [Troubleshooting](troubleshooting.md)
