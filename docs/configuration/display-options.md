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
  theme: "dark"  # "dark", "light", or URL to custom CSS
  
  # Timezone settings
  timezone: "local"  # "local", "UTC", or IANA timezone
  
  # Sound notifications
  play_sounds: false  # Enable sound notifications for new alerts
  
  # Alert grouping
  group_by: []  # Labels to group alerts by (e.g., ["namespace", "job"])
```

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
