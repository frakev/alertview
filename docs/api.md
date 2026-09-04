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

**Response:**

```json
{
  "alerts": [
    {
      "fingerprint": "alertmanager:abc123",
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
      "status": "firing",
      "severity": "critical",
      "name": "HighCPUUsage",
      "source": "alertmanager",
      "source_type": "alertmanager",
      "link_url": "http://prometheus.example.com/graph?g0.expr=..."
    }
  ],
  "sources": [
    {
      "name": "alertmanager",
      "status": "ok",
      "alert_count": 1,
      "error": null
    }
  ],
  "refresh_interval": 30,
  "display_labels": ["namespace", "job", "instance"],
  "timezone": "local",
  "theme": null,
  "play_sounds": false,
  "groups": [],
  "group_by": []
}
```

**Response Fields:**

- `alerts`: Array of alert objects
- `sources`: Array of source status objects
- `refresh_interval`: Seconds between auto-refreshes
- `display_labels`: Labels to display on alert cards
- `timezone`: Current timezone setting
- `theme`: Current theme (dark/light/custom URL)
- `play_sounds`: Whether sound notifications are enabled
- `groups`: Array of alert groups (if grouping is enabled)
- `group_by`: Labels used for grouping

**Alert Object Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `fingerprint` | string | Unique alert identifier |
| `labels` | object | Alert labels (key-value pairs) |
| `annotations` | object | Alert annotations (key-value pairs) |
| `starts_at` | string | When the alert started (ISO 8601) |
| `ends_at` | string | When the alert ended (ISO 8601), null if still firing |
| `status` | string | Alert status: firing, silenced, pending |
| `severity` | string | Alert severity: critical, error, high, warning, info, none (or any custom level, see `display.severity_order`) |
| `name` | string | Alert name (from alertname label) |
| `source` | string | Source name from configuration |
| `source_type` | string | Source type: alertmanager, grafana, zabbix |
| `link_url` | string | URL to view the alert in its source system |

**Source Status Object Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Source name |
| `status` | string | Source status: ok, error |
| `alert_count` | integer | Number of alerts from this source |
| `error` | string | Error message if status is error, null otherwise |

**Status Codes:**

- `200 OK`: Success
- `500 Internal Server Error`: Error fetching alerts

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

**Status Codes:**

- `200 OK`: Healthy

### GET /events

Server-Sent Events (SSE) endpoint for real-time alert notifications.

**Request:**

```
GET /events HTTP/1.1
Host: localhost:8080
Accept: text/event-stream
```

**Response:**

Stream of SSE events with the following format:

```
event: new_alert
data: {"fingerprint":"alertmanager:abc123","labels":{"alertname":"HighCPUUsage","severity":"critical"},"annotations":{"summary":"High CPU usage"},"starts_at":"2024-01-15T10:30:00Z","status":"firing","severity":"critical","name":"HighCPUUsage","source":"alertmanager","source_type":"alertmanager","link_url":"http://prometheus.example.com/..."}

```

**Event Types:**

- `new_alert`: A new alert has been detected (not previously seen)
- `config_reloaded`: Configuration file has been reloaded (sent when config changes are detected)

**Notes:**

- The connection remains open and sends events as new alerts arrive
- Automatic reconnection with exponential backoff is handled client-side
- Each event contains a complete alert object in JSON format
- Only alerts that are **new** (not previously seen in cache) trigger events

**Status Codes:**

- `200 OK`: Connection established, event stream begins

## Error Handling

AlertView returns appropriate HTTP status codes:

| Status Code | Description |
|-------------|-------------|
| 200 | Success |
| 500 | Internal Server Error |

## CORS

CORS is not currently implemented. All API endpoints are accessible without CORS headers.

## API Versioning

The current API is unversioned. All endpoints are at the root level (e.g., `/api/alerts`).

## Examples

### Fetch All Alerts

```bash
curl http://localhost:8080/api/alerts
```

### Health Check

```bash
curl http://localhost:8080/health
```

### Stream Real-time Events

```bash
# Using curl (will hang and print events as they arrive)
curl -N http://localhost:8080/events

# Using curl with timeout
curl -N --max-time 10 http://localhost:8080/events
```

### JavaScript Example

```javascript
// Connect to SSE endpoint
const eventSource = new EventSource('http://localhost:8080/events');

eventSource.onopen = () => {
  console.log('Connection to server opened');
};

eventSource.onerror = () => {
  console.log('EventSource failed.');
};

eventSource.addEventListener('new_alert', (event) => {
  const alert = JSON.parse(event.data);
  console.log('New alert:', alert);
});
```

## Additional Resources

- [Configuration Reference](configuration/config-file.md)
- [Examples](examples/README.md)
- [Troubleshooting](troubleshooting.md)
