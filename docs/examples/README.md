# Configuration Examples

Three configurations that are known to work, from the smallest useful one to a
wall display. Every key below is real — each block is parsed by the test suite,
so an option that is renamed or removed breaks CI rather than your dashboard.

The annotated reference is [`config.example`](../../config.example) at the root
of the repository: it lists **every** option with its default. The schema and
the validation rules are in [Configuration File](../configuration/config-file.md).

> AlertView does **not** expand `${VARIABLES}` inside the YAML. A token must be
> written literally. On Kubernetes, put the whole file in a Secret — see
> [Secrets Management](../deployment/kubernetes.md#secrets-management).

## Minimal

One source, everything else on its defaults.

```yaml alertview-config
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager.example.com:9093"
```

`url` is the plain service URL — the API path is appended for you. That is the
whole file: the port defaults to 8080, the refresh to 30 seconds.

## Two sources, with links out

```yaml alertview-config
port: 8080
refresh_interval: 30

sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager.example.com:9093"
    dashboard_url: "https://grafana.example.com/alerting/list"

  - name: "Zabbix"
    type: zabbix
    url: "https://zabbix.example.com/zabbix"
    bearer_token: "REPLACE_ME"
    timeout: 30

display:
  labels: [namespace, job, host, hostgroup]
  prefix_labels: [hostname]
  alert_link_template: "https://wiki.example.com/runbook/{{.Labels.alertname}}"
```

`alert_link_template` turns the severity marker into a link to your runbook. It
accepts `{{.Labels.x}}`, `{{.Annotations.x}}` and a handful of scalars —
[the full list](../configuration/config-file.md#link-templates). An alert
missing a label the template asks for is simply not clickable; **a placeholder
that does not exist drops the link entirely**, so check the names.

`alertname` works for Zabbix too: the problem name is exposed under that key so
one template covers every source type.

## Wall display

Grouped by team, dense rows, and a cache so twenty browsers do not multiply the
load on the sources.

```yaml alertview-config
refresh_interval: 20
cache_ttl_seconds: 15

sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager.example.com:9093"

display:
  tv_mode_default: true
  group_by: [team]
  prefix_labels: [hostname]
  play_sounds: true
  link_new_tab: false
  timezone: "Europe/Paris"
```

Open it with `?tv=1` rather than relying on `tv_mode_default`: the URL wins over
the stored preference, which is what you want on a screen nobody logs into.
`link_new_tab: false` keeps navigation in the same tab on a kiosk.

For one screen **per** team, keep a single instance and filter in the URL:
`?q=team=sre`. See [Display Options](../configuration/display-options.md).

## Checking a file

```bash
alertview /path/to/config.yaml
```

It validates on startup, names any unknown key, prints the sources it found,
then serves. Ctrl-C once you have seen the log.
