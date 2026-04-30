# Minimal Configuration

This example shows the simplest possible AlertView configuration to get you started quickly.

## Basic Alertmanager Setup

The most minimal configuration connects to a single Alertmanager instance:

```yaml
# config.yaml
sources:
  - name: alertmanager
    kind: alertmanager
    url: http://localhost:9093

port: 8080
```

**Explanation:**
- `sources`: List of alert sources (only Alertmanager in this case)
  - `name`: A friendly name for this source (used in the UI)
  - `kind`: The type of source (`alertmanager`, `grafana`, or `zabbix`)
  - `url`: The URL to the Alertmanager API
- `port`: The port AlertView will listen on (default: 8080)

## Run AlertView

```bash
# Run with the config file
alertview --config config.yaml

# Or if config.yaml is in the current directory, just:
alertview
```

Then open your browser to http://localhost:8080

## Minimal Grafana Setup

```yaml
# config.yaml
sources:
  - name: grafana
    kind: grafana
    url: https://grafana.example.com
    api_key: "your-api-key"

port: 8080
```

**Note:** Grafana requires an API key for authentication.

## Minimal Zabbix Setup

```yaml
# config.yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password

port: 8080
```

## Command Line Only

You can also configure AlertView entirely via command line arguments and environment variables:

```bash
# Set via command line
alertview --port 9090

# Set via environment variables
export ALERTVIEW_PORT=9090
export ALERTVIEW_REFRESH_INTERVAL=60
alertview
```

## Docker Minimal Setup

```yaml
# docker-compose.yml
version: '3'

services:
  alertview:
    image: ghcr.io/your-org/alertview:latest
    ports:
      - "8080:8080"
    environment:
      - ALERTVIEW_PORT=8080
```

With a config file:

```yaml
# docker-compose.yml
version: '3'

services:
  alertview:
    image: ghcr.io/your-org/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/alertview/config.yaml:ro
```

## Next Steps

Once you have a minimal setup working, you can:

1. **Add more sources**: Monitor multiple Alertmanager, Grafana, or Zabbix instances
2. **Customize the display**: Change theme, timezone, refresh interval
3. **Add link templates**: Make alert links point to your dashboards
4. **Enable caching**: Reduce load on your monitoring systems
5. **Set up authentication**: Secure your AlertView instance

See the other examples for more advanced configurations.
