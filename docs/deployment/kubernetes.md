# Kubernetes Deployment

This guide covers deploying AlertView on Kubernetes, including configuration options, best practices, and troubleshooting tips.

## Prerequisites

- Kubernetes cluster (v1.20+)
- `kubectl` configured to access your cluster

## Quick Start

### Using kubectl

1. **Clone the repository or download the manifests:**

```bash
git clone https://github.com/your-org/alertview.git
cd alertview
```

2. **Create a namespace (optional but recommended):**

```bash
kubectl create namespace alertview
```

3. **Create a ConfigMap with your configuration:**

```bash
kubectl create configmap alertview-config --from-file=config.yaml -n alertview
```

4. **Deploy AlertView:**

```bash
kubectl apply -f 01-namespace.yaml -f 02-configmap.yaml -f 03-deployment.yaml -f 04-service.yaml -f 05-ingress.yaml -n alertview
```

This creates:
- A Deployment with 1 replica
- A Service (ClusterIP type)
- An Ingress resource

5. **Access AlertView:**

```bash
# Get the service URL
kubectl get svc -n alertview

# Or if using ingress, check your ingress controller
kubectl get ingress -n alertview
```

## Kubernetes Manifests

The repository includes the following Kubernetes manifests at the root level:

```
.
├── 01-namespace.yaml       # Namespace configuration
├── 02-configmap.yaml       # ConfigMap for AlertView configuration
├── 03-deployment.yaml      # Deployment configuration
├── 04-service.yaml         # Service configuration
└── 05-ingress.yaml
│   ├── deployment.yaml      # Deployment configuration
│   ├── service.yaml         # Service configuration
│   └── ingress.yaml         # Ingress configuration
```

### deployment.yaml

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: alertview
  labels:
    app: alertview
spec:
  replicas: 1
  selector:
    matchLabels:
      app: alertview
  template:
    metadata:
      labels:
        app: alertview
    spec:
      containers:
      - name: alertview
        image: ghcr.io/your-org/alertview:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 8080
          name: http
        volumeMounts:
        - name: config
          mountPath: /etc/alertview/config.yaml
          subPath: config.yaml
          readOnly: true
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "64Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: config
        configMap:
          name: alertview-config
```

### service.yaml

```yaml
apiVersion: v1
kind: Service
metadata:
  name: alertview
  labels:
    app: alertview
spec:
  type: ClusterIP
  ports:
  - port: 80
    targetPort: 8080
    name: http
  selector:
    app: alertview
```

### ingress.yaml

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: alertview
  labels:
    app: alertview
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    # For basic auth (optional)
    # nginx.ingress.kubernetes.io/auth-type: basic
    # nginx.ingress.kubernetes.io/auth-secret: alertview-auth
    # nginx.ingress.kubernetes.io/auth-realm: "Authentication Required"
spec:
  ingressClassName: nginx
  rules:
  - host: alertview.your-domain.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: alertview
            port:
              number: 80
```

## Configuration

### Using ConfigMap

The recommended approach is to use a ConfigMap for your AlertView configuration:

1. Create a `config.yaml` file with your AlertView configuration
2. Create the ConfigMap:

```bash
kubectl create configmap alertview-config --from-file=config.yaml -n alertview
```

3. Update the ConfigMap when configuration changes:

```bash
kubectl create configmap alertview-config --from-file=config.yaml -n alertview -o yaml --dry-run=client | kubectl replace -f -
```

**Note:** AlertView automatically reloads its configuration when the file changes. However, in Kubernetes, ConfigMap updates require a pod restart to take effect. You can:

- Restart the deployment: `kubectl rollout restart deployment/alertview -n alertview`
- Use a sidecar to sync ConfigMap changes to a file (e.g., `configmap-reload` sidecar)

### Using Environment Variables

You can also configure AlertView using environment variables in the deployment:

```yaml
spec:
  template:
    spec:
      containers:
      - name: alertview
        env:
        - name: ALERTVIEW_PORT
          value: "8080"
        - name: ALERTVIEW_REFRESH_INTERVAL
          value: "30"
        - name: ALERTVIEW_LOG_FORMAT
          value: "json"
        - name: ALERTVIEW_CACHE_TTL
          value: "60"
```

> A value written in `config.yaml` **wins over the environment variable** —
> these are defaults for fields the file leaves out. The `02-configmap.yaml`
> shipped here sets `port` and `refresh_interval` explicitly, so setting
> `ALERTVIEW_PORT` alongside it has no effect. Remove the field from the file
> to let the variable through.
>
> This applies only to the settings above: source URLs, tokens and every
> `display:` option are read from the file alone.

See [Environment Variables](../configuration/environment-variables.md) for a complete list.

### Multiple Configurations

For different environments (staging, production), create separate ConfigMaps:

```bash
# Staging
kubectl create configmap alertview-config-staging --from-file=config-staging.yaml -n alertview-staging

# Production
kubectl create configmap alertview-config-prod --from-file=config-prod.yaml -n alertview-prod
```

## microk8s Deployment

[microk8s](https://microk8s.io/) is a lightweight Kubernetes distribution ideal for local development and testing.

### Installation

1. Install microk8s:

```bash
# On Ubuntu
sudo snap install microk8s --classic --channel=latest/stable

# On other Linux distributions
curl -sL https://microk8s.io/install | sudo bash
```

2. Add your user to the microk8s group:

```bash
sudo usermod -a -G microk8s $USER
sudo chown -f -R $USER ~/.kube
newgrp microk8s
```

3. Enable required addons:

```bash
microk8s enable dns storage ingress helm3
```

4. Verify installation:

```bash
microk8s kubectl get nodes
microk8s kubectl get pods -A
```

### Deploy AlertView on microk8s

1. Create a namespace:

```bash
microk8s kubectl create namespace alertview
```

2. Create the ConfigMap:

```bash
microk8s kubectl create configmap alertview-config --from-file=config.yaml -n alertview
```

3. Deploy AlertView:

```bash
microk8s kubectl apply -f 01-namespace.yaml -f 02-configmap.yaml -f 03-deployment.yaml -f 04-service.yaml -f 05-ingress.yaml -n alertview
```

4. Access AlertView:

```bash
# Get the IP address
microk8s kubectl get svc -n alertview

# Or use port-forwarding
microk8s kubectl port-forward svc/alertview 8080:80 -n alertview
```

5. Enable Ingress (optional):

```bash
# Get the ingress IP
microk8s kubectl get ingress -n alertview

# Add the IP to your /etc/hosts
# e.g., 127.0.0.1 alertview.local
```

## Production Best Practices

### Resource Limits

Set appropriate resource requests and limits based on your expected load:

```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "250m"
  limits:
    memory: "512Mi"
    cpu: "1000m"
```

### Horizontal Pod Autoscaling

For high-traffic deployments, consider adding HPA:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: alertview-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: alertview
  minReplicas: 1
  maxReplicas: 5
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

### Persistent Storage

If you need to persist data:

```yaml
volumes:
- name: data
  persistentVolumeClaim:
    claimName: alertview-data

volumeMounts:
- name: data
  mountPath: /var/lib/alertview
```

### Security

#### Network Policies

Restrict access to AlertView:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: alertview-ingress
spec:
  podSelector:
    matchLabels:
      app: alertview
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          access: alertview
    ports:
    - protocol: TCP
      port: 8080
```

#### Pod Security

The image already runs as `65532:65532`, and AlertView writes nothing to disk —
its configuration is a read-only mount and it keeps no state — so it runs
under a fully restricted context:

```yaml
spec:
  template:
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 65532
        runAsGroup: 65532
        seccompProfile:
          type: RuntimeDefault
      containers:
        - name: alertview
          securityContext:
            allowPrivilegeEscalation: false
            readOnlyRootFilesystem: true
            capabilities:
              drop: ["ALL"]
```

The published image runs under exactly this context: non-root, read-only root
filesystem, no capabilities, no privilege escalation.

Nothing is written to the mounted volume, so no `fsGroup` is required for the
ConfigMap mount shipped here, whose files are world-readable (mode `0644`). It
*is* required as soon as you narrow the file's mode — see
[Secrets Management](#secrets-management) below.

#### Secrets Management

> **AlertView does not expand environment variables inside `config.yaml`.**
> Writing `bearer_token: "${ALERTMANAGER_TOKEN}"` and injecting the value with
> `secretKeyRef` does not work — the placeholder is read as the literal token
> and every request is rejected. Only the settings listed under
> [Using Environment Variables](#using-environment-variables) have an
> environment fallback; source credentials have none.

Since credentials live in the configuration file itself, put the **whole file
in a Secret** rather than a ConfigMap, and mount it exactly where the ConfigMap
would have gone:

```bash
kubectl create secret generic alertview-config \
  --from-file=config.yaml=./config.yaml \
  -n alertview
```

```yaml
    spec:
      securityContext:
        runAsUser: 65532
        runAsGroup: 65532
        # Secret files are owned by root; fsGroup makes them group-readable by
        # the container's user. Without it, mode 0440 means "permission denied"
        # on a file the pod cannot open — and AlertView refuses to start.
        fsGroup: 65532
      volumes:
        - name: config
          secret:
            secretName: alertview-config
            defaultMode: 0440
```

The container, the mount path and the arguments are unchanged — AlertView only
ever sees a file at `/config/config.yaml`. Auto-reload keeps working: Secret
updates reach the pod the same way ConfigMap ones do (see the note under
[Using ConfigMap](#using-configmap)), and `config_watch_method: "polling"`
picks up the symlink swap that inotify misses.

The `02-configmap.yaml` shipped with this repository is an **example**: it
carries `YOUR_GRAFANA_TOKEN_HERE`-style placeholders. As soon as you replace
them with real tokens, move the file to a Secret as above — a ConfigMap is
readable by anything that can list ConfigMaps in the namespace, and it is
usually committed to git.

For encrypted-at-rest workflows, a `SealedSecret`, an External Secrets
`ExternalSecret` or a SOPS-encrypted file all produce the same Secret and need
no change on the AlertView side.

### Rolling Updates

AlertView handles `SIGTERM`: it stops accepting new connections, lets the
requests already in flight finish, closes the SSE streams and exits — usually
in a few milliseconds. The default `terminationGracePeriodSeconds: 30` is
ample; no `preStop` hook is needed.

Browsers reconnect their event stream on their own, and a dashboard that
cannot reach the server in the meantime says so rather than showing stale
alerts as if they were live (see [Features](../getting-started/features.md)).

### Monitoring

**AlertView exposes no Prometheus metrics** — there is no `/metrics` endpoint,
and `prometheus.io/scrape` annotations on the pod will only produce a failing
target. What it does expose is `/health`, which returns `200 OK` as soon as the
HTTP server is up:

```bash
curl -s http://alertview.alertview.svc.cluster.local:8080/health   # OK
```

That is what the readiness and liveness probes in `03-deployment.yaml` use. To
alert on the dashboard itself being down, probe it from the outside — with
`blackbox_exporter`, for instance:

```yaml
# Prometheus scrape config (on the blackbox exporter, not on AlertView)
- job_name: alertview-health
  metrics_path: /probe
  params:
    module: [http_2xx]
  static_configs:
    - targets: ["http://alertview.alertview.svc.cluster.local:8080/health"]
```

`/health` reflects the HTTP server, not the state of the sources: a source
that is failing keeps the endpoint at `200`, is reported per source in
`/api/alerts` and is shown next to its name in the dashboard.

### Logging

Configure JSON logging for better integration with log aggregation systems:

```yaml
env:
- name: ALERTVIEW_LOG_FORMAT
  value: "json"
- name: RUST_LOG
  value: "info"
```

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -n alertview
kubectl describe pod alertview-xxxx -n alertview
```

### View Logs

```bash
kubectl logs deployment/alertview -n alertview
kubectl logs deployment/alertview -n alertview --previous  # Previous instance
```

### Test Connectivity

```bash
# Test if the pod is running
kubectl exec -it deployment/alertview -n alertview -- curl -I localhost:8080/health

# Test DNS resolution
kubectl exec -it deployment/alertview -n alertview -- nslookup alertmanager.your-namespace
```

### Common Issues

#### ConfigMap Not Mounted

Ensure the ConfigMap exists and the volume mount is correct:

```bash
kubectl get configmap -n alertview
kubectl describe pod alertview-xxxx -n alertview | grep -A 10 Volumes
```

#### Configuration Errors

AlertView will log configuration errors on startup. Check the logs:

```bash
kubectl logs deployment/alertview -n alertview | grep -i error
```

#### Connection to Datasources

Verify that AlertView can reach your datasources:

```bash
# Test from within the pod
kubectl exec -it deployment/alertview -n alertview -- curl -v http://alertmanager:9093/api/v2/alerts
```

#### Ingress Not Working

Check the ingress controller logs:

```bash
kubectl get ingress -n alertview
kubectl describe ingress alertview -n alertview
kubectl logs -n ingress-nginx deployment/ingress-nginx-controller
```

### Debug Mode

Enable debug logging for troubleshooting:

```yaml
env:
- name: RUST_LOG
  value: "debug"
```

Then restart the deployment:

```bash
kubectl rollout restart deployment/alertview -n alertview
```

## Upgrading

### Using kubectl

1. Update the image in your deployment:

```yaml
image: ghcr.io/your-org/alertview:v1.2.0
```

2. Apply the changes:

```bash
kubectl apply -f 01-namespace.yaml -f 02-configmap.yaml -f 03-deployment.yaml -f 04-service.yaml -f 05-ingress.yaml -n alertview
```

### Rolling Updates

Kubernetes will automatically perform a rolling update. To monitor:

```bash
kubectl rollout status deployment/alertview -n alertview
```

## Customizing

### Custom Configuration File Path

By default, AlertView looks for configuration at `/etc/alertview/config.yaml`. You can override this:

```yaml
containers:
- name: alertview
  args:
  - --config
  - /custom/path/config.yaml
  volumeMounts:
  - name: custom-config
    mountPath: /custom/path
```

### Multiple Instances

Run multiple AlertView instances with different configurations:

```yaml
# deployment-staging.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: alertview-staging
spec:
  template:
    spec:
      containers:
      - name: alertview
        env:
        - name: ALERTVIEW_CONFIG_PATH
          value: /etc/alertview/config-staging.yaml
        volumeMounts:
        - name: config
          mountPath: /etc/alertview/config-staging.yaml
          subPath: config-staging.yaml
```

## Uninstalling

```bash
kubectl delete -f 01-namespace.yaml -f 02-configmap.yaml -f 03-deployment.yaml -f 04-service.yaml -f 05-ingress.yaml -n alertview
kubectl delete configmap alertview-config -n alertview
kubectl delete namespace alertview
```

## Additional Resources

- [Kubernetes Documentation](https://kubernetes.io/docs/home/)
- [microk8s Documentation](https://microk8s.io/docs)
- [AlertView Configuration](../configuration/config-file.md)
