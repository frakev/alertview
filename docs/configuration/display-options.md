# Display Options

Customize how alerts are displayed in AlertView with the `display` configuration section.

## Configuration Reference

```yaml
display:
  # Labels to show on each alert card
  labels:
    - namespace
    - job
    - instance
    - cluster
    - node
    - pod
    - host
    - hostgroup
  
  # Theme settings
  theme: "auto"          # "auto" (follow the OS), "dark" or "light"
  custom_css: ""         # optional extra stylesheet
  
  # Timezone settings
  timezone: "local"  # "local", "UTC", or IANA timezone
  
  # Sound notifications
  play_sounds: false  # Enable sound notifications for new alerts
  
  # Alert grouping
  group_by: []  # Labels to group alerts by (e.g., ["namespace", "job"])

  # Severity levels, most severe first
  severity_order: ["critical", "error", "high", "warning", "info", "none"]

  # Labels shown in front of the alert name
  prefix_labels: ["hostname"]
  prefix_separator: " / "

  # Start in TV mode
  tv_mode_default: false

  # Whole-alert link, and the "open in the source" arrow
  alert_link_template: ""
  source_link: true
  link_new_tab: true
```

## Prefix Labels

Labels shown **in front of the alert name**, joined by a separator, in both
normal and TV mode:

```yaml
display:
  prefix_labels: ["hostname", "namespace"]
  prefix_separator: " / "
```

```
● srv-01 / prod  systemd_unit_failed  ERROR  FIRING  Systemd "salt-exporter.service" failed
```

- Defaults to `["hostname"]`, so a `hostname` label shows up without any
  configuration.
- Only the labels the alert actually carries are shown, in the order of the
  list. Nothing is rendered — not even a stray separator — when it carries none.
- A prefix label is shown even when it is **not** listed in `display.labels`,
  and it is removed from the trailing label chips so it never appears twice.
- The prefix is truncated with an ellipsis past ~22 characters so a long FQDN
  cannot push the alert name out of a TV row.

## TV Mode by Default

```yaml
display:
  tv_mode_default: true
```

Starts in TV mode in a browser where **nobody has used the TV button yet**. An
explicit choice wins over it, and stays: once someone toggles TV mode, that
browser remembers their choice and ignores this setting.

For a wall display, put `?tv=1` in the URL instead — it wins over both the
stored choice and this setting, so the screen always comes up in TV mode.

## Theme

```yaml
display:
  theme: "auto"     # "auto" (default), "dark", "light"
  custom_css: "https://example.com/alertview-theme.css"
```

- `auto` follows the browser/OS light-dark setting, and **switches live** when
  the OS switches — a wall display goes light in the morning and dark at night
  on its own.
- The theme button cycles `auto → light → dark`; its icon shows the preference
  (auto / sun / moon), and its tooltip the resolved theme.
- Precedence: `?theme=auto|light|dark` in the URL, then the choice made in this
  browser, then `display.theme`.
- `custom_css` is layered on top of the theme. For backwards compatibility a
  `theme` holding a URL is still treated as a custom stylesheet.

## Alert Links

Each alert can carry **two independent links**, and either can be left out:

```yaml
display:
  alert_link_template: "https://wiki.example.com/runbook/{{.Labels.alertname}}?host={{.Labels.hostname}}"
  source_link: true
  link_new_tab: true
```

| | Declared by | Rendered as |
|---|---|---|
| Alert link | `alert_link_template` | the whole card/row is clickable |
| Source link | `link_template` → the alert's generator URL → `dashboard_url` | the ↗ button on the right |

- Both can be overridden per source: `alert_link_template` and `source_link`
  under a `sources:` entry win over the `display:` values.
- No `alert_link_template` anywhere means the alert is simply not clickable;
  `source_link: false` hides the ↗ button.
- Placeholders: `{{.Labels.x}}`, `{{.Annotations.x}}`, `{{.Name}}`,
  `{{.Severity}}`, `{{.Status}}`, `{{.Source}}`, `{{.SourceType}}`,
  `{{.Fingerprint}}`, `{{.StartsAt}}`, `{{.EndsAt}}`.
- Values are percent-encoded, so a label containing a space, `&` or `/` cannot
  break the URL — note that this also means a value cannot be used as a path
  separator.
- If the alert does not carry a label the template asks for, the template is
  **not** used: the source link falls back to the next candidate, and the alert
  link is dropped (no URL with a leftover `{{.Labels.x}}` in it).
- Only `http` and `https` links are rendered; anything else, including a
  `javascript:` generator URL coming from a source, is ignored.

## Severity Order

Alerts are sorted by severity, most severe first, then by age. `severity_order`
defines that ranking:

```yaml
display:
  severity_order: ["critical", "error", "high", "warning", "info", "none"]
```

- The default is the list above, so `severity` labels such as `error` are ranked
  without any configuration.
- Any severity **not** listed sorts after every listed level. Add your own levels
  to the list to place them: `["disaster", "critical", "error", "notice"]`.
- Matching is case-insensitive and understands the aliases `crit`, `err`, `warn`
  and `information`.
- The same order drives the severity filter chips (normal and TV mode), the sound
  picked for a batch of new alerts, and the severity badges on groups.

Only `critical`, `error`, `high`, `warning`, `info` and `none` have dedicated
colors; a custom level is displayed with the neutral style.

## Alert Grouping

AlertView can group alerts by one or more labels for better organization.

### Configuration

```yaml
display:
  group_by: ["namespace", "job"]
```

### How It Works

1. **Group Key**: Alerts are grouped by the combination of label values
2. **Collapsible Groups**: Groups are displayed as collapsible sections in the UI
3. **Severity Counts**: Each group shows the count of alerts by severity
4. **Group Label**: The group header displays the label key=value pairs

### Examples

**Group by namespace only:**
```yaml
group_by: ["namespace"]
```
- Creates groups like: `namespace=production`, `namespace=staging`

**Group by namespace and job:**
```yaml
group_by: ["namespace", "job"]
```
- Creates groups like: `namespace=production,job=api`, `namespace=production,job=web`

**Group by multiple labels:**
```yaml
group_by: ["cluster", "namespace", "service"]
```
- Creates hierarchical groups

### Disabling Grouping

```yaml
group_by: []  # or omit the field entirely
```

### Grouping with Missing Labels

If an alert doesn't have one of the `group_by` labels:
- The label is shown as `<missing>` in the group key
- The alert is still included in the group

### Best Practices

1. **Start simple**: Begin with one label (e.g., `namespace`)
2. **Avoid too many labels**: More than 3 labels can create too many groups
3. **Use meaningful labels**: Choose labels that naturally group your alerts
4. **Consider cardinality**: Labels with high cardinality (many unique values) create many groups

## Labels Configuration

The `labels` array specifies which labels from your alerts should be displayed on each alert card.

### Default Labels

If you don't specify any labels, AlertView will show these by default:
- namespace
- job
- instance
- cluster
- node

### Custom Labels

```yaml
display:
  labels:
    - namespace
    - job
    - pod
    - service
    - team
```

### How Labels Work

1. **Label Matching**: AlertView looks for each label in the `labels` array in the alert's labels
2. **Display Order**: Labels are displayed in the order specified in the array
3. **Fallback**: If a label doesn't exist for an alert, it's simply skipped
4. **Special Labels**: `severity` and `alertname` are always shown as badges, regardless of this setting

### Example with Custom Labels

```yaml
display:
  labels:
    - environment  # Shows: environment=production
    - application # Shows: application=web
    - version     # Shows: version=v1.2.3
```

### Common Label Patterns

**Kubernetes:**
```yaml
labels:
  - namespace
  - pod
  - node
  - deployment
  - statefulset
  - daemonset
```

**Prometheus:**
```yaml
labels:
  - job
  - instance
  - alertname
  - severity
```

**Custom Applications:**
```yaml
labels:
  - app
  - env
  - region
  - team
  - service
```

## Theme Configuration

AlertView supports three theme options:

### Built-in Themes

**Dark Theme (default):**
```yaml
display:
  theme: "dark"
```
- Dark background with light text
- Best for low-light environments
- TV mode works well with dark theme

**Light Theme:**
```yaml
display:
  theme: "light"
```
- Light background with dark text
- Best for well-lit environments
- More accessible for some users

### Custom CSS Theme

You can provide a URL to a custom CSS file:

```yaml
display:
  theme: "https://example.com/custom-alertview-theme.css"
```

**Custom CSS Requirements:**
- Must be accessible from the browser (CORS headers if needed)
- Should follow AlertView's CSS structure
- Can override any existing styles

### Custom CSS Example

```css
/* custom-theme.css */

/* Change background color */
body {
  background-color: #1a1a2e;
}

/* Change alert card colors */
.alert-card.sev-critical {
  border-left: 4px solid #ff6b6b;
  background-color: rgba(255, 107, 107, 0.1);
}

.alert-card.sev-high {
  border-left: 4px solid #ffa500;
}

.alert-card.sev-warning {
  border-left: 4px solid #ffd166;
}

/* Change text colors */
.alert-name {
  color: #e94560;
}

/* Custom severity badges */
.sev-badge.sev-critical {
  background-color: #ff6b6b;
  color: white;
}
```

### Theme Switching

Users can switch between dark and light themes using the theme toggle button in the UI. This preference is:
- Saved in localStorage
- Persisted across page reloads
- Included in URL for sharing

Custom CSS themes are always applied in addition to the selected theme.

## Timezone Configuration

AlertView can display timestamps in different timezones.

### Options

**Local Timezone (default):**
```yaml
display:
  timezone: "local"
```
- Uses the browser's local timezone
- Each user sees times in their own timezone
- Recommended for most use cases

**UTC:**
```yaml
display:
  timezone: "UTC"
```
- All times displayed in UTC
- Consistent across all users
- Good for distributed teams

**IANA Timezone:**
```yaml
display:
  timezone: "Europe/Paris"
```
- Uses a specific timezone from the [IANA Time Zone Database](https://www.iana.org/time-zones)
- All users see the same timezone
- Good for team-specific dashboards

### Common IANA Timezones

| Region | Timezone |
|--------|----------|
| US East | America/New_York |
| US West | America/Los_Angeles |
| UK | Europe/London |
| Central Europe | Europe/Paris |
| Germany | Europe/Berlin |
| Japan | Asia/Tokyo |
| Australia | Australia/Sydney |
| India | Asia/Kolkata |

### Timezone in TV Mode

The timezone setting also affects the clock displayed in TV mode. The clock will show the current time in the configured timezone.

## Sound Notifications

AlertView can play sounds when new alerts arrive.

### Configuration

```yaml
display:
  play_sounds: true  # Enable sound notifications
```

### How It Works

1. **New Alert Detection**: AlertView detects when new alerts appear (not previously seen)
2. **Severity-Based Sounds**: Different sounds are played based on the highest severity of new alerts
3. **Web Audio API**: Sounds are generated using the browser's Web Audio API (no external files needed)
4. **Auto-Play**: Sounds play automatically when new alerts arrive

### Sound Presets

AlertView includes built-in sound presets for each severity level:

| Severity | Sound Description |
|----------|-------------------|
| critical | Two low-frequency beeps (800Hz, 600Hz) |
| high | Two mid-frequency beeps (600Hz, 500Hz) |
| warning | One medium-frequency beep (400Hz) |
| info | One high-frequency beep (300Hz) |

### Customizing Sounds

To customize sounds, you would need to modify the JavaScript in `static/app.js`. Look for the `SOUND_PRESETS` object and `AudioContext.playBeep()` function.

### Browser Requirements

- **Web Audio API**: Required for sound notifications
- **User Interaction**: Most browsers require user interaction before playing sounds
- **Autoplay Policy**: Some browsers may block autoplay of sounds

### Troubleshooting Sounds

**Sounds not playing?**
1. Check if `play_sounds` is set to `true` in config
2. Verify the API response includes `play_sounds: true`
3. Check browser console for errors
4. Try clicking on the page first (autoplay policy)
5. Verify Web Audio API is supported in your browser

**Sounds too quiet?**
- Adjust your system volume
- The sounds are intentionally subtle (volume ~0.1)

### Real-time Notifications

AlertView supports **Server-Sent Events (SSE)** for real-time push notifications of new alerts.

#### How It Works

1. **SSE Connection**: When the page loads, AlertView automatically connects to the `/events` endpoint
2. **New Alert Detection**: The server detects new alerts (not previously seen in cache)
3. **Push Notification**: New alerts are pushed to all connected clients via SSE
4. **Desktop Notification**: If browser notifications are enabled, a desktop notification is shown
5. **Sound Notification**: If `play_sounds` is enabled, a sound is played
6. **Auto-Refresh**: The alert list is automatically refreshed

#### Browser Notifications

AlertView can show **desktop notifications** for new alerts:

1. **Permission Required**: Click the 🔔 button to grant notification permission
2. **Notification Content**: Shows alert count, severity, and names
3. **Click Behavior**: Clicking a notification focuses the AlertView window
4. **Auto-Close**: Notifications auto-close after 9 seconds

**Browser Support:**
- ✅ Chrome
- ✅ Firefox
- ✅ Edge
- ✅ Safari
- ❌ Some mobile browsers (limited support)

**Troubleshooting Notifications:**
- If notifications don't appear, check browser permission settings
- Notifications are blocked if permission was previously denied
- To reset: Clear site permissions in browser settings

#### SSE Connection Details

- **Endpoint**: `/events`
- **Protocol**: Server-Sent Events (SSE)
- **Event Type**: `new_alert`
- **Data Format**: JSON (complete alert object)
- **Reconnection**: Automatic with exponential backoff (2s, 4s, 8s, 16s, 32s)
- **Max Retries**: 5 attempts before giving up

#### Disabling Real-time Features

To disable real-time notifications:
1. **Disable SSE**: Not configurable (always enabled when supported)
2. **Disable Sounds**: Set `play_sounds: false` in config
3. **Disable Desktop Notifications**: Click the 🔔 button to revoke permission

## Complete Display Configuration Example

```yaml
display:
  # Show these labels on alert cards
  labels:
    - namespace
    - job
    - instance
    - pod
    - cluster
    - host
    - hostgroup
  
  # Use dark theme
  theme: "dark"
  
  # Use local timezone
  timezone: "local"
  
  # Enable sound notifications
  play_sounds: true
```

## Best Practices

1. **Start with defaults**: Begin with the default label set and add/remove as needed
2. **Consider your audience**: Choose labels that are meaningful to your users
3. **Theme consistency**: Match the theme to your environment (dark for NOC, light for offices)
4. **Timezone consistency**: Use the same timezone across all your monitoring tools
5. **Sound considerations**: Only enable sounds in environments where they're appropriate
6. **Test on mobile**: Verify your label choices work well on mobile devices

## Advanced Usage

### Dynamic Label Selection

You can use different label configurations for different environments:

```yaml
# Development
 display:
   labels:
     - namespace
     - pod
   theme: "light"
   timezone: "local"
   play_sounds: false

# Production
 display:
   labels:
     - namespace
     - job
     - instance
     - cluster
   theme: "dark"
   timezone: "UTC"
   play_sounds: true
```

### URL-Persisted Display Settings

Display settings can be included in the URL for sharing:
- `?theme=dark` or `?theme=light`
- `?timezone=Europe/Paris`

Example: `http://alertview:8080/?theme=dark&timezone=UTC`
