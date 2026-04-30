# Configuration Guide

This section covers all aspects of AlertView configuration.

## Quick Links

- [Configuration File](./config-file.md) - Main configuration reference
- [Environment Variables](./environment-variables.md) - All supported environment variables
- [Source Types](./source-types.md) - Detailed guide for each source type
- [Display Options](./display-options.md) - Customize the UI
- [Advanced Configuration](./advanced.md) - Caching, retries, timeouts, etc.

## Configuration Hierarchy

AlertView uses a hierarchical configuration system with the following priority (highest to lowest):

1. **Environment Variables** - Override any config file setting
2. **Config File** - Main YAML configuration
3. **Defaults** - Built-in default values

## Configuration Files

### Locations

AlertView looks for configuration in these locations (in order):

1. Command-line argument: `alertview /path/to/config.yaml`
2. Default: `config.yaml` in current directory

### Format

All configuration files use YAML format. Example:

```yaml
# Server settings
port: 8080
refresh_interval: 30

# Sources
sources:
  - name: "Alertmanager"
    type: alertmanager
    url: "http://localhost:9093"

# Display settings
display:
  labels:
    - namespace
    - job
  theme: "dark"
  timezone: "local"
  play_sounds: false

# Advanced settings
cache_ttl_seconds: 60
log_format: "text"
tls_insecure: false
```

## Best Practices

### 1. Security

- **Never commit credentials**: Use `.gitignore` to exclude config files
- **Use environment variables**: For sensitive data like tokens and passwords
- **Restrict file permissions**: `chmod 600 config.yaml`
- **Use separate config files**: One for each environment (dev, staging, prod)

### 2. Organization

- **Group related sources**: Use meaningful names for sources
- **Use comments**: Document why each source is configured
- **Consistent formatting**: Use consistent indentation and structure

### 3. Performance

- **Set appropriate timeouts**: Match source response times
- **Configure retries**: Balance between reliability and load
- **Enable caching**: For sources that don't change frequently
- **Limit refresh interval**: Don't poll too frequently

### 4. Maintainability

- **Version control**: Track changes to configuration
- **Document changes**: Keep a changelog of config modifications
- **Test changes**: Verify new config works before deploying

## Common Patterns

### Multiple Environments

```
config/
├── base.yaml          # Shared configuration
├── dev.yaml           # Development overrides
├── staging.yaml       # Staging overrides
└── prod.yaml          # Production overrides
```

Use a tool like `yq` to merge configurations:

```bash
# For development
yq eval-all 'select(fileIndex == 0) * select(fileIndex == 1)' config/base.yaml config/dev.yaml > config.yaml

# For production
yq eval-all 'select(fileIndex == 0) * select(fileIndex == 1)' config/base.yaml config/prod.yaml > config.yaml
```

### Secrets Management

**Option 1: Environment Variables**

```yaml
sources:
  - name: "Grafana"
    type: grafana
    url: "http://grafana:3000"
    bearer_token: "${GRAFANA_TOKEN}"  # Set via env var
```

**Option 2: External Secrets**

Use tools like:
- HashiCorp Vault
- AWS Secrets Manager
- Kubernetes Secrets
- .env files (for development)

**Option 3: Config File Permissions**

```bash
# Create config with restricted permissions
umask 077
touch config.yaml
# Only owner can read/write
chmod 600 config.yaml
```

## Troubleshooting Configuration

### Common Issues

1. **Invalid YAML**
   ```
   Error: Cannot parse config: YAML parse error
   Solution: Validate your YAML syntax using a YAML validator
   ```

2. **Missing Required Fields**
   ```
   Error: Missing field 'url' for source 'my-source'
   Solution: Ensure all required fields are present
   ```

3. **Invalid Source Type**
   ```
   Error: Unknown source type 'prometheus'
   Solution: Use one of: alertmanager, grafana, zabbix
   ```

4. **Connection Errors**
   ```
   Error: Failed to fetch from my-source: Connection refused
   Solution: Verify the URL is correct and the service is running
   ```

5. **Authentication Errors**
   ```
   Error: Failed to fetch from my-source: HTTP 401
   Solution: Verify your authentication credentials
   ```

### Debugging Tips

1. **Enable debug logging**:
   ```bash
   RUST_LOG=debug cargo run -- config.yaml
   ```

2. **Validate YAML**:
   ```bash
   yamllint config.yaml
   # or
   python3 -c 'import yaml, sys; yaml.safe_load(open(sys.argv[1]))' config.yaml
   ```

3. **Check file permissions**:
   ```bash
   ls -la config.yaml
   ```

4. **Test connectivity**:
   ```bash
   curl -v http://your-alertmanager:9093/api/v2/alerts
   ```
