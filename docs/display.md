# Display

Everything under `display:` in the configuration. Each option is listed with its
default in [`config.example`](../config.example); this page explains what they
do.

## What each alert shows

```yaml alertview-config
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager:9093"

display:
  labels: [namespace, job, instance]   # shown as key=value chips
  prefix_labels: [hostname]            # shown before the title
  prefix_separator: " / "
  show_alert_name: true
  title_annotations: ["summary"]
  show_labels: true
```

- **`labels`** — the labels shown as chips, in this order, when the alert has
  them. Default: `namespace, job, instance, cluster, node`.
- **`prefix_labels`** — shown in front of the title (default: `hostname`), and
  then not repeated as chips.
- **`show_alert_name: false`** replaces the alert name with an annotation:
  the first of **`title_annotations`** the alert has (default `summary`). With
  `title_annotations: description` (a single name or a list), the title is the
  description. An alert with none of them keeps its name.
  `title_annotations` has no effect while `show_alert_name` is `true`.
- **`show_labels: false`** hides the chips.

Nothing hidden is lost: a **`+N`** button on the alert shows the hidden labels
and the alert name.

Under the title, a card shows the summary and the description (each once). In
TV mode, a row shows one sentence: when an annotation is the title, the summary
is not added next to it.

## Severity

```yaml
display:
  severity_order: ["critical", "error", "high", "warning", "info", "none"]
  critical_icon: "flame"
  status_icons:
    silenced: "bell-off"
    inhibited: "bell-off"
    pending: "hourglass"
```

- **`severity_order`** ranks severities, most severe first. It drives the
  sorting, the filter chips, the order of groups and the sound played. A
  severity missing from the list sorts last; add your own levels to place them.
  `crit`, `err`, `warn` and `information` are understood as aliases.
- **`critical_icon`** replaces the coloured dot of critical alerts. `""` gives
  them back the dot.
- **`status_icons`** marks silenced, inhibited and pending alerts. Firing alerts
  have no marker: it is the normal state. A silenced alert has a 💬 button that
  shows the silence comment and its author. `silenced` and `inhibited` share the
  bell by default; give them different icons to tell them apart on sight. A map
  that sets `silenced` but says nothing about `inhibited` — as any map written
  before 0.14 does — gives `inhibited` the same icon rather than none; write
  `inhibited: ""` to ask for no marker.

`flame`, `bell-off` and `hourglass` are built-in drawings that look the same on
every machine. Any other text is shown as is; a typed emoji works only where the
machine has an emoji font.

The severity chips filter the list. Each click adds a severity or takes it out,
so several can be on at once — `critical` + `error` and nothing else. The `all`
chip clears them, and so does taking the last one out. A selected severity that
no alert currently carries stays on the row at zero, so a filter is never active
without a chip to show it.

## Silenced and inhibited alerts

An alert can be quiet for two different reasons, and Alertmanager reports both as
`suppressed`:

- **silenced** — somebody decided it should be quiet, and said why.
- **inhibited** — another, more serious alert is firing, so this one is masked as
  a consequence. Nobody chose it.

Zabbix has no inhibition; an acknowledged or suppressed problem is `silenced`.

Four chips in the toolbar (and in the TV filter panel) choose what to show:

| Chip | Shows |
|---|---|
| **Firing** | everything that is not suppressed — the default |
| **Silenced** | the alerts somebody has silenced |
| **Inhibited** | the alerts another alert is masking |
| **All** | everything, whatever the chips |

They combine, the way the source chips do: each click adds a chip or takes it out.
`Silenced` + `Inhibited` shows everything suppressed and nothing else; `Firing` +
`Silenced` leaves out only the inhibited ones. `All` clears the selection, and so
does taking the last chip out.

`Firing` covers every status that is not suppressed, `pending` included, so there
is no chip for it.

Selecting only suppressed kinds answers "what am I not being told about right
now?" — an empty list then says so (*No silenced alerts*) instead of showing the
green all-clear. A source that has not answered is still reported first, whatever
the chips say: "nothing is silenced" is not a claim to make while the source that
would know is unreachable.

## Grouping

```yaml
display:
  group_by: [team]
```

Alerts are grouped under collapsible headers, one per combination of values.
Groups are ordered by their most severe alert. An alert without the label goes
to a `<missing>` group.

## TV mode

TV mode is a dense view for wall screens: one aligned row per alert, and a
small status bar (source dots, clock, last refresh, version).

- Press **`T`** to toggle it, **`Escape`** to leave it.
- **`Ctrl+F`** or **`/`** opens the filter panel with the search field focused,
  without leaving TV mode. `Escape` empties the field, then closes the panel.
  **`Ctrl+F` again**, from that field, opens the browser's own find-in-page.
- Whenever the filters hide anything, the status bar shows the count and what is
  filtering — a wall screen has to say why it is displaying 12 alerts out of 54,
  and the panel holding the chips is closed most of the time.
- For a wall screen, open the dashboard with **`?tv=1`**: it always starts in TV
  mode.
- `tv_mode_default: true` starts in TV mode only in browsers where nobody has
  used the TV button yet.
- On a kiosk, `link_new_tab: false` keeps links in the same tab.

## Search and URL parameters

The search box accepts free text and label filters, separated by commas:

```
team=sre, hostname~web
team=sre|dba                several values
severity=critical, team!=dba
```

`=` is an exact match, `~` contains, `!=` excludes, `|` separates values.

`Ctrl+F` or `/` jumps to the search box — in TV mode, to the one in the filter
panel, which those keys open. Both take over from the browser's find-in-page,
which only searches the rows already on screen.

To get it anyway, **press `Ctrl+F` a second time** without leaving the search
box: the dashboard lets that press through and the browser opens its own bar.
Useful for finding a word inside a row — a link, an annotation — rather than
narrowing the list. Leaving the search box puts the first behaviour back:
click elsewhere, or press `Escape` until the box loses focus (twice, if there
is a query to clear first).

Filters are kept in the URL, so a view can be bookmarked or put on a screen:

| Parameter | Effect |
|---|---|
| `?q=team=sre` | search / label filter |
| `?sev=critical,error` | severity filter — one or several, comma-separated |
| `?src=Zabbix` | source filter |
| `?show=silenced,inhibited` | `firing` (default), `silenced`, `inhibited`, comma-separated, or `all` — see [above](#silenced-and-inhibited-alerts) |
| `?silenced=1` | the old spelling of `?show=all`, still honoured |
| `?tv=1` / `?tv=0` | force TV mode on or off |
| `?theme=dark` | `auto`, `light` or `dark` |

For one screen per team, use a single AlertView and a filter in each screen's
URL: `?tv=1&q=team=sre`.

## Theme and time

```yaml
display:
  theme: "auto"                 # auto, light or dark
  custom_css: "https://example.com/alertview.css"
  timezone: "local"             # local, UTC or e.g. Europe/Paris
```

- `auto` follows the operating system and switches live. The theme button in
  the page overrides it for that browser.
- `custom_css` adds a stylesheet on top of the theme.
- `timezone` applies to the times shown and to the TV clock.

## Notifications

```yaml
display:
  play_sounds: true
```

- **Sounds**: a beep per new alert batch, with a tone per severity. Browsers only
  allow sound once someone has clicked on the page.
- **Desktop notifications**: click 🔔 in the page to allow them.

Only really new alerts are announced: not the ones already present at startup,
nor those of a source coming back after an outage.

## Two banners

- **Backend unreachable** (red): the browser cannot reach AlertView. The list is
  dimmed and frozen, and it retries every 15 seconds.
- **Configuration rejected** (amber): the last edit of the file was refused;
  the previous configuration is still running.

When the list is empty, it says why: ✅ no alerts, ⏳ a source has not answered
yet, ⚠ a source is unreachable.
