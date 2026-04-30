# Reverse Proxy Configuration

This guide covers configuring various reverse proxies to serve AlertView, including authentication, SSL/TLS termination, and path-based routing.

## Overview

AlertView runs on port 8080 by default. In production, you typically want to:
- Serve it on standard HTTP (80) or HTTPS (443) ports
- Add authentication
- Enable SSL/TLS encryption
- Route based on domain or path
- Add rate limiting or other middleware

A reverse proxy sits between clients and AlertView, handling these concerns.

## Nginx

### Basic Configuration

```nginx
server {
    listen 80;
    server_name alertview.example.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### HTTPS with Let's Encrypt

```nginx
server {
    listen 80;
    server_name alertview.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name alertview.example.com;

    ssl_certificate /etc/letsencrypt/live/alertview.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/alertview.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Basic Authentication

1. Create a password file:

```bash
# Install apache2-utils if needed
sudo apt-get install apache2-utils

# Create password file
sudo htpasswd -c /etc/nginx/.htpasswd username
```

2. Update Nginx configuration:

```nginx
server {
    listen 80;
    server_name alertview.example.com;

    location / {
        auth_basic "AlertView Authentication";
        auth_basic_user_file /etc/nginx/.htpasswd;

        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Path-Based Routing

Serve AlertView under a subpath (e.g., `/alertview`):

```nginx
server {
    listen 80;
    server_name example.com;

    location /alertview/ {
        proxy_pass http://localhost:8080/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_redirect off;
    }
}
```

**Important:** When using path-based routing, you must configure AlertView with the base path:

```yaml
# In your AlertView config
display:
  base_path: /alertview
```

Or via environment variable:

```bash
ALERTVIEW_BASE_PATH=/alertview
```

### Gzip Compression

AlertView supports gzip compression natively, but you can also enable it at the proxy level:

```nginx
server {
    listen 80;
    server_name alertview.example.com;

    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml;
    gzip_min_length 1000;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### WebSocket Support

If you use WebSocket connections (for real-time updates):

```nginx
server {
    listen 80;
    server_name alertview.example.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

## Apache HTTP Server

### Basic Configuration

```apache
<VirtualHost *:80>
    ServerName alertview.example.com

    ProxyPreserveHost On
    ProxyPass / http://localhost:8080/
    ProxyPassReverse / http://localhost:8080/

    ErrorLog ${APACHE_LOG_DIR}/alertview-error.log
    CustomLog ${APACHE_LOG_DIR}/alertview-access.log combined
</VirtualHost>
```

### HTTPS Configuration

```apache
<VirtualHost *:443>
    ServerName alertview.example.com

    SSLEngine on
    SSLCertificateFile /etc/letsencrypt/live/alertview.example.com/cert.pem
    SSLCertificateKeyFile /etc/letsencrypt/live/alertview.example.com/privkey.pem
    SSLCertificateChainFile /etc/letsencrypt/live/alertview.example.com/chain.pem

    ProxyPreserveHost On
    ProxyPass / http://localhost:8080/
    ProxyPassReverse / http://localhost:8080/

    ErrorLog ${APACHE_LOG_DIR}/alertview-error.log
    CustomLog ${APACHE_LOG_DIR}/alertview-access.log combined
</VirtualHost>
```

### Basic Authentication

```apache
<VirtualHost *:80>
    ServerName alertview.example.com

    <Location />
        AuthType Basic
        AuthName "AlertView Authentication"
        AuthUserFile /etc/apache2/.htpasswd
        Require valid-user
    </Location>

    ProxyPreserveHost On
    ProxyPass / http://localhost:8080/
    ProxyPassReverse / http://localhost:8080/
</VirtualHost>
```

## Caddy

Caddy provides automatic HTTPS with Let's Encrypt out of the box.

### Basic Configuration

```caddy
alertview.example.com {
    reverse_proxy localhost:8080
}
```

That's it! Caddy will automatically:
- Obtain and renew SSL certificates from Let's Encrypt
- Redirect HTTP to HTTPS
- Proxy requests to AlertView

### With Basic Authentication

```caddy
alertview.example.com {
    basicauth {
        username encrypted_password
    }
    reverse_proxy localhost:8080
}
```

To create the encrypted password:

```bash
caddy hash-password --plaintext yourpassword
```

### Path-Based Routing

```caddy
example.com {
    handle /alertview* {
        reverse_proxy localhost:8080
    }
}
```

## Traefik

### Docker Compose with Traefik

```yaml
version: '3'

services:
  traefik:
    image: traefik:v2.10
    command:
      - --api.insecure=true
      - --providers.docker
      - --entrypoints.web.address=:80
      - --entrypoints.websecure.address=:443
      - --certificatesresolvers.myresolver.acme.tlschallenge=true
      - --certificatesresolvers.myresolver.acme.email=your@email.com
      - --certificatesresolvers.myresolver.acme.storage=/letsencrypt/acme.json
    ports:
      - "80:80"
      - "443:443"
      - "8080:8080"  # Traefik dashboard
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - ./letsencrypt:/letsencrypt

  alertview:
    image: ghcr.io/your-org/alertview:latest
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.alertview.rule=Host(`alertview.example.com`)"
      - "traefik.http.routers.alertview.entrypoints=websecure"
      - "traefik.http.routers.alertview.tls.certresolver=myresolver"
      - "traefik.http.services.alertview.loadbalancer.server.port=8080"
```

### With Basic Authentication

```yaml
  alertview:
    image: ghcr.io/your-org/alertview:latest
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.alertview.rule=Host(`alertview.example.com`)"
      - "traefik.http.routers.alertview.entrypoints=websecure"
      - "traefik.http.routers.alertview.tls.certresolver=myresolver"
      - "traefik.http.routers.alertview.middlewares=auth"
      - "traefik.http.middlewares.auth.basicauth.users=username:$$apr1$$hashedpassword"
      - "traefik.http.services.alertview.loadbalancer.server.port=8080"
```

To create the hashed password:

```bash
htpasswd -nb username password | openssl passwd -apr1 -stdin
```

## HAProxy

### Basic Configuration

```haproxy
frontend http-in
    bind *:80
    acl is_alertview hdr(host) -i alertview.example.com
    use_backend alertview if is_alertview

backend alertview
    server alertview1 127.0.0.1:8080 check
```

### HTTPS Configuration

```haproxy
frontend https-in
    bind *:443 ssl crt /etc/haproxy/certs/alertview.pem
    acl is_alertview hdr(host) -i alertview.example.com
    use_backend alertview if is_alertview

backend alertview
    server alertview1 127.0.0.1:8080 check
```

## Cloudflare Tunnel (Argo Tunnel)

Cloudflare Tunnel allows you to expose AlertView without opening ports on your firewall.

### Installation

1. Install cloudflared:

```bash
# On Ubuntu/Debian
sudo apt-get install cloudflared

# Or download directly
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o cloudflared
chmod +x cloudflared
sudo mv cloudflared /usr/local/bin/
```

2. Authenticate:

```bash
cloudflared tunnel login
```

3. Create a tunnel:

```bash
cloudflared tunnel create alertview-tunnel
```

4. Create a configuration file (`~/.cloudflared/config.yml`):

```yaml
tunnel: alertview-tunnel
credentials-file: /path/to/credentials.json

ingress:
  - hostname: alertview.example.com
    service: http://localhost:8080
  - service: http_status:404
```

5. Run the tunnel:

```bash
cloudflared tunnel run alertview-tunnel
```

6. Update DNS records:

```bash
cloudflared tunnel route dns alertview-tunnel alertview.example.com
```

### As a Service

Create a systemd service for automatic startup:

```ini
# /etc/systemd/system/cloudflared-tunnel.service
[Unit]
Description=Cloudflare Tunnel for AlertView
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/cloudflared tunnel run alertview-tunnel
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Then enable and start:

```bash
sudo systemctl enable cloudflared-tunnel.service
sudo systemctl start cloudflared-tunnel.service
```

## Common Configuration Patterns

### Multiple Instances

Route different domains to different AlertView instances:

```nginx
# Nginx example
server {
    listen 80;
    server_name alertview-staging.example.com;
    location / {
        proxy_pass http://localhost:8081;  # Staging instance
    }
}

server {
    listen 80;
    server_name alertview.example.com;
    location / {
        proxy_pass http://localhost:8080;  # Production instance
    }
}
```

### Load Balancing

Distribute traffic across multiple AlertView instances:

```nginx
upstream alertview_backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.2:8080;
}

server {
    listen 80;
    server_name alertview.example.com;
    location / {
        proxy_pass http://alertview_backend;
    }
}
```

### Rate Limiting

Protect against abuse:

```nginx
limit_req_zone $binary_remote_addr zone=alertview:10m rate=10r/s;

server {
    listen 80;
    server_name alertview.example.com;

    location / {
        limit_req zone=alertview burst=20 nodelay;
        proxy_pass http://localhost:8080;
    }
}
```

### IP Whitelisting

Restrict access to specific IPs:

```nginx
server {
    listen 80;
    server_name alertview.example.com;

    location / {
        allow 192.168.1.0/24;
        allow 10.0.0.0/8;
        deny all;

        proxy_pass http://localhost:8080;
    }
}
```

### Custom Headers

Add custom headers to responses:

```nginx
server {
    listen 80;
    server_name alertview.example.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Custom headers
        add_header X-Frame-Options "SAMEORIGIN" always;
        add_header X-Content-Type-Options "nosniff" always;
        add_header X-XSS-Protection "1; mode=block" always;
    }
}
```

## Troubleshooting

### Check Proxy Logs

```bash
# Nginx
sudo tail -f /var/log/nginx/error.log
sudo tail -f /var/log/nginx/access.log

# Apache
sudo tail -f /var/log/apache2/error.log
sudo tail -f /var/log/apache2/access.log

# Caddy
sudo journalctl -u caddy -f

# Traefik
sudo docker logs traefik_container_name
```

### Test Connectivity

```bash
# Test if AlertView is running locally
curl -I http://localhost:8080/health

# Test through the proxy
curl -I http://alertview.example.com/health
curl -v http://alertview.example.com/
```

### Common Issues

#### 502 Bad Gateway

This usually means the proxy cannot connect to AlertView:

1. Verify AlertView is running: `curl -I http://localhost:8080/health`
2. Check the proxy configuration for typos
3. Verify the port is correct
4. Check firewall rules

#### 404 Not Found

If you're using path-based routing:
- Ensure the base_path is configured in AlertView
- Check the proxy_pass directive includes the trailing slash

#### SSL Errors

- Verify certificate paths are correct
- Ensure certificates are valid (not expired)
- Check certificate permissions

#### Authentication Not Working

- Verify the password file exists and is readable
- Check the authentication module is enabled
- Test with a simple configuration first

## Best Practices

1. **Always use HTTPS** in production
2. **Enable authentication** to prevent unauthorized access
3. **Set proper timeouts** to match your use case
4. **Monitor proxy performance** and adjust as needed
5. **Keep software updated** (proxy and AlertView)
6. **Use health checks** to monitor backend status
7. **Implement rate limiting** to prevent abuse
8. **Log all access** for auditing

## Additional Resources

- [Nginx Documentation](https://nginx.org/en/docs/)
- [Apache HTTP Server Documentation](https://httpd.apache.org/docs/)
- [Caddy Documentation](https://caddyserver.com/docs/)
- [Traefik Documentation](https://doc.traefik.io/traefik/)
- [HAProxy Documentation](https://www.haproxy.org/documentation/)
- [Cloudflare Tunnel Documentation](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/)
