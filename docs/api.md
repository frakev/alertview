# API

Three endpoints, all `GET`, without authentication (put a proxy in front, see
[Deployment](deployment.md#reverse-proxy)).

| Endpoint | Returns |
|---|---|
| `/api/alerts` | every alert, the state of each source, and the display settings |
| `/events` | a stream of server-sent events |
| `/health` | `OK` when the server runs |

The rest (`/`, `/app.js`, `/style.css`, `/theme.js`, `/sw.js`,
`/manifest.webmanifest`, `/icons/…`) is the web page itself.

## `GET /api/alerts`

Served from the last poll of the sources: calling it never makes AlertView
query a source. It always answers `200`; a failing source is reported in
`sources`.

```bash
curl -s http://localhost:8080/api/alerts | jq '.sources'
```

```json
{
  "alerts": [
    {
      "fingerprint": "Alertmanager:abc123",
      "source": "Alertmanager",
      "source_type": "alertmanager",
      "status": "firing",
      "severity": "critical",
      "name": "HighCPUUsage",
      "labels": { "alertname": "HighCPUUsage", "instance": "server-01" },
      "annotations": { "summary": "High CPU usage on server-01" },
      "starts_at": "2026-09-29T10:30:00Z",
      "ends_at": null,
      "link_url": "http://prometheus.example.com/graph?g0.expr=...",
      "alert_link_url": null
    }
  ],
  "sources": [
    { "name": "Alertmanager", "status": "ok", "alert_count": 1, "error": null }
  ],
  "groups": [],
  "refresh_interval": 30
}
```

**Alert**

| Field | Meaning |
|---|---|
| `fingerprint` | unique identifier |
| `source`, `source_type` | source name, and `alertmanager`, `grafana` or `zabbix` |
| `status` | `firing`, `silenced` or `pending` |
| `severity` | normalised severity (`critical`, `error`, `high`, `warning`, `info`, `none`, or a custom level) |
| `name` | the alert name (`alertname`) |
| `labels`, `annotations` | as received from the source |
| `starts_at`, `ends_at` | RFC 3339 timestamps; `ends_at` is `null` while active |
| `link_url` | the ↗ link, or `null` |
| `alert_link_url` | the link from `alert_link_template`, or `null` |

Links are always `http` or `https`.

**Source**

| Field | Meaning |
|---|---|
| `name` | source name |
| `status` | `ok`, `error`, or `pending` (not answered yet since startup) |
| `alert_count` | number of alerts |
| `error` | the error message when `status` is `error`; passwords in URLs are masked |

**Group** (only with `display.group_by`): `key`, `labels` (the `group_by`
values), `count` and `severity_counts`. The alerts themselves are in `alerts`.

The other fields of the response (`display_labels`, `timezone`, `theme`,
`severity_order`, `title_annotations`…) repeat the `display` settings for the
page; see [Display](display.md).

## `GET /events`

A [server-sent events](https://developer.mozilla.org/docs/Web/API/Server-sent_events)
stream:

| Event | Data |
|---|---|
| `new_alert` | an alert (same object as above) that was not there before |
| `config_reloaded` | the configuration file was reloaded |
| `config_error` | a configuration edit was refused; the data is the reason |

```bash
curl -N http://localhost:8080/events
```

At most 100 streams are open at once; beyond that, the endpoint answers `429`.

## `GET /health`

Answers `200 OK` as soon as the server runs, whatever the state of the sources.
