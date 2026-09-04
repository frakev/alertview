# Docker Compose Deployment

Deploy AlertView with Docker Compose for multi-container setups.

## Basic Example

```yaml
version: '3.8'

services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/config/config.yaml:ro
    restart: unless-stopped
    environment:
      - ALERTVIEW_PORT=8080
      - ALERTVIEW_REFRESH_INTERVAL=30
```

Run with:
```bash
docker compose up -d
```

## Complete Example with Alertmanager

```yaml
version: '3.8'

services:
  alertmanager:
    image: prom/alertmanager:latest
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml
    command:
      - '--config.file=/etc/alertmanager/alertmanager.yml'
      - '--storage.path=/alertmanager'

  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/config/config.yaml:ro
    depends_on:
      - alertmanager
    restart: unless-stopped
    environment:
      - ALERTVIEW_REFRESH_INTERVAL=15
```

## With Multiple Sources

```yaml
version: '3.8'

services:
  alertmanager:
    image: prom/alertmanager:latest
    ports:
      - "9093:9093"

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana-storage:/var/lib/grafana

  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/config/config.yaml:ro
    depends_on:
      - alertmanager
      - grafana
    restart: unless-stopped

volumes:
  grafana-storage:
```

## With Reverse Proxy

```yaml
version: '3.8'

services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    volumes:
      - ./config.yaml:/config/config.yaml:ro
    environment:
      - ALERTVIEW_PORT=8080
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./certs:/etc/nginx/certs
    depends_on:
      - alertview
    restart: unless-stopped
```

Example `nginx.conf`:
```nginx
server {
    listen 80;
    server_name alertview.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name alertview.example.com;
    
    ssl_certificate /etc/nginx/certs/fullchain.pem;
    ssl_certificate_key /etc/nginx/certs/privkey.pem;
    
    location / {
        proxy_pass http://alertview:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## With Prometheus

```yaml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-storage:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'

  alertmanager:
    image: prom/alertmanager:latest
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager.yml:/etc/alertmanager/alertmanager.yml

  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/config/config.yaml:ro
    depends_on:
      - alertmanager
    restart: unless-stopped

volumes:
  prometheus-storage:
```

## Environment-Specific Configurations

```yaml
version: '3.8'

services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.${ENVIRONMENT:-production}.yaml:/config/config.yaml:ro
    environment:
      - ENVIRONMENT=${ENVIRONMENT:-production}
      - ALERTVIEW_PORT=8080
    restart: unless-stopped
```

Run with:
```bash
# Production
ENVIRONMENT=production docker compose up -d

# Development
ENVIRONMENT=development docker compose up -d
```

## With Health Checks

```yaml
version: '3.8'

services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/config/config.yaml:ro
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 5s
    restart: unless-stopped
```

## With Resource Limits

```yaml
version: '3.8'

services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config.yaml:/config/config.yaml:ro
    deploy:
      resources:
        limits:
          cpus: '0.5'
          memory: 128M
        reservations:
          cpus: '0.1'
          memory: 64M
    restart: unless-stopped
```

## Commands

```bash
# Start services
docker compose up -d

# Stop services
docker compose down

# View logs
docker compose logs -f

# View specific service logs
docker compose logs -f alertview

# Restart services
docker compose restart

# Rebuild images
docker compose build

# Pull latest images
docker compose pull

# View service status
docker compose ps

# Execute command in container
docker compose exec alertview /bin/sh
```

## Tips

1. **Use `depends_on`** to ensure dependencies start first
2. **Set `restart: unless-stopped`** for automatic restarts
3. **Use named volumes** for persistent data
4. **Configure health checks** for monitoring
5. **Set resource limits** to prevent resource exhaustion
6. **Use environment variables** for configuration
7. **Version pin images** for production stability
8. **Use `.env` file** for environment-specific settings

## Example .env File

```bash
# .env
ALERTVIEW_PORT=8080
ALERTVIEW_REFRESH_INTERVAL=30
ALERTVIEW_LOG_FORMAT=json
```

Reference in docker-compose.yml:
```yaml
services:
  alertview:
    image: ghcr.io/frakev/alertview:latest
    environment:
      - ALERTVIEW_PORT=${ALERTVIEW_PORT}
      - ALERTVIEW_REFRESH_INTERVAL=${ALERTVIEW_REFRESH_INTERVAL}
      - ALERTVIEW_LOG_FORMAT=${ALERTVIEW_LOG_FORMAT}
```
