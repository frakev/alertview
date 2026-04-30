# Local Development

Run AlertView directly on your local machine for development and testing.

## Prerequisites

1. **Rust**: Rust 1.75+ is required
2. **Git**: For cloning the repository (optional)

### Install Rust

If you don't have Rust installed:

```bash
# Install rustup (Rust toolchain installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source the environment
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Update Rust

```bash
rustup update
```

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/frakev/alertview.git
cd alertview
```

### 2. Build AlertView

```bash
# Development build (faster, larger binary)
cargo build

# Release build (slower, optimized binary)
cargo build --release
```

### 3. Create Configuration

```bash
# Copy the example configuration
cp config.example config.yaml

# Edit the configuration
nano config.yaml  # or use your preferred editor
```

### 4. Run AlertView

```bash
# Run with development build
cargo run -- config.yaml

# Or run the release binary
./target/release/alertview config.yaml
```

### 5. Access the Dashboard

Open your browser and navigate to: `http://localhost:8080`

## Development Workflow

### Run in Development Mode

```bash
# Run with automatic rebuild on code changes
cargo watch -x run -- config.yaml
```

This requires `cargo-watch`:
```bash
cargo install cargo-watch
```

### Run with Custom Port

```bash
# Set port via environment variable
ALERTVIEW_PORT=3000 cargo run -- config.yaml

# Or in config.yaml
# port: 3000
```

### Run with Debug Logging

```bash
RUST_LOG=debug cargo run -- config.yaml
```

### Run with JSON Logs

```bash
ALERTVIEW_LOG_FORMAT=json cargo run -- config.yaml
# or in config.yaml
# log_format: json
```

## Configuration for Local Development

### Minimal Configuration

```yaml
# config.yaml
sources:
  - name: "Local Alertmanager"
    type: alertmanager
    url: "http://localhost:9093"
    tls_insecure: true  # For self-signed certs in dev
```

### Multiple Local Sources

```yaml
# config.yaml
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://localhost:9093"
    tls_insecure: true

  - name: "Grafana"
    type: grafana
    url: "http://localhost:3000"
    bearer_token: "your-dev-token"
    tls_insecure: true

  - name: "Zabbix"
    type: zabbix
    url: "http://localhost:8080/zabbix"
    bearer_token: "your-zabbix-token"
    tls_insecure: true

# Development-specific settings
refresh_interval: 10  # Faster refresh for testing
log_format: json      # JSON logs for debugging
```

## Using Local Alertmanager

### 1. Run Alertmanager Locally

```bash
# Download Alertmanager
docker pull prom/alertmanager

# Run with a simple config
cat > alertmanager.yml <<EOF
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname']
  group_wait: 10s
  group_interval: 5m
  repeat_interval: 3h
  receiver: 'webhook'

receivers:
- name: 'webhook'
  webhook_configs:
  - url: 'http://localhost:8080/webhook'
EOF

# Run Alertmanager
docker run -p 9093:9093 -v $(pwd)/alertmanager.yml:/etc/alertmanager/alertmanager.yml prom/alertmanager
```

### 2. Create Test Alerts

```bash
# Send a test alert to Alertmanager
curl -X POST -H "Content-Type: application/json" \
  -d '[{"labels":{"alertname":"TestAlert","severity":"critical"},"annotations":{"summary":"This is a test"}}]' \
  http://localhost:9093/api/v2/alerts
```

### 3. Configure AlertView

```yaml
sources:
  - name: "Local Alertmanager"
    type: alertmanager
    url: "http://localhost:9093"
```

### 4. Run AlertView

```bash
cargo run -- config.yaml
```

You should see the test alert appear in the dashboard.

## Using Local Grafana

### 1. Run Grafana Locally

```bash
# Run Grafana
docker run -p 3000:3000 grafana/grafana
```

Access Grafana at: `http://localhost:3000` (admin/admin)

### 2. Set Up Alerts in Grafana

1. Create a data source (e.g., Prometheus)
2. Create an alert rule
3. Note the alert name

### 3. Get API Token

1. Go to Configuration → Service Accounts
2. Create a service account
3. Assign the "Admin" role
4. Copy the token

### 4. Configure AlertView

```yaml
sources:
  - name: "Local Grafana"
    type: grafana
    url: "http://localhost:3000"
    bearer_token: "your-grafana-token"
```

### 5. Run AlertView

```bash
cargo run -- config.yaml
```

## Debugging Local Development

### Check AlertView Logs

```bash
# Run with debug logging
RUST_LOG=debug cargo run -- config.yaml 2>&1 | tee alertview.log
```

### Test API Directly

```bash
# Get alerts from API
curl http://localhost:8080/api/alerts | jq

# Check health
curl http://localhost:8080/health
```

### Test Source Connectivity

```bash
# Test Alertmanager
curl http://localhost:9093/api/v2/alerts | jq

# Test Grafana
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:3000/api/alertmanager/grafana/api/v2/alerts | jq
```

### Common Issues

**Port already in use:**
```
Error: Address already in use (os error 98)
Solution: Kill the existing process or use a different port
```

**Connection refused:**
```
Error: Failed to fetch from Alertmanager: Connection refused
Solution: Verify Alertmanager is running on the correct port
```

**TLS errors:**
```
Error: Failed to fetch from Alertmanager: SSL error
Solution: Set tls_insecure: true for development
```

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_source_defaults

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

## Building for Production

```bash
# Build release binary
cargo build --release

# The binary will be at target/release/alertview
ls -lh target/release/alertview

# Run the release binary
./target/release/alertview config.yaml
```

## Cross-Compilation

### Build for Linux from macOS

```bash
# Install cross-compilation target
rustup target add x86_64-unknown-linux-gnu

# Build for Linux
cargo build --release --target x86_64-unknown-linux-gnu

# The binary will be at target/x86_64-unknown-linux-gnu/release/alertview
```

### Build for ARM

```bash
# Install ARM target
rustup target add arm-unknown-linux-gnueabihf

# Build for ARM
cargo build --release --target arm-unknown-linux-gnueabihf
```

## Cleaning Up

```bash
# Clean build artifacts
cargo clean

# Remove target directory
rm -rf target

# Remove Cargo.lock (to update dependencies)
rm Cargo.lock
```

## Tips for Local Development

1. **Use `cargo watch`** for automatic rebuilds on code changes
2. **Enable debug logging** with `RUST_LOG=debug`
3. **Use JSON logs** for easier parsing: `ALERTVIEW_LOG_FORMAT=json`
4. **Set short refresh intervals** (10-15 seconds) for faster testing
5. **Disable caching** (`cache_ttl_seconds: 0`) for real-time testing
6. **Use `tls_insecure: true`** for development with self-signed certs
7. **Test with local sources** (Alertmanager, Grafana, etc.)
8. **Check the API** at `/api/alerts` for raw data
