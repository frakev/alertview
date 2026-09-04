# Environment Variables

AlertView supports configuration through environment variables. This allows you to configure AlertView without modifying the configuration file, which is particularly useful for containerized deployments.

## Configuration Priority

Environment variables take precedence over configuration file settings. The priority order is:

1. **Environment Variable** (highest priority)
2. **Configuration File**
3. **Default Value** (lowest priority)

## Available Environment Variables

### Server Configuration

| Variable | Default | Description | Config File Equivalent |
|----------|---------|-------------|------------------------|
| `ALERTVIEW_PORT` | 8080 | Port to listen on | `port` |
| `ALERTVIEW_REFRESH_INTERVAL` | 30 | Seconds between auto-refreshes | `refresh_interval` |
| `ALERTVIEW_TLS_INSECURE` | false | Skip TLS certificate verification | `tls_insecure` |
| `ALERTVIEW_CACHE_TTL` | 0 | Cache TTL in seconds (0 = disabled) | `cache_ttl_seconds` |
| `ALERTVIEW_LOG_FORMAT` | text | Log format: `text` or `json` | `log_format` |

### Example: Server Configuration

```bash
# Set port and refresh interval
export ALERTVIEW_PORT=9090
export ALERTVIEW_REFRESH_INTERVAL=60

# Enable JSON logs and caching
export ALERTVIEW_LOG_FORMAT=json
export ALERTVIEW_CACHE_TTL=60

# Skip TLS verification (for development)
export ALERTVIEW_TLS_INSECURE=true

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
