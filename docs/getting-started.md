# Getting started

## 1. Write a configuration

Create `conf/config.yaml` with your sources:

```yaml alertview-config
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager.example.com:9093"
```

`type` is `alertmanager`, `grafana` or `zabbix`. Grafana and Zabbix need a
`bearer_token`; see [Configuration](configuration.md#sources).

## 2. Start AlertView

```bash
docker run -d -p 8080:8080 -v $(pwd)/conf:/config:ro ghcr.io/frakev/alertview:latest
```

or, with the binary from the [releases](https://github.com/frakev/alertview/releases):

```bash
./alertview-linux-amd64 conf/config.yaml
```

The log lists the sources it found, or says what is wrong in the file.

## 3. Open the dashboard

Go to <http://localhost:8080>. Each source has a dot next to its name: green
when it answers, red with the error when it does not.

For a wall screen, open `http://localhost:8080/?tv=1`.

Before sharing it, put AlertView behind a proxy with authentication: it has
none of its own ([Deployment](deployment.md#reverse-proxy)).

## How it works

```
Alertmanager ─┐
Grafana ──────┼──►  AlertView  ──►  browsers
Zabbix ───────┘     polls every      refresh every
                    30 s             30 s, plus a live
                                     stream of new alerts
```

- AlertView **polls the sources itself**, every `refresh_interval` seconds (or
  `cache_ttl_seconds`), in parallel, with retries. Browsers are served from the
  result: the load on your sources does not depend on the number of screens.
- Alerts from every source are put in one list, sorted by severity then age.
- New alerts are pushed to open pages (sound and desktop notification if
  enabled).
- The configuration file is watched: edits apply without a restart.
- Nothing is stored: no database, nothing written to disk.

Next: [Configuration](configuration.md) · [Display](display.md) ·
[Deployment](deployment.md)
