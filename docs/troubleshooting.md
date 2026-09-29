# Troubleshooting

Start with the logs: AlertView prints what it loaded and every source error.

```bash
alertview config.yaml                          # or: docker logs <container>
RUST_LOG=debug alertview config.yaml           # more detail
curl -s http://localhost:8080/api/alerts | jq '.sources'   # state of each source
```

## AlertView does not start

**`N key(s) AlertView does not understand`** — a key in the file is unknown.
The message gives its path and, when it lives elsewhere, where to move it. The
usual case is a display option at the top level (`play_sounds: true` next to
`sources:`): it belongs under `display:`. Before 0.13 such keys were silently
ignored, so an old file may fail after an upgrade.

**`Invalid configuration for source at index N`**, **`Duplicate source name`**,
**`… cannot be 0`** — see the rules in
[Configuration](configuration.md#validation).

**`Cannot read "config.yaml"`** — wrong path, or the file is not readable by
the user running AlertView (65532 in the Docker image). On Kubernetes with a
Secret, set `fsGroup: 65532` (see [Deployment](deployment.md#credentials)).

**Address already in use** — another process uses the port; change `port`.

## A source is in error

The source's dot is red and the error is shown next to its name (hover it), and
in `/api/alerts`.

| Error | Likely cause |
|---|---|
| `HTTP 401` / `HTTP 403` | missing or wrong `bearer_token` / `basic_auth`, or a token without read access to alerts |
| `HTTP 404` | wrong `url` or wrong `type`. Give the plain service URL (`http://alertmanager:9093`); AlertView adds the API path |
| connection refused, DNS error | the source is not reachable from where AlertView runs. Test from there: `curl <url>` (or `kubectl exec`) |
| timeout | the source is slow: raise `timeout` (30 for Zabbix is common) |
| certificate error | see below |

A token written as `"${TOKEN}"` is sent as is: AlertView does not expand
variables in the file.

**Certificates.** AlertView trusts the public certificate authorities bundled in
it, not the system's certificate store. A source using a self-signed
certificate or a private CA therefore fails, and needs `tls_insecure: true`
(which disables the check for every source).

## The dashboard is empty

The empty list says why:

- **✅ No active alerts** — every source answered, with nothing.
- **⏳ Waiting for …** — just after startup, these sources have not answered
  yet (a slow source, or one being retried).
- **⚠ … unreachable** — these sources are in error (see above); their alerts
  are not shown.
- **🔍** — your search or filters hide everything. Clear the search box and the
  chips, and check the URL (`?q=`, `?sev=`, `?src=`).

Silenced alerts are hidden by default: click **Show silenced**, or add
`?silenced=1` to the URL.

## The dashboard does not update

- **Red "Backend unreachable" banner** — the browser cannot reach AlertView
  (server down, network, proxy). It retries every 15 seconds and clears by
  itself.
- **Amber "Configuration rejected" banner** — your last edit of the file was
  refused; the reason is in the banner. The previous configuration is still
  running.
- **Edits of the file are ignored** — `port`, `tls_insecure` and `log_format`
  need a restart. With Docker, mount the directory, not the file. With
  `config_watch_method: inotify` on Kubernetes, switch back to `polling`.
- **New alerts arrive late behind a proxy** — the proxy buffers `/events`; see
  [Deployment](deployment.md#reverse-proxy).

## Links

- **An alert is not clickable** — no `alert_link_template` is set, or the alert
  lacks a label the template uses.
- **The ↗ button is missing** — `source_link: false`, or no link could be built
  (no `link_template`, no URL from the source, no `dashboard_url`).

## Sound and notifications

- **No sound** — `display.play_sounds: true` must be set (under `display:`), and
  browsers only play sound after someone has clicked on the page.
- **No desktop notification** — click 🔔 and allow notifications. If they were
  refused once, re-allow them in the browser's site settings.

## Reporting a problem

Open an [issue](https://github.com/frakev/alertview/issues) with the AlertView
version (footer of the page, or `alertview --version`), the log lines around
the problem, and the relevant part of the configuration **without tokens**.
