# Troubleshooting Guide

This guide helps you diagnose and fix common issues with AlertView.

## General Troubleshooting

### Check AlertView Status

```bash
# Check if AlertView is running
ps aux | grep alertview

# Check the process
systemctl status alertview  # If using systemd

# Check logs
journalctl -u alertview -f  # Systemd logs
```

### Enable Debug Logging

```bash
# Set log level to debug
RUST_LOG=debug alertview --config config.yaml

# Set log level to trace (maximum verbosity)
RUST_LOG=trace alertview --config config.yaml

# Set log level for specific modules
RUST_LOG=debug,alertview::config=trace,alertview::alerts=debug alertview
```

### Check Configuration

```bash
# Validate YAML syntax
python3 -c "import yaml, sys; yaml.safe_load(open(sys.argv[1]))" config.yaml

# Or use yamllint
yamllint config.yaml

# Check if AlertView can load the config: it validates and reports on startup
alertview --config config.yaml
```

## Common Issues

### AlertView Won't Start

**Symptoms:**
- Process exits immediately
- No output or error messages
- "Permission denied" errors

**Diagnosis:**

1. **Check for errors:**
   ```bash
   alertview --config config.yaml 2>&1 | head -50
   ```

2. **Check configuration file:**
   ```bash
   # Verify the file exists
   ls -la config.yaml
   
   # Verify it's valid YAML
   python3 -c "import yaml; yaml.safe_load(open('config.yaml'))"
   ```

3. **Check file permissions:**
   ```bash
   ls -la config.yaml
   # Should be readable by the user running AlertView
   ```

**Solutions:**

- **Invalid YAML:** Fix syntax errors in your config file
- **Missing file:** Ensure the config file exists at the specified path
- **Permission denied:** Make the file readable: `chmod 644 config.yaml`
- **Invalid config:** Check for invalid configuration values

### AlertView Starts but No Alerts

**Symptoms:**
- AlertView runs without errors
- UI shows "No alerts" or empty list
- No alerts from any source

**Diagnosis:**

1. **Check source connectivity:**
   ```bash
   # Test Alertmanager
   curl -v http://localhost:9093/api/v2/alerts
   
   # Test Grafana
   curl -H "Authorization: Bearer your-api-key" \
     https://grafana.example.com/api/v1/alerts
   
   # Test Zabbix
   curl -v -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"user.login","params":{"user":"api-user","password":"api-password"},"id":1}' \
     https://zabbix.example.com/api_jsonrpc.php
   ```

2. **Check AlertView logs:**
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   ```

3. **Check source status via API:**
   ```bash
   curl http://localhost:8080/api/sources
   ```

**Solutions:**

- **Network issues:** Verify network connectivity to your monitoring systems
- **Authentication:** Check API keys, usernames, and passwords
- **URL errors:** Verify the URLs in your configuration
- **No alerts:** Your monitoring systems may not have any active alerts
- **Filtering:** Check if filters are excluding all alerts

### Alerts Not Updating

**Symptoms:**
- Alerts show initially but don't update
- Stale alert data
- "Last updated" timestamp doesn't change

**Diagnosis:**

1. **Check the refresh interval** (top level, not under `display:`):
   ```yaml
   refresh_interval: 30  # Must be > 0
   ```

2. **Check caching:**
   ```yaml
   cache_ttl_seconds: 60  # If too high, alerts may seem stale
   ```

3. **Check source status** — every source reports its own state in the
   dashboard payload:
   ```bash
   curl -s http://localhost:8080/api/alerts | jq '.sources'
   ```

4. **Check for errors in logs:**
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   ```

**Solutions:**

- **Increase refresh interval:** Set to a lower value (e.g., 10-30 seconds)
- **Disable caching:** Set `cache_ttl_seconds: 0` for real-time updates
- **Check source health:** Verify your monitoring systems are responding
- **Check for errors:** Look for fetch errors in logs

### "Backend unreachable" Banner

**Symptoms:**
- A red banner across the top: *Backend unreachable — data frozen since HH:MM*
- The alert list is dimmed, the browser tab reads `⚠ stale`
- The countdown keeps ticking, on a 15-second retry

**What it means:** the browser could not reach AlertView itself — not that a
source failed. The alerts still on screen are the last ones that arrived and
may be minutes old. A source that fails instead shows a red dot and `⚠ error`
in the sources bar, while the rest of the dashboard stays live.

**Diagnosis:**

1. **Is the server up?**
   ```bash
   curl -s http://localhost:8080/health   # expects: OK
   ```
2. **Is a reverse proxy in the way?** Check its logs for 502/504 on
   `/api/alerts`, and make sure it does not buffer or time out `/events`.
3. **Was it restarted?** A rolling update drains connections gracefully, and
   the banner clears by itself on the next successful poll.

The banner disappears on its own as soon as one poll succeeds; nothing has to
be reloaded by hand.

### Connection Errors

**Symptoms:**
- "Connection refused" errors
- "Connection timed out" errors
- "No route to host" errors

**Diagnosis:**

1. **Test connectivity manually:**
   ```bash
   # Test basic connectivity
   ping alertmanager.example.com
   
   # Test port connectivity
   nc -zv alertmanager.example.com 9093
   telnet alertmanager.example.com 9093
   
   # Test HTTP connectivity
   curl -v http://alertmanager.example.com:9093
   ```

2. **Check DNS resolution:**
   ```bash
   nslookup alertmanager.example.com
   dig alertmanager.example.com
   ```

3. **Check from AlertView container:**
   ```bash
   docker exec -it alertview ping alertmanager.example.com
   docker exec -it alertview curl -v http://alertmanager.example.com:9093
   ```

**Solutions:**

- **DNS issues:** Fix DNS resolution or use IP addresses
- **Network issues:** Check network connectivity, firewalls, security groups
- **Port issues:** Verify the correct port is open
- **Proxy issues:** Configure proxy if needed

### Authentication Errors

**Symptoms:**
- "401 Unauthorized" errors
- "403 Forbidden" errors
- "Authentication failed" errors

**Diagnosis:**

1. **Test authentication manually:**
   ```bash
   # Alertmanager basic auth
   curl -u username:password http://alertmanager.example.com:9093/api/v2/alerts
   
   # Grafana API key
   curl -H "Authorization: Bearer your-api-key" \
     https://grafana.example.com/api/v1/alerts
   
   # Zabbix
   curl -v -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"user.login","params":{"user":"api-user","password":"api-password"},"id":1}' \
     https://zabbix.example.com/api_jsonrpc.php
   ```

2. **Check credentials:**
   - Verify username and password
   - Verify API key is correct
   - Check if credentials have expired

3. **Check permissions:**
   - Verify the user/API key has read access to alerts
   - Check if IP restrictions are in place

**Solutions:**

- **Invalid credentials:** Update credentials in configuration
- **Expired credentials:** Generate new API keys or reset passwords
- **Insufficient permissions:** Grant read access to alerts
- **IP restrictions:** Add AlertView's IP to allowed list

### SSL/TLS Errors

**Symptoms:**
- "SSL certificate problem" errors
- "self signed certificate" errors
- "certificate verify failed" errors

**Diagnosis:**

1. **Test with curl:**
   ```bash
   curl -v https://alertmanager.example.com:9093
   ```

2. **Check certificate:**
   ```bash
   openssl s_client -connect alertmanager.example.com:9093 -showcerts
   ```

3. **Check certificate validity:**
   ```bash
   openssl x509 -in cert.pem -noout -dates
   ```

**Solutions:**

- **Self-signed certificate:** Set `tls_insecure: true` in the source configuration (not recommended for production)
- **Expired certificate:** Renew the certificate
- **Invalid hostname:** Use the correct hostname or configure SNI

Note: AlertView currently only supports `tls_insecure` to skip TLS verification. Custom CA certificates are not yet supported.

### Timeout Errors

**Symptoms:**
- "Request timeout" errors
- "Connection timed out" errors
- Slow responses

**Diagnosis:**

1. **Check response time:**
   ```bash
   time curl -v http://alertmanager.example.com:9093/api/v2/alerts
   ```

2. **Check AlertView timeout:**
   ```yaml
   sources:
     - name: alertmanager
       timeout: 15  # Default is 15 seconds
   ```

3. **Check monitoring system performance:**
   - Verify Alertmanager/Grafana/Zabbix is responding quickly
   - Check for high load on monitoring systems

**Solutions:**

- **Increase timeout:** Set a higher timeout value (e.g., 30-60 seconds)
- **Improve performance:** Optimize your monitoring systems
- **Reduce load:** Fetch fewer alerts or use filtering
- **Enable caching:** Use `cache_ttl` to reduce API calls

### Memory Issues

**Symptoms:**
- AlertView crashes with "Out of memory" errors
- High memory usage
- Slow performance

**Diagnosis:**

1. **Check memory usage:**
   ```bash
   # Check process memory
   ps aux | grep alertview
   
   # Check system memory
   free -h
   top
   ```

2. **Check for memory leaks:**
   ```bash
   # Monitor memory over time
   watch -n 5 'ps aux | grep alertview'
   ```

**Solutions:**

- **Reduce cache size:** Lower `cache_ttl` or disable caching
- **Reduce refresh interval:** Fetch alerts less frequently
- **Limit alerts:** Use filters to reduce the number of alerts
- **Increase memory:** Allocate more memory to AlertView
- **Restart AlertView:** Regularly restart to free memory

### High CPU Usage

**Symptoms:**
- High CPU usage by AlertView process
- Slow system performance
- AlertView is unresponsive

**Diagnosis:**

1. **Check CPU usage:**
   ```bash
   top
   htop
   ps aux | grep alertview
   ```

2. **Profile CPU usage:**
   ```bash
   # Using perf
   perf top -p $(pgrep alertview)
   
   # Using flamegraph
   cargo flamegraph --bench my_benchmark
   ```

**Solutions:**

- **Reduce refresh interval:** Fetch alerts less frequently
- **Reduce number of sources:** Monitor fewer systems
- **Enable caching:** Use `cache_ttl` to reduce API calls
- **Optimize configuration:** Review and optimize your configuration

## Source-Specific Issues

### Alertmanager Issues

**Symptom: No alerts from Alertmanager**

**Diagnosis:**

1. **Check Alertmanager API:**
   ```bash
   curl -v http://alertmanager.example.com:9093/api/v2/alerts
   ```

2. **Check Alertmanager logs:**
   ```bash
   kubectl logs alertmanager-0 -n monitoring
   ```

3. **Check Alertmanager configuration:**
   ```yaml
   # alertmanager.yml
   route:
     receiver: 'default'
   
   receivers:
   - name: 'default'
   
   web:
     expose: true  # API must be enabled
   ```

**Solutions:**

- **Enable API:** Ensure `web.expose: true` in Alertmanager config
- **Check API path:** Verify the API path is correct
- **Check authentication:** Verify basic auth or bearer token
- **Check network:** Verify network connectivity

### Grafana Issues

**Symptom: No alerts from Grafana**

**Diagnosis:**

1. **Check Grafana API:**
   ```bash
   curl -H "Authorization: Bearer your-api-key" \
     https://grafana.example.com/api/v1/alerts
   ```

2. **Check API key:**
   - Verify the API key is valid
   - Check if the API key has expired
   - Verify the API key has the correct permissions

3. **Check Grafana version:**
   ```bash
   curl -H "Authorization: Bearer your-api-key" \
     https://grafana.example.com/api/health
   ```

**Solutions:**

- **Invalid API key:** Generate a new API key
- **Expired API key:** Create a new API key without expiration
- **Insufficient permissions:** Grant Admin or Editor role to the API key
- **Wrong API version:** AlertView automatically detects the API version

### Zabbix Issues

**Symptom: No alerts from Zabbix**

**Diagnosis:**

1. **Check Zabbix API:**
   ```bash
   curl -v -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"user.login","params":{"user":"api-user","password":"api-password"},"id":1}' \
     https://zabbix.example.com/api_jsonrpc.php
   ```

2. **Check user permissions:**
   - Verify the user has API access
   - Check if the user has permissions to view triggers

3. **Check Zabbix version:**
   - AlertView supports Zabbix 3.0+

**Solutions:**

- **Invalid credentials:** Update username and password
- **Insufficient permissions:** Grant the user read access to triggers
- **API disabled:** Enable the Zabbix API
- **Wrong URL:** Use the correct API endpoint (`/api_jsonrpc.php`)

## Deployment Issues

### Docker Issues

**Symptom: Docker container won't start**

**Diagnosis:**

1. **Check container logs:**
   ```bash
   docker logs alertview
   ```

2. **Check container status:**
   ```bash
   docker ps -a | grep alertview
   docker inspect alertview
   ```

3. **Test running the container:**
   ```bash
   docker run -it --rm \
     -v /path/to/config.yaml:/etc/alertview/config.yaml:ro \
     -p 8080:8080 \
     ghcr.io/your-org/alertview:latest
   ```

**Solutions:**

- **Invalid config:** Fix configuration file
- **Missing config:** Mount the config file correctly
- **Port conflict:** Use a different port
- **Permission issues:** Ensure config file is readable

### Kubernetes Issues

**Symptom: Pod won't start**

**Diagnosis:**

1. **Check pod status:**
   ```bash
   kubectl get pods -n alertview
   kubectl describe pod alertview-xxxx -n alertview
   ```

2. **Check pod logs:**
   ```bash
   kubectl logs alertview-xxxx -n alertview
   kubectl logs alertview-xxxx -n alertview --previous
   ```

3. **Check ConfigMap:**
   ```bash
   kubectl get configmap alertview-config -n alertview
   kubectl describe configmap alertview-config -n alertview
   ```

**Solutions:**

- **Invalid config:** Fix the ConfigMap
- **Missing ConfigMap:** Create the ConfigMap
- **Image pull issues:** Check image name and registry access
- **Resource limits:** Increase memory/CPU limits

## UI Issues

### UI Not Loading

**Symptoms:**
- Blank page
- "Loading..." message stuck
- JavaScript errors in console

**Diagnosis:**

1. **Check browser console:**
   - Open developer tools (F12)
   - Check for JavaScript errors

2. **Check network requests:**
   - Verify `/api/alerts` returns valid JSON
   - Check for CORS errors

3. **Test API directly:**
   ```bash
   curl http://localhost:8080/api/alerts
   ```

**Solutions:**

- **CORS issues:** Configure CORS in reverse proxy
- **API errors:** Check AlertView logs for API errors
- **Browser issues:** Try a different browser
- **Cache issues:** Clear browser cache or use incognito mode

### UI Not Updating

**Symptoms:**
- Alerts don't update automatically
- "Last updated" timestamp doesn't change
- Manual refresh works but auto-refresh doesn't

**Diagnosis:**

1. **Check refresh interval:**
   ```yaml
   display:
     refresh_interval: 30  # Should be > 0
   ```

2. **Check browser console:**
   - Look for JavaScript errors
   - Check if the refresh timer is running

3. **Test API:**
   ```bash
   # First fetch
   curl http://localhost:8080/api/alerts
   
   # Wait for refresh interval
   sleep 30
   
   # Second fetch
   curl http://localhost:8080/api/alerts
   ```

**Solutions:**

- **Set refresh interval:** Ensure `refresh_interval > 0`
- **Check JavaScript:** Verify no errors in browser console
- **Check API:** Ensure the API is returning updated data
- **Disable cache:** Set `cache_ttl: 0` for real-time updates

### Sound Not Working

**Symptoms:**
- No sound when alerts appear
- Sound settings are enabled but no sound

**Diagnosis:**

1. **Check sound settings:**
   ```yaml
   display:
     play_sounds: true
   ```

2. **Check browser support:**
   - Verify browser supports Web Audio API
   - Check if browser is muted

3. **Test sound manually:**
   ```javascript
   // In browser console
   const audioContext = new (window.AudioContext || window.webkitAudioContext)();
   const oscillator = audioContext.createOscillator();
   oscillator.connect(audioContext.destination);
   oscillator.start();
   oscillator.stop(audioContext.currentTime + 0.5);
   ```

**Solutions:**

- **Enable sounds:** Set `play_sounds: true`
- **Check browser:** Use a browser that supports Web Audio API
- **Unmute browser:** Ensure browser tab is not muted
- **Check system:** Verify system sound is not muted

## Configuration Issues

### Invalid Configuration

**Symptoms:**
- "Invalid configuration" errors
- AlertView fails to start
- Unexpected behavior

**Diagnosis:**

1. **Validate YAML:**
   ```bash
   python3 -c "import yaml; yaml.safe_load(open('config.yaml'))"
   ```

2. **Check for typos:**
   - Verify all field names are correct
   - Check for missing colons, commas, etc.

3. **Check with debug logging:**
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   ```

**Solutions:**

- **Fix YAML syntax:** Correct any syntax errors
- **Fix field names:** Use correct field names from documentation
- **Remove invalid fields:** Remove unsupported configuration options

### Configuration Not Reloading

**Symptoms:**
- Changes to config file don't take effect
- Need to restart AlertView for changes to apply

**Diagnosis:**

1. **Check file watcher:**
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   # Look for "Config file changed" messages
   ```

2. **Check file permissions:**
   ```bash
   ls -la config.yaml
   # AlertView needs read access
   ```

3. **Test file watcher:**
   - Modify the config file
   - Check logs for reload messages

**Solutions:**

- **Check file permissions:** Ensure AlertView can read the file
- **Check inotify:** Ensure inotify is working (Linux)
- **Restart AlertView:** Manually restart to pick up changes
- **Use environment variables:** Some settings can only be set via environment variables

### Environment Variables Not Working

**Symptoms:**
- Environment variables are ignored
- Configuration file values are used instead

**Diagnosis:**

1. **Check variable names:**
   ```bash
   # Correct format
   ALERTVIEW_PORT=8080
   ALERTVIEW_REFRESH_INTERVAL=30
   ```

2. **Check variable is set:**
   ```bash
   echo $ALERTVIEW_PORT
   env | grep ALERTVIEW
   ```

3. **Check precedence:**
   - Command line > Environment variables > Config file > Defaults

**Solutions:**

- **Use correct names:** Use `ALERTVIEW_` prefix
- **Export variables:** Ensure variables are exported
- **Check scope:** Variables must be set before starting AlertView
- **Use uppercase:** Environment variable names are case-sensitive

## Performance Issues

### Slow Alert Fetching

**Symptoms:**
- Slow UI updates
- Long delays between refreshes
- High latency in API responses

**Diagnosis:**

1. **Check response times:**
   ```bash
   time curl http://localhost:8080/api/alerts
   ```

2. **Check source response times:**
   ```bash
   time curl http://alertmanager.example.com:9093/api/v2/alerts
   ```

3. **Check with profiling:**
   ```bash
   RUST_LOG=debug alertview --config config.yaml
   ```

**Solutions:**

- **Enable caching:** Set `cache_ttl` to a reasonable value
- **Reduce refresh interval:** Fetch alerts less frequently
- **Filter alerts:** Use filters to reduce the number of alerts
- **Optimize sources:** Improve performance of your monitoring systems

### High Memory Usage

**Symptoms:**
- AlertView process uses a lot of memory
- Memory usage grows over time
- Out of memory errors

**Diagnosis:**

1. **Check memory usage:**
   ```bash
   ps aux | grep alertview
   top
   ```

2. **Monitor over time:**
   ```bash
   watch -n 5 'ps aux | grep alertview'
   ```

**Solutions:**

- **Reduce cache:** Lower `cache_ttl` or disable caching
- **Reduce alerts:** Use filters to reduce the number of alerts
- **Reduce sources:** Monitor fewer systems
- **Restart regularly:** Schedule regular restarts

## Network Issues

### DNS Resolution Issues

**Symptoms:**
- "Name or service not known" errors
- "Could not resolve host" errors

**Diagnosis:**

1. **Test DNS resolution:**
   ```bash
   nslookup alertmanager.example.com
   dig alertmanager.example.com
   ping alertmanager.example.com
   ```

2. **Check /etc/hosts:**
   ```bash
   cat /etc/hosts
   ```

3. **Check DNS configuration:**
   ```bash
   cat /etc/resolv.conf
   ```

**Solutions:**

- **Fix DNS:** Configure DNS correctly
- **Use /etc/hosts:** Add entries to /etc/hosts
- **Use IP addresses:** Use IP addresses instead of hostnames

### Proxy Issues

**Symptoms:**
- "Connection refused" through proxy
- "Proxy authentication required" errors

**Diagnosis:**

1. **Check proxy settings:**
   ```bash
   echo $http_proxy
   echo $https_proxy
   echo $no_proxy
   ```

2. **Test through proxy:**
   ```bash
   curl -x http://proxy.example.com:8080 http://alertmanager.example.com:9093
   ```

**Solutions:**

- **Configure proxy:** Set `http_proxy` and `https_proxy` environment variables
- **No proxy for local:** Add monitoring systems to `no_proxy`
- **Proxy authentication:** Configure proxy authentication

## Debugging Tools

### Logging

AlertView uses the `tracing` crate for logging. Set `RUST_LOG` to control log level:

```bash
# Log levels (from least to most verbose)
RUST_LOG=error    # Only errors
RUST_LOG=warn     # Warnings and errors
RUST_LOG=info     # Information, warnings, and errors (default)
RUST_LOG=debug    # Debug information
RUST_LOG=trace    # Maximum verbosity
```

**Log to file:**

```bash
RUST_LOG=info alertview --config config.yaml > alertview.log 2>&1
```

**Rotate logs:**

Use a process manager or logrotate to manage log files.

### Health Check

Check AlertView health:

```bash
curl http://localhost:8080/health

# With details
curl -H "Accept: application/json" http://localhost:8080/health
```

### API Testing

Test the API with curl:

```bash
# Get all alerts
curl http://localhost:8080/api/alerts

# Get alerts from specific source
curl http://localhost:8080/api/alerts/alertmanager

# Get source information
curl http://localhost:8080/api/sources

# Get configuration
curl http://localhost:8080/api/config
```

### Browser Developer Tools

Use browser developer tools to debug UI issues:

1. **Open developer tools:** F12 or Ctrl+Shift+I
2. **Check console:** Look for JavaScript errors
3. **Check network:** Verify API requests are successful
4. **Check performance:** Identify performance bottlenecks

## Common Error Messages

### "Configuration error: invalid type"

**Cause:** Invalid configuration value type

**Solution:** Check the configuration file for invalid types (e.g., string instead of number)

### "Failed to parse configuration: ..."

**Cause:** Invalid YAML syntax or structure

**Solution:** Fix YAML syntax errors or invalid configuration structure

### "Connection refused"

**Cause:** Cannot connect to monitoring system

**Solution:** Check if the monitoring system is running and accessible

### "401 Unauthorized"

**Cause:** Authentication failed

**Solution:** Check API keys, usernames, and passwords

### "404 Not Found"

**Cause:** URL not found

**Solution:** Check the URL in your configuration

### "Request timeout"

**Cause:** Request took too long

**Solution:** Increase timeout or check monitoring system performance

### "SSL certificate problem"

**Cause:** SSL/TLS certificate issue

**Solution:** Configure TLS settings or fix certificates

## Getting Help

### Collect Information

When asking for help, collect the following information:

1. **AlertView version:**
   ```bash
   alertview --version
   ```

2. **Configuration:**
   ```bash
   cat config.yaml
   ```

3. **Logs:**
   ```bash
   RUST_LOG=debug alertview --config config.yaml 2>&1 | head -100
   ```

4. **Environment:**
   - Operating system
   - Rust version (`rustc --version`)
   - Docker version (if using Docker)
   - Kubernetes version (if using Kubernetes)

5. **Monitoring systems:**
   - Alertmanager version
   - Grafana version
   - Zabbix version

### Ask for Help

1. **Check documentation:** Review this troubleshooting guide
2. **Search issues:** Check if the issue has been reported
3. **Open an issue:** Create a new issue with the collected information
4. **Ask in discussions:** Start a discussion for general questions

## Additional Resources

- [Configuration Reference](configuration/config-file.md)
- [Examples](examples/README.md)
- [API Documentation](api.md)
- [Deployment Guide](deployment/README.md)
- [FAQ](faq.md)
