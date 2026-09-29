# Configuration

AlertView reads one YAML file. Only `sources` is required; everything else has a
default. [`config.example`](../config.example) lists every option with its
default value and a comment — copy it and edit it.

Display options (labels, titles, TV mode, themes…) have their own page:
[Display](display.md).

## A working file

```yaml alertview-config
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager.example.com:9093"

  - name: "Zabbix"
    type: zabbix
    url: "https://zabbix.example.com/zabbix"
    bearer_token: "REPLACE_ME"

display:
  labels: [namespace, job, host]
```

`url` is the plain service URL: the API path is appended for you.

## Where the file is read from

In order: `alertview --config <file>`, then `alertview <file>`, then
`$ALERTVIEW_CONFIG`, then `./config.yaml`. The Docker image reads
`/config/config.yaml`.

**Edits apply without a restart**, except `port`, `tls_insecure` and
`log_format`, which are read once at startup. The file is checked every 10
seconds by default (`config_watch_method: polling`); this works everywhere,
including on a Kubernetes ConfigMap or Secret.

## Validation

AlertView checks the file on startup and on every reload.

- **An unknown key stops it from starting**, with the key's path and, when the
  option lives elsewhere, where to put it:

  ```
  config.yaml: 2 key(s) AlertView does not understand:
    • play_sounds — a display option: move it under `display:` (`display.play_sounds`)
    • sources[0].cache_ttl — caching is global: use the top-level `cache_ttl_seconds`
  ```

- It also refuses a `port` or `refresh_interval` of 0, two sources with the same
  name, a source without `url`, and invalid enum values (`log_format`,
  `config_watch_method`).
- **A reload that fails is refused**: the previous configuration keeps running,
  and the dashboard shows an amber banner with the reason.

To check a file, start AlertView on it: it prints the sources it found, or the
error.

## Server options

| Option | Default | Meaning |
|---|---|---|
| `port` | `8080` | HTTP port |
| `refresh_interval` | `30` | Seconds between two refreshes in the browser |
| `cache_ttl_seconds` | `0` | Seconds between two polls of the sources; `0` = `refresh_interval` |
| `tls_insecure` | `false` | Accept invalid TLS certificates from every source |
| `log_format` | `text` | `text` or `json`. The level comes from `RUST_LOG` |
| `config_watch_method` | `polling` | `polling`, or `inotify` (instant, but misses ConfigMap updates) |
| `config_poll_interval` | `10` | Seconds between two checks of the file, with `polling` |

### How the sources are polled

AlertView polls its sources on its own schedule and serves every browser from
the result: one dashboard or fifty make the same number of calls to your
sources. Each source is published as soon as it answers, so a slow one does not
hold back the others. A source that fails is shown in red next to its name; the
rest of the dashboard stays live.

## Sources

```yaml alertview-config
sources:
  - name: "Alertmanager"          # required, unique
    type: alertmanager            # required: alertmanager, grafana or zabbix
    url: "http://alertmanager:9093"   # required
    timeout: 15                   # seconds per attempt
    dashboard_url: "https://grafana.example.com/alerting/list"
    # bearer_token: "…"           # or basic_auth: { username: …, password: … }
    # severity_label: "severity"  # label holding the severity
    # link_template, alert_link_template, source_link: see Links below
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 30000
```

**Retries.** The delay doubles after each failed attempt, from
`initial_delay_ms` up to `max_delay_ms`. HTTP 4xx errors are not retried (a 401
or a 404 is a configuration problem), except 408 and 429.

**Credentials** are written in the file as they are. There is **no `${VAR}`
expansion**: `bearer_token: "${TOKEN}"` sends those characters literally. Keep
the file out of git, and on Kubernetes put it in a Secret (see
[Deployment](deployment.md#credentials)).

### Alertmanager

- Reads `{url}/api/v2/alerts`. Basic auth or bearer token.
- Severity comes from the `severity` label (or `severity_label`).
- A silenced alert shows who silenced it and the silence's comment.

### Grafana

- Reads Grafana's built-in Alertmanager:
  `{url}/api/alertmanager/grafana/api/v2/alerts`.
- Use a **service account token** as `bearer_token` (basic auth also works).
- Grafana adds `dashboardUid` and `panelId` annotations, handy in links:
  `https://grafana.example.com/d/{{.Annotations.dashboardUid}}?viewPanel={{.Annotations.panelId}}`.

### Zabbix

- Reads problems through `{url}/api_jsonrpc.php`, with an **API token** as
  `bearer_token`. Zabbix 6.0 to 7.x, detected automatically.
- Severity mapping: Disaster → `critical`, High → `high`, Average and
  Warning → `warning`, Information → `info`, Not classified → `none`.
- An acknowledged or suppressed problem is shown as silenced, with the
  acknowledgement message and its author. Problems of disabled triggers are
  hidden, as in Zabbix's own Problems view.
- The problem name is exposed as `alertname`, and `eventid` / `triggerid` as
  labels, so one link template can serve every source.
- The Zabbix API can be slow: a `timeout` of 30 seconds is reasonable.

## Links

Each alert can carry two links, independently:

| Link | Configured by | Shown as |
|---|---|---|
| **Alert link** | `alert_link_template` (per source, or under `display:`) | the severity dot becomes clickable |
| **Source link** | `link_template`, else the alert's own URL, else `dashboard_url` | the ↗ button |

`source_link: false` hides the ↗ button (under `display:`, or per source).

Templates accept these placeholders: `{{.Labels.x}}`, `{{.Annotations.x}}`,
`{{.Name}}`, `{{.Severity}}`, `{{.Status}}`, `{{.Source}}`, `{{.SourceType}}`,
`{{.Fingerprint}}`, `{{.StartsAt}}`, `{{.EndsAt}}`.

```yaml
display:
  alert_link_template: "https://wiki.example.com/runbook/{{.Labels.alertname}}"
```

- Values are percent-encoded, so a label cannot break the URL.
- If an alert lacks a label the template uses, the link is not built: the
  alert is not clickable, and the ↗ button falls back to the next choice.
- Only `http` and `https` links are ever shown.

## Environment variables

They provide **defaults**: a value written in the file wins.

| Variable | Option |
|---|---|
| `ALERTVIEW_CONFIG` | path of the file |
| `ALERTVIEW_PORT` | `port` |
| `ALERTVIEW_REFRESH_INTERVAL` | `refresh_interval` |
| `ALERTVIEW_CACHE_TTL` | `cache_ttl_seconds` |
| `ALERTVIEW_LOG_FORMAT` | `log_format` |
| `ALERTVIEW_CONFIG_WATCH_METHOD` | `config_watch_method` |
| `ALERTVIEW_CONFIG_POLL_INTERVAL` | `config_poll_interval` |
| `RUST_LOG` | log level: `error`, `warn`, `info`, `debug`, `trace` |

Sources and display options can only be set in the file.
