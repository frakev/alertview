# Docker Deployment

Deploy AlertView using Docker containers.

## Quick Start

```bash
# Run with a configuration file
docker run -p 8080:8080 -v $(pwd)/config.yaml:/config/config.yaml:rw ghcr.io/frakev/alertview:latest

# Access the dashboard at http://localhost:8080
```

## Docker Images

### Official Images

AlertView images are available on GitHub Container Registry (GHCR):

- **Latest**: `ghcr.io/frakev/alertview:latest` - Latest commit to main branch
- **Versioned**: `ghcr.io/frakev/alertview:v1.0.0` - Specific version
- **Main branch**: `ghcr.io/frakev/alertview:main` - Latest main branch

### Image Tags

| Tag | Description |
|-----|-------------|
| `latest` | Latest stable release |
| `vX.Y.Z` | Specific version |
| `main` | Latest main branch (may be unstable) |
| `SHA` | Specific commit SHA |

### Image Size

```bash
# Check image size
docker images ghcr.io/frakev/alertview

# Typically ~20-30MB (compressed)
```

## Basic Deployment

### 1. Create Configuration

```bash
# Create config.yaml
cat > config.yaml <<EOF
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://alertmanager:9093"
EOF
```

### 2. Run Container

```bash
docker run -d \
  --name alertview \
  -p 8080:8080 \
  -v $(pwd)/config.yaml:/config/config.yaml:rw \
  ghcr.io/frakev/alertview:latest
```

### 3. Verify

```bash
# Check container is running
docker ps | grep alertview

# Check logs
docker logs alertview

# Test health endpoint
curl http://localhost:8080/health
```

## Configuration Options

### Mount Configuration File

```bash
-v $(pwd)/config.yaml:/config/config.yaml:rw
```

**Important**: Use `:rw` (read-write) to enable config auto-reload. With `:ro` (read-only), config changes won't be detected.

### Environment Variables

```bash
docker run -d \
  -p 8080:8080 \
  -e ALERTVIEW_PORT=8080 \
  -e ALERTVIEW_REFRESH_INTERVAL=60 \
  -e ALERTVIEW_LOG_FORMAT=json \
  -v $(pwd)/config.yaml:/config/config.yaml:rw \
  ghcr.io/frakev/alertview:latest
```

### Custom Port

```bash
# Map host port 9090 to container port 8080
docker run -d -p 9090:8080 -v config.yaml:/config/config.yaml:rw alertview

# Access at http://localhost:9090
```

### Multiple Config Files

```bash
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/config1.yaml:/config/config1.yaml:rw \
  -v $(pwd)/config2.yaml:/config/config2.yaml:rw \
  ghcr.io/frakev/alertview:latest \
  /config/config1.yaml
```

## Docker Compose

See [Docker Compose](./docker-compose.md) for multi-container deployments.

## Dockerfile Reference

The official Dockerfile:

```dockerfile
FROM rust:1-slim AS builder

WORKDIR /usr/src/alertview
COPY . .
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/alertview/target/release/alertview /usr/local/bin/alertview
COPY static /static

EXPOSE 8080
ENTRYPOINT ["alertview"]
CMD ["config.yaml"]
```

## Building Custom Image

### 1. Clone Repository

```bash
git clone https://github.com/frakev/alertview.git
cd alertview
```

### 2. Build Image

```bash
docker build -t alertview:custom .
```

### 3. Run Custom Image

```bash
docker run -p 8080:8080 -v config.yaml:/config/config.yaml:rw alertview:custom
```

### 4. Push to Registry

```bash
# Login to registry
docker login ghcr.io

# Tag and push
docker tag alertview:custom ghcr.io/your-username/alertview:custom
docker push ghcr.io/your-username/alertview:custom
```

## Multi-Architecture Builds

### Using Buildx

```bash
# Enable buildx
docker buildx create --use

# Build for multiple architectures
docker buildx build --platform linux/amd64,linux/arm64 -t alertview:multiarch --push .
```

### Cross-Compilation

```bash
# Build for ARM64 on x86_64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu

# Build Docker image for ARM64
FROM rust:1-slim AS builder
WORKDIR /usr/src/alertview
COPY . .
RUN apt-get update && apt-get install -y gcc-aarch64-linux-gnu
RUN rustup target add aarch64-unknown-linux-gnu
RUN cargo build --release --target aarch64-unknown-linux-gnu

FROM debian:stable-slim
COPY --from=builder /usr/src/alertview/target/aarch64-unknown-linux-gnu/release/alertview /usr/local/bin/
EXPOSE 8080
ENTRYPOINT ["alertview"]
```

## Security

### Run as Non-Root

```dockerfile
FROM ghcr.io/frakev/alertview:latest
USER 1000:1000
EXPOSE 8080
```

Run with:
```bash
docker run -d --user 1000:1000 -p 8080:8080 alertview
```

### Read-Only Filesystem

```bash
docker run -d --read-only -p 8080:8080 alertview
```

**Note**: This disables config auto-reload. Use environment variables instead.

### Seccomp Profile

```bash
docker run -d --security-opt seccomp=unconfined -p 8080:8080 alertview
```

### No New Privileges

```bash
docker run -d --security-opt no-new-privileges -p 8080:8080 alertview
```

## Networking

### Expose Multiple Ports

```bash
# Expose 8080 and 8443
docker run -d -p 8080:8080 -p 8443:8443 alertview
```

### Use Host Network

```bash
# Use host network (not recommended for production)
docker run -d --network host alertview
```

### Custom Network

```bash
# Create custom network
docker network create alertview-net

# Run container on custom network
docker run -d --network alertview-net --name alertview alertview

# Run other containers on same network
docker run -d --network alertview-net --name alertmanager prom/alertmanager
```

## Volumes

### Persistent Configuration

```bash
# Create named volume
docker volume create alertview-config

# Run with named volume
docker run -d -p 8080:8080 -v alertview-config:/config alertview /config/config.yaml
```

### Multiple Configuration Files

```bash
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/configs:/configs:rw \
  alertview /configs/production.yaml
```

## Logging

### View Logs

```bash
# View logs
docker logs alertview

# Follow logs
docker logs -f alertview

# View last N lines
docker logs --tail 100 alertview

# View logs with timestamps
docker logs -t alertview
```

### Log to File

```bash
# Create log file
touch alertview.log

# Run with log file
docker run -d \
  -p 8080:8080 \
  -v $(pwd)/alertview.log:/var/log/alertview.log \
  alertview 2>&1 | tee /var/log/alertview.log
```

### JSON Logs

```bash
docker run -d \
  -e ALERTVIEW_LOG_FORMAT=json \
  -p 8080:8080 \
  alertview
```

## Monitoring

### Health Check

```bash
# Test health endpoint
curl http://localhost:8080/health

# In Docker
curl http://$(docker inspect -f '{{.NetworkSettings.IPAddress}}' alertview):8080/health
```

### Docker Healthcheck

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1
```

## Upgrading

### Pull Latest Image

```bash
# Pull latest image
docker pull ghcr.io/frakev/alertview:latest

# Stop and remove old container
docker stop alertview
docker rm alertview

# Run new container
docker run -d -p 8080:8080 -v config.yaml:/config/config.yaml:rw ghcr.io/frakev/alertview:latest
```

### Rollback

```bash
# Rollback to previous version
docker run -d -p 8080:8080 -v config.yaml:/config/config.yaml:rw ghcr.io/frakev/alertview:v1.0.0
```

## Troubleshooting

### Common Issues

**Port already in use:**
```
Error: Address already in use
Solution: docker ps | grep 8080; docker stop <container>
```

**Configuration file not found:**
```
Error: Cannot read config.yaml
Solution: Verify volume mount: -v $(pwd)/config.yaml:/config/config.yaml
```

**Permission denied:**
```
Error: Permission denied
Solution: chmod 644 config.yaml
```

**Container exits immediately:**
```
Solution: docker logs alertview to see error
```

### Debug Mode

```bash
# Run in foreground with debug logging
docker run -it --rm \
  -e RUST_LOG=debug \
  -p 8080:8080 \
  -v config.yaml:/config/config.yaml:rw \
  alertview
```

## Best Practices

1. **Use `:rw` for config files** to enable auto-reload
2. **Use environment variables** for secrets and environment-specific settings
3. **Pin to specific versions** in production (not `latest`)
4. **Enable health checks** for monitoring
5. **Use custom networks** for multi-container deployments
6. **Limit resources** with `--memory` and `--cpus` flags
7. **Set restart policies** (`--restart unless-stopped`)
8. **Use named volumes** for persistent configuration
9. **Monitor logs** regularly
10. **Keep images updated** but test before deploying to production
