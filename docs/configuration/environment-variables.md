# Environment Variables

AlertView supports configuration through environment variables. This allows you to configure AlertView without modifying the configuration file, which is particularly useful for containerized deployments.

## Configuration Priority

**The configuration file wins.** These variables are *defaults* for keys the
file leaves out, not overrides:

1. **Configuration File** (highest priority)
2. **Environment Variable**
3. **Built-in Default** (lowest priority)

So setting `ALERTVIEW_PORT=9090` has no effect if `port:` is written in the
file. Remove the key to let the variable through.

> There is no `${VARIABLE}` interpolation inside the YAML. A token written as
> `bearer_token: "${GRAFANA_TOKEN}"` is sent literally, as those nine
> characters. Source credentials have no environment fallback at all — put the
> whole file in a Kubernetes Secret instead, see
> [Secrets Management](../deployment/kubernetes.md#secrets-management).

## Available Environment Variables

This is the complete list. A test asserts that it matches the variables the
code actually reads, in both directions — nothing here is invented, and nothing
real is missing.

| Variable | Default | Description | Config File Equivalent |
|----------|---------|-------------|------------------------|
| `ALERTVIEW_CONFIG` | `config.yaml` | Path to the configuration file | — (also `--config`) |
| `ALERTVIEW_PORT` | 8080 | Port to listen on | `port` |
| `ALERTVIEW_REFRESH_INTERVAL` | 30 | Seconds between browser refreshes | `refresh_interval` |
| `ALERTVIEW_CACHE_TTL` | 0 | Seconds between source polls; 0 means use `refresh_interval` | `cache_ttl_seconds` |
| `ALERTVIEW_LOG_FORMAT` | text | Log format: `text` or `json` | `log_format` |
| `ALERTVIEW_CONFIG_WATCH_METHOD` | polling | How the config file is watched: `polling` or `inotify` | `config_watch_method` |
| `ALERTVIEW_CONFIG_POLL_INTERVAL` | 10 | Seconds between checks when polling | `config_poll_interval` |

`RUST_LOG` sets the log level (`error`, `warn`, `info`, `debug`, `trace`) and is
read by the logging layer rather than by the configuration.

### Example: Server Configuration

```bash
# Set port and refresh interval
export ALERTVIEW_PORT=9090
export ALERTVIEW_REFRESH_INTERVAL=60

# Enable JSON logs, and poll the sources once a minute
export ALERTVIEW_LOG_FORMAT=json
export ALERTVIEW_CACHE_TTL=60

# React to config edits immediately rather than every 10 seconds
export ALERTVIEW_CONFIG_WATCH_METHOD=inotify

cargo run -- config.yaml
```

## Docker Usage

Environment variables are particularly useful with Docker:

```bash
# Run with environment variables
docker run -d \
  -p 9090:9090 \
  -e ALERTVIEW_PORT=9090 \
  -e ALERTVIEW_REFRESH_INTERVAL=60 \
  -e ALERTVIEW_LOG_FORMAT=json \
  -v $(pwd)/config.yaml:/config/config.yaml:ro \
  ghcr.io/frakev/alertview:latest
```

## Docker Compose Usage

```yaml
version: '3.8'
services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "9090:9090"
    environment:
      - ALERTVIEW_PORT=9090
      - ALERTVIEW_REFRESH_INTERVAL=60
      - ALERTVIEW_LOG_FORMAT=json
      - ALERTVIEW_CACHE_TTL=60
    volumes:
      - ./config.yaml:/config/config.yaml:ro
```

## Kubernetes Usage

In Kubernetes, you can set environment variables in your deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: alertview
spec:
  template:
    spec:
      containers:
      - name: alertview
        image: ghcr.io/frakev/alertview:latest
        env:
        - name: ALERTVIEW_PORT
          value: "8080"
        - name: ALERTVIEW_REFRESH_INTERVAL
          value: "60"
        - name: ALERTVIEW_LOG_FORMAT
          value: "json"
        - name: ALERTVIEW_CACHE_TTL
          value: "60"
        args: ["/config/config.yaml"]
        volumeMounts:
        - name: config
          mountPath: /config
      volumes:
      - name: config
        configMap:
          name: alertview-config
```

Or use a ConfigMap for environment variables:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: alertview-env
data:
  ALERTVIEW_PORT: "8080"
  ALERTVIEW_REFRESH_INTERVAL: "60"
  ALERTVIEW_LOG_FORMAT: "json"
  ALERTVIEW_CACHE_TTL: "60"
```

## Complete Example

Here's a complete example using only environment variables (no config file):

```bash
# Set all configuration via environment variables
export ALERTVIEW_PORT=8080
export ALERTVIEW_REFRESH_INTERVAL=30
export ALERTVIEW_LOG_FORMAT=json
export ALERTVIEW_CACHE_TTL=60

# Create a minimal config file with just sources
cat > config.yaml <<EOF
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager:9093"
EOF

# Run AlertView
cargo run -- config.yaml
```

## Best Practices

### 1. Use Environment Variables for Secrets

**❌ Don't do this:**
```yaml
# config.yaml
sources:
  - name: "Grafana"
    type: grafana
    bearer_token: "my-secret-token"  # Hardcoded secret!
```

**✅ Do this instead:**
```yaml
# config.yaml
sources:
  - name: "Grafana"
    type: grafana
    bearer_token: "${GRAFANA_TOKEN}"  # Reference env var
```

```bash
# Set secret via environment
export GRAFANA_TOKEN="my-secret-token"
cargo run -- config.yaml
```

### 2. Use for Environment-Specific Settings

```bash
# Development
export ALERTVIEW_PORT=3000
export ALERTVIEW_REFRESH_INTERVAL=10
export ALERTVIEW_LOG_FORMAT=json

# Production
export ALERTVIEW_PORT=8080
export ALERTVIEW_REFRESH_INTERVAL=60
export ALERTVIEW_LOG_FORMAT=text
```

### 3. Combine with Config File

Use environment variables for settings that change between environments, and use the config file for everything else:

```yaml
# config.yaml (same for all environments)
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager:9093"
  - name: "Grafana"
    type: grafana
    url: "http://grafana:3000"
    bearer_token: "${GRAFANA_TOKEN}"

display:
  labels:
    - namespace
    - job
    - instance
  theme: "dark"
```

```bash
# Development
export ALERTVIEW_PORT=3000
export ALERTVIEW_REFRESH_INTERVAL=10
export GRAFANA_TOKEN="dev-token"
cargo run -- config.yaml

# Production
export ALERTVIEW_PORT=8080
export ALERTVIEW_REFRESH_INTERVAL=60
export GRAFANA_TOKEN="prod-token"
cargo run -- config.yaml
```

## Type Conversion

All environment variables are parsed as strings and then converted to the appropriate type:

- **Numbers**: Parsed as integers (e.g., `ALERTVIEW_PORT=8080` → `8080` as u16)
- **Booleans**: Case-insensitive (e.g., `true`, `True`, `TRUE`, `1` → `true`)
- **Strings**: Used as-is

If a value cannot be parsed, the default value is used instead.

## Debugging

To see what environment variables are being used:

```bash
# Print all AlertView-related environment variables
printenv | grep ALERTVIEW

# Run with debug logging to see configuration
RUST_LOG=debug cargo run -- config.yaml
```

## Limitations

1. **No nested configuration**: Environment variables can only set top-level and source-level settings, not deeply nested configurations.

2. **No arrays**: Environment variables cannot set array values (like `display.labels`).

3. **String values only**: All environment variables are strings and must be convertible to the expected type.

For complex configurations, use the configuration file instead.
