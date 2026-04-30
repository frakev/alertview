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

Run as non-root user:

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  fsGroup: 2000
```

#### Secrets Management

For sensitive data (e.g., API tokens for datasources):

```yaml
env:
- name: ALERTMANAGER_API_TOKEN
  valueFrom:
    secretKeyRef:
      name: alertview-secrets
      key: alertmanager-token
```

### Monitoring

Add Prometheus annotations for monitoring:

```yaml
metadata:
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "8080"
    prometheus.io/path: "/metrics"
```

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
