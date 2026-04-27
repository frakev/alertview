# AlertView

A lightweight alert dashboard for Alertmanager and Grafana. Built with Rust (Axum) and a single-page HTML frontend — no database, no dependencies at runtime.

**Features:**
- Aggregates alerts from multiple Alertmanager and/or Grafana sources
- Severity-colored cards (critical / high / warning / info)
- Filter by severity, status (firing / silenced / pending), and source
- TV mode for wall displays — full-screen, auto-refresh, URL-persisted filters
- Dark/light theme

## Requirements

- Rust 1.75+ (build only)
- Docker (optional, for containerized deployment)

## Configuration

Copy `config.example` to `config.yaml` and edit it:

```yaml
port: 8080
refresh_interval: 30   # seconds between auto-refreshes
tls_insecure: false    # set true to skip TLS verification (self-signed certs)

sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager.monitoring.svc.cluster.local:9093"
    dashboard_url: "https://grafana.example.com/alerting/list"  # optional link on cards

  - name: "Grafana"
    type: grafana
    url: "http://grafana.monitoring.svc.cluster.local:3000"
    bearer_token: "glsa_xxxx"   # Grafana service account token
    # or: basic_auth: { username: admin, password: secret }

display:
  labels:          # which labels to show on each alert card
    - namespace
    - job
    - instance
```

> `config.yaml` is gitignored — never commit credentials.

## Running locally

```bash
cargo run -- config.yaml
# open http://localhost:8080
```

## Docker

```bash
# Build
docker build -t alertview .

# Run
docker run -p 8080:8080 -v $(pwd)/config.yaml:/config/config.yaml alertview
```

The image is also published automatically to GHCR on every push to `main`:

```bash
docker pull ghcr.io/frakev/alertview:latest
```

## Kubernetes deployment

The `k8s/` manifests deploy AlertView into its own namespace using a ConfigMap for configuration.

**1. Edit the manifests**

| File | What to change |
|---|---|
| `02-configmap.yaml` | Alertmanager/Grafana URLs, dashboard link |
| `05-ingress.yaml` | Your domain, TLS secret name, middlewares |

**2. Apply**

```bash
kubectl apply -f 01-namespace.yaml
kubectl apply -f 02-configmap.yaml
kubectl apply -f 03-deployment.yaml
kubectl apply -f 04-service.yaml
kubectl apply -f 05-ingress.yaml
```

Or with the Makefile (requires `kubectl` in PATH, or override `KUBECTL=microk8s kubectl`):

```bash
make deploy
```

**3. Update after a config change**

```bash
kubectl rollout restart deployment/alertview -n alertview
# or:
make restart
```

## CI/CD

The included `.github/workflows/docker-publish.yml` builds and pushes the image to GHCR on every push to `main` and on version tags (`v*`). It uses `GITHUB_TOKEN` — no extra secrets required.

```
push to main  →  ghcr.io/frakev/alertview:main
push v1.2.3   →  ghcr.io/frakev/alertview:1.2.3 + :latest
```

The workflow targets a `self-hosted` runner labeled `k8s-home`. Change `runs-on` in the workflow file if your runner has a different label.

## API

| Endpoint | Description |
|---|---|
| `GET /` | Web UI |
| `GET /api/alerts` | JSON — all alerts aggregated from all sources |
