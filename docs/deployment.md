# Deployment

AlertView is a single binary with no database. It needs a configuration file
(see [Configuration](configuration.md)) and network access to your sources.

> **AlertView has no authentication.** Anyone who reaches it sees your alerts.
> Put it behind a reverse proxy with authentication and TLS
> ([below](#reverse-proxy)), never directly on an untrusted network.

## Docker

Images are published for every release on `ghcr.io/frakev/alertview`, tagged
with the version (`0.13.3`) and `latest`, for `linux/amd64`.

```bash
docker run -d -p 8080:8080 \
  -v $(pwd)/conf:/config:ro \
  ghcr.io/frakev/alertview:0.13.3
```

The image reads `/config/config.yaml` and runs as an unprivileged user (65532).
Mount the **directory** rather than the single file: editors that save by
renaming (vim…) would otherwise leave the container on the old file, and edits
would not be picked up.

With Docker Compose:

```yaml
services:
  alertview:
    image: ghcr.io/frakev/alertview:0.13.3
    ports: ["8080:8080"]
    volumes: ["./conf:/config:ro"]
    restart: unless-stopped
```

## Binary

Each [GitHub release](https://github.com/frakev/alertview/releases) has a
Linux x86-64 binary, `alertview-linux-amd64`:

```bash
./alertview-linux-amd64 /etc/alertview/config.yaml
```

To build it yourself, see [Development](development.md).

## Kubernetes

The repository has ready-to-use manifests:

| File | Content | To adapt |
|---|---|---|
| `01-namespace.yaml` | the `alertview` namespace | — |
| `02-configmap.yaml` | the configuration | your sources |
| `03-deployment.yaml` | one pod, hardened (non-root, read-only filesystem) | the image version |
| `04-service.yaml` | ClusterIP service on port 8080 | — |
| `05-ingress.yaml` | ingress with TLS (cert-manager, Traefik) | your domain, middlewares |

```bash
kubectl apply -f 01-namespace.yaml -f 02-configmap.yaml -f 03-deployment.yaml \
  -f 04-service.yaml -f 05-ingress.yaml
# or: make deploy   (KUBECTL="microk8s kubectl" make deploy for microk8s)
```

**Configuration changes** are picked up without restarting the pod: Kubernetes
updates the mounted file within a minute or so, and AlertView reloads it.

### Credentials

`02-configmap.yaml` is an example. As soon as it holds real tokens, use a
**Secret** instead: a ConfigMap is readable by anything that can list
ConfigMaps in the namespace, and usually ends up in git.

```bash
kubectl create secret generic alertview-config \
  --from-file=config.yaml=./config.yaml -n alertview
```

Then, in `03-deployment.yaml`, replace the ConfigMap volume with the Secret, and
let the pod's user read it:

```yaml
    spec:
      securityContext:
        fsGroup: 65532          # Secret files are owned by root
      volumes:
        - name: config
          secret:
            secretName: alertview-config
            defaultMode: 0440
```

Don't try `bearer_token: "${TOKEN}"` with an environment variable from the
Secret: AlertView does not expand variables in the file.

SealedSecrets, External Secrets or SOPS work the same way: they all produce a
Secret.

## Reverse proxy

The proxy must add authentication and TLS. Two constraints:

- **Serve AlertView at the root of a (sub)domain** (`https://alerts.example.com/`),
  not under a path: the page loads `/app.js` and `/api/alerts` from the root.
- **Don't buffer `/events`**: it is a live stream (server-sent events). A
  buffering proxy delays new-alert notifications.

Example with nginx and basic authentication:

```nginx
server {
    listen 443 ssl;
    server_name alerts.example.com;
    ssl_certificate     /etc/letsencrypt/live/alerts.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/alerts.example.com/privkey.pem;

    auth_basic "AlertView";
    auth_basic_user_file /etc/nginx/alertview.htpasswd;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
    }

    location /events {
        proxy_pass http://127.0.0.1:8080;
        proxy_buffering off;
        proxy_read_timeout 1h;
    }
}
```

HTTPS also makes AlertView installable as an app (PWA) from the browser menu,
on phones and desktops.

## Monitoring AlertView

- **`/health`** answers `200 OK` as soon as the server runs. The probes in
  `03-deployment.yaml` use it. It does not reflect the sources: a failing source
  is reported in the dashboard and in `/api/alerts`.
- There is **no `/metrics`** endpoint. To be alerted when AlertView is down,
  probe `/health` from outside (blackbox exporter, uptime checker…).
- Logs go to stdout; `log_format: json` for a log collector, `RUST_LOG=debug` for
  details.
- On `SIGTERM`, AlertView finishes the requests in progress and exits within
  milliseconds: rolling updates need no special setting.

## Upgrading

Change the image version (or the binary) and restart. The
[CHANGELOG](../CHANGELOG.md) lists what changes between versions; read it for
configuration keys that moved — AlertView refuses to start on a key it does not
know, and says what to write instead.
