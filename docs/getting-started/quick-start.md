# Quick Start

Get AlertView up and running in just a few minutes!

## Option 1: Using Docker (Recommended)

### 1. Create a configuration file

Create a file named `config.yaml` with your alert sources:

```yaml
sources:
  - name: "My Alertmanager"
    type: alertmanager
    url: "http://your-alertmanager:9093"
```

Replace the URL with your actual Alertmanager endpoint.

### 2. Run with Docker

```bash
docker run -p 8080:8080 -v $(pwd)/config.yaml:/config/config.yaml:ro ghcr.io/frakev/alertview:latest
```

### 3. Access the dashboard

Open your browser and navigate to: `http://localhost:8080`

---

## Option 2: From Source

### 1. Install Rust

If you don't have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Clone and build

```bash
git clone https://github.com/frakev/alertview.git
cd alertview
cargo build --release
```

### 3. Create a configuration file

Copy the example configuration:

```bash
cp config.example config.yaml
```

Edit `config.yaml` with your alert sources.

### 4. Run AlertView

```bash
./target/release/alertview config.yaml
```

### 5. Access the dashboard

Open your browser and navigate to: `http://localhost:8080`

---

## Verify It's Working

1. Check the logs for any errors
2. You should see your configured sources listed
3. Alerts should appear automatically (if any exist)

## Next Steps

- [Configure additional sources](../configuration/source-types.md)
- [Customize the display](../configuration/display-options.md)
- [Deploy to production](../deployment/README.md)
- [Explore advanced features](../configuration/advanced.md)

## Troubleshooting

If you encounter issues:

1. **No alerts appearing?**
   - Verify your source URLs are correct
   - Check authentication tokens
   - Look for errors in the logs

2. **Connection errors?**
   - Ensure your alert sources are accessible from where AlertView is running
   - Check firewall rules
   - Try `tls_insecure: true` if using self-signed certificates

3. **Configuration errors?**
   - Validate your YAML syntax
   - Check required fields are present

For more help, see the [Troubleshooting](../troubleshooting.md) guide.
