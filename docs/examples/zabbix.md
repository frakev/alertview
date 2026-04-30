# Zabbix Configuration

This guide covers configuring AlertView to work with Zabbix.

## Basic Configuration

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password

port: 8080
```

## Complete Configuration

```yaml
sources:
  - name: zabbix-production
    kind: zabbix
    url: https://zabbix.prod.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    
    # Connection settings
    timeout: 30
    
    # Retry settings
    retry_policy:
      max_retries: 3
      initial_delay_ms: 1000
      max_delay_ms: 10000
    
    # Caching
    cache_ttl: 60
    
    # Link template for alert URLs
    link_template: "https://zabbix.example.com/monitoring.php?triggerid={{.TriggerID}}"
    
    # TLS settings (optional)
    tls:
      skip_verify: false

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC

port: 8080
log_format: json
```

## Authentication

Zabbix uses username/password authentication for its API.

### Configuration

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
```

### Environment Variables

```bash
export ZABBIX_USERNAME=api-user
export ZABBIX_PASSWORD=api-password
```

## Zabbix API

AlertView uses the Zabbix JSON-RPC API to fetch triggers (alerts).

**API endpoint:** `/api_jsonrpc.php`

**Methods used:**
- `user.login` - Authenticate
- `trigger.get` - Get triggers (alerts)
- `user.logout` - Logout

## Link Templates

Customize the URL that opens when clicking a Zabbix alert:

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    link_template: "https://zabbix.example.com/monitoring.php?triggerid={{.TriggerID}}&hostid={{.HostID}}"
```

**Available template variables for Zabbix alerts:**
- `{{.TriggerID}}` - Trigger ID
- `{{.TriggerName}}` - Trigger name
- `{{.HostID}}` - Host ID
- `{{.HostName}}` - Host name
- `{{.ItemID}}` - Item ID
- `{{.ItemName}}` - Item name
- `{{.Severity}}` - Severity level (0-5)
- `{{.Status}}` - Trigger status
- `{{.LastChange}}` - Last change timestamp

**Examples:**

```yaml
# Simple trigger link
link_template: "https://zabbix.example.com/triggers.php?triggerid={{.TriggerID}}"

# Host monitoring link
link_template: "https://zabbix.example.com/monitoring.php?hostid={{.HostID}}"

# Latest data for host
link_template: "https://zabbix.example.com/latest.php?hostid={{.HostID}}"

# With time range
link_template: "https://zabbix.example.com/monitoring.php?triggerid={{.TriggerID}}&from=now-1h&to=now"
```

## Severity Mapping

Zabbix uses numeric severity levels (0-5):

| Zabbix Level | AlertView Severity | Color (default) |
|--------------|-------------------|----------------|
| 0 (Not classified) | info | Blue |
| 1 (Information) | info | Blue |
| 2 (Warning) | warning | Yellow |
| 3 (Average) | warning | Yellow |
| 4 (High) | critical | Orange |
| 5 (Disaster) | critical | Red |

## Filtering

Filter Zabbix triggers by various criteria:

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    
    # Only active triggers
    only_active: true
    
    # Minimum severity (0-5)
    min_severity: 2  # Warning and above
    
    # Filter by host group
    host_group: "Linux servers"
    
    # Filter by host
    host: "web-server-01"
```

## Multiple Zabbix Sources

Monitor multiple Zabbix instances:

```yaml
sources:
  - name: zabbix-production
    kind: zabbix
    url: https://zabbix.prod.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    timeout: 30
    
  - name: zabbix-staging
    kind: zabbix
    url: https://zabbix.staging.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    timeout: 15

display:
  refresh_interval: 30

port: 8080
```

## Best Practices

### Production Configuration

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.prod.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    timeout: 30
    retry_policy:
      max_retries: 5
      initial_delay_ms: 2000
      max_delay_ms: 30000
    cache_ttl: 60
    link_template: "https://zabbix.prod.example.com/monitoring.php?triggerid={{.TriggerID}}"
    only_active: true
    min_severity: 2

display:
  refresh_interval: 30
  theme: dark
  timezone: UTC

port: 8080
log_format: json
```

### Development Configuration

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: http://localhost:8080/api_jsonrpc.php
    username: Admin
    password: zabbix
    timeout: 10
    retry_policy:
      max_retries: 2
      initial_delay_ms: 500
      max_delay_ms: 2000
    cache_ttl: 0

display:
  refresh_interval: 10
  theme: light
  timezone: local

port: 8080
log_format: text
```

## Troubleshooting

### Connection Issues

**Symptom:** AlertView can't connect to Zabbix

**Checks:**
1. Verify Zabbix server is running
2. Check network connectivity
3. Verify API URL is correct
4. Check if API is enabled in Zabbix

**Debug:**

```bash
# Enable debug logging
RUST_LOG=debug alertview --config config.yaml

# Test connectivity manually
curl -v -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"user.login","params":{"user":"api-user","password":"api-password"},"id":1}' \
  https://zabbix.example.com/api_jsonrpc.php
```

### Authentication Issues

**Symptom:** Login failed or access denied

**Checks:**
1. Verify username and password
2. Check if user has API access
3. Verify user has permissions to view triggers

**Solution:**

In Zabbix:
1. Go to Administration > Users
2. Select the user
3. Ensure "Type" is "Zabbix user" or "Zabbix admin"
4. Check "Password" is correct
5. Go to Permissions tab
6. Ensure user has read access to the required hosts/host groups

### API Issues

**Symptom:** Unexpected API responses

**Checks:**
1. Verify Zabbix version
2. Check API version compatibility
3. Test API manually

**Zabbix API versions:**
- Zabbix 5.0+: JSON-RPC API v2
- Zabbix 4.0-4.4: JSON-RPC API v1
- Zabbix 3.0-3.4: JSON-RPC API v1

### Timeout Issues

**Symptom:** Requests timeout

**Solutions:**
1. Increase timeout value
2. Check Zabbix server performance
3. Reduce the number of triggers being fetched

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://slow-zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    timeout: 120
```

## Performance Considerations

### Reducing Load on Zabbix

1. **Enable caching**: Set `cache_ttl` to a reasonable value
2. **Increase refresh interval**: Reduce how often AlertView fetches triggers
3. **Filter triggers**: Only fetch active triggers with minimum severity

```yaml
sources:
  - name: zabbix
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    only_active: true
    min_severity: 2
    cache_ttl: 60

display:
  refresh_interval: 60
```

### Handling Large Numbers of Triggers

1. **Filter by host group**: Only fetch triggers for specific host groups
2. **Increase timeouts**: Give AlertView more time to process large responses
3. **Use multiple sources**: Split across multiple Zabbix sources

```yaml
sources:
  - name: zabbix-web
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    host_group: "Web servers"
    timeout: 60
    
  - name: zabbix-db
    kind: zabbix
    url: https://zabbix.example.com/api_jsonrpc.php
    username: api-user
    password: api-password
    host_group: "Database servers"
    timeout: 60
```

## Additional Resources

- [Zabbix Documentation](https://www.zabbix.com/documentation)
- [Zabbix API Documentation](https://www.zabbix.com/documentation/current/manual/api)
- [Zabbix JSON-RPC API](https://www.zabbix.com/documentation/current/manual/api/reference)
- [AlertView Configuration Reference](../configuration/config-file.md)
