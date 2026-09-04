# Deployment Guide

This section covers various ways to deploy AlertView in different environments.

## Quick Links

- [Local Development](./local.md) - Running on your local machine
- [Docker](./docker.md) - Containerized deployment
- [Docker Compose](./docker-compose.md) - Multi-container deployment
- [Kubernetes](./kubernetes.md) - Kubernetes deployment
- [Reverse Proxy](./reverse-proxy.md) - Proxying AlertView behind Nginx/Apache

## Choosing a Deployment Method

| Method | Best For | Complexity | Maintenance |
|--------|----------|------------|-------------|
| Local | Development, Testing | ⭐ | ⭐ |
| Docker | Single server, Production | ⭐⭐ | ⭐⭐ |
| Docker Compose | Multi-container, Development | ⭐⭐ | ⭐⭐ |
| Kubernetes | Production, Scalability | ⭐⭐⭐ | ⭐⭐⭐ |

## Common Deployment Scenarios

### Scenario 1: Quick Test
**Goal**: Try AlertView quickly
**Method**: Local development
**Command**: `cargo run -- config.yaml`

### Scenario 2: Single Server
**Goal**: Deploy to a single server
**Method**: Docker
**Command**: `docker run -p 8080:8080 -v config.yaml:/config/config.yaml:ro alertview`

### Scenario 3: Team Dashboard
**Goal**: Team-wide alert dashboard
**Method**: Docker Compose with reverse proxy
**Files**: `docker-compose.yml`, `nginx.conf`

### Scenario 4: Production at Scale
**Goal**: High-availability production deployment
**Method**: Kubernetes
**Files**: Kubernetes manifests

### Scenario 5: NOC Wall Display
**Goal**: Large screen display for operations center
**Method**: Docker with TV mode enabled
**Config**: `display: { theme: "dark", timezone: "UTC" }`

## Deployment Checklist

- [ ] Configuration file created and tested
- [ ] All source URLs are correct and accessible
- [ ] Authentication tokens/credentials are configured
- [ ] TLS certificates are valid (or `tls_insecure` is set for dev)
- [ ] Port is not blocked by firewall
- [ ] Logs are configured appropriately
- [ ] Monitoring is set up for AlertView itself
- [ ] Backup of configuration file exists
- [ ] Rollback plan is in place

## Post-Deployment Tasks

1. **Verify Health**: Check `/health` endpoint
2. **Test Alerts**: Verify alerts are appearing
3. **Check Logs**: Monitor for errors
4. **Configure Monitoring**: Set up monitoring for AlertView
5. **Set Up Alerts**: Configure alerts for AlertView itself
6. **Document**: Document the deployment for your team

## Troubleshooting Deployments

### Common Issues

**Port already in use:**
```
Error: Address already in use (os error 98)
Solution: Change port or stop the existing service
```

**Permission denied:**
```
Error: Permission denied (os error 13)
Solution: Check file permissions, use sudo if needed
```

**Connection refused:**
```
Error: Failed to fetch from Alertmanager: Connection refused
Solution: Verify the source is running and accessible
```

**Configuration error:**
```
Error: Cannot parse config: YAML parse error
Solution: Validate your YAML syntax
```

### Debugging Tips

1. **Check logs**: Look at the container or application logs
2. **Test connectivity**: Use `curl` to test source URLs
3. **Verify config**: Test with a minimal configuration first
4. **Increase logging**: Use `RUST_LOG=debug` for more details
5. **Check network**: Verify network connectivity to sources

## Upgrading AlertView

### From Docker

```bash
# Pull the latest image
docker pull ghcr.io/frakev/alertview:latest

# Restart the container
docker restart alertview
```

### From Source

```bash
# Pull the latest code
git pull origin main

# Rebuild
cargo build --release

# Restart
systemctl restart alertview  # or your restart command
```

### From Kubernetes

```bash
# Update the image in your deployment
kubectl set image deployment/alertview alertview=ghcr.io/frakev/alertview:latest -n alertview

# Or apply updated manifests
kubectl apply -f 03-deployment.yaml
```

## Rollback

### Docker

```bash
# Rollback to previous version
docker run -p 8080:8080 ghcr.io/frakev/alertview:v1.0.0
```

### Kubernetes

```bash
# Rollback to previous deployment
kubectl rollout undo deployment/alertview -n alertview
```

### Git

```bash
# Rollback to previous commit
git checkout v1.0.0
cargo build --release
```

## Performance Considerations

### Resource Requirements

| Deployment Size | CPU | Memory | Notes |
|-----------------|-----|--------|-------|
| Small (1-10 sources) | 0.1 CPU | 64MB | Default settings |
| Medium (10-50 sources) | 0.5 CPU | 128MB | Increase timeouts |
| Large (50+ sources) | 1+ CPU | 256MB+ | Enable caching |

### Scaling

AlertView is designed to be lightweight:
- **Stateless**: Can run multiple instances behind a load balancer
- **No Database**: No external dependencies
- **Low Overhead**: Minimal CPU and memory usage

For high-traffic deployments:
- Use multiple replicas
- Enable caching
- Consider load balancing

## Security Checklist

- [ ] Use HTTPS in production
- [ ] Restrict access to AlertView
- [ ] Use strong authentication for sources
- [ ] Rotate tokens regularly
- [ ] Restrict file permissions
- [ ] Enable audit logging
- [ ] Set up monitoring
- [ ] Configure alerts for AlertView

## Next Steps

After deployment:
1. [Configure sources](../configuration/source-types.md)
2. [Customize display](../configuration/display-options.md)
3. [Set up monitoring](#monitoring-alertview)
4. [Train your team](https://github.com/frakev/alertview#readme)

## Security Considerations

### TLS Certificate Verification

By default, AlertView verifies TLS certificates when connecting to alert sources. If you need to disable this (e.g., for self-signed certificates in development):

```yaml
# In config.yaml
tls_insecure: true
```

**Warning**: This will log a warning message and is not recommended for production environments.

### Rate Limiting and Resource Limits

AlertView includes several built-in limits to prevent resource exhaustion:

- **Cache Size**: Maximum 1000 cached entries. When exceeded, oldest entries are removed.
- **SSE Connections**: Maximum 100 concurrent Server-Sent Events connections. Additional connections receive HTTP 429 (Too Many Requests).
- **Configuration Validation**: Port, timeout, and retry settings are validated on startup.

### Configuration Validation

AlertView validates configuration on startup:

- Port cannot be 0
- `refresh_interval` cannot be 0
- Source URLs cannot be empty
- Timeout cannot be 0
- Retry policy delays must be valid (initial_delay_ms > 0, max_delay_ms >= initial_delay_ms)

If validation fails, AlertView will refuse to start with an error message indicating the problem.

### Network Security

- AlertView only makes outbound HTTP/HTTPS requests to configured sources
- No inbound connections are made except for the web interface and SSE endpoint
- Consider using a reverse proxy with authentication for production deployments
- Use HTTPS for all external connections

### Authentication

**Note**: AlertView does not currently support authentication for its web interface. If you need authentication:

1. Use a reverse proxy (Nginx, Apache) with basic auth or OAuth
2. Deploy AlertView on an internal network only
3. Use network-level firewalls to restrict access

See the [Reverse Proxy](./reverse-proxy.md) documentation for examples.
