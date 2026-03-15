# Deployment Guide

Production deployment options for Synth.

## Deployment Options

1. **Systemd Service** (Recommended for single-user)
2. **Docker Compose** (Recommended for multi-user)
3. **Standalone Binary** (Development/testing)

---

## 1. Systemd Service

### Installation

```bash
# Build release binary
cd ~/synth
cargo build --release

# Create service file
mkdir -p ~/.config/systemd/user
cp synth.service ~/.config/systemd/user/

# Edit paths and environment
nano ~/.config/systemd/user/synth.service
```

### Service File

```ini
[Unit]
Description=Synth - AI research engine
Documentation=https://github.com/Bentlybro/synth
After=network.target

[Service]
Type=simple
WorkingDirectory=/home/USER/synth
ExecStart=/home/USER/synth/target/release/synth
Restart=on-failure
RestartSec=5s

# Environment
Environment="ANTHROPIC_API_KEY=sk-ant-..."
Environment="OPENAI_API_KEY=sk-..."
Environment="SEARXNG_URL=http://localhost:8888"
Environment="RUST_LOG=info"

[Install]
WantedBy=default.target
```

### Enable and Start

```bash
# Reload systemd
systemctl --user daemon-reload

# Enable auto-start
systemctl --user enable synth

# Start service
systemctl --user start synth

# Check status
systemctl --user status synth

# View logs
journalctl --user -u synth -f
```

### Troubleshooting

```bash
# Check if running
systemctl --user is-active synth

# Restart
systemctl --user restart synth

# Stop
systemctl --user stop synth

# View full logs
journalctl --user -u synth --no-pager | tail -100
```

---

## 2. Docker Compose

### docker-compose.yml

```yaml
version: '3.8'

services:
  searxng:
    image: searxng/searxng:latest
    container_name: searxng
    ports:
      - "8888:8080"
    volumes:
      - ./searxng:/etc/searxng:rw
    restart: unless-stopped

  synth:
    build: .
    container_name: synth
    ports:
      - "8765:8765"
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - SEARXNG_URL=http://searxng:8080
      - RUST_LOG=info
    volumes:
      - ./index:/app/index:rw
    depends_on:
      - searxng
    restart: unless-stopped
```

### .env File

```bash
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
```

### Dockerfile

```dockerfile
FROM rust:1.88 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    python3-pip \
    && pip3 install --break-system-packages yt-dlp \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/synth /app/synth

EXPOSE 8765
CMD ["/app/synth"]
```

### Deploy

```bash
# Build and start
docker-compose up -d

# View logs
docker-compose logs -f synth

# Check status
docker-compose ps

# Stop
docker-compose down

# Rebuild after changes
docker-compose up -d --build
```

---

## 3. Reverse Proxy (Nginx)

### Nginx Config

```nginx
server {
    listen 80;
    server_name synth.example.com;

    location / {
        proxy_pass http://localhost:8765;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts for long-running requests
        proxy_connect_timeout 300s;
        proxy_send_timeout 300s;
        proxy_read_timeout 300s;
    }
}
```

### SSL with Let's Encrypt

```bash
# Install certbot
sudo apt-get install certbot python3-certbot-nginx

# Get certificate
sudo certbot --nginx -d synth.example.com

# Auto-renewal
sudo certbot renew --dry-run
```

---

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | ✅ | - | Claude API key |
| `OPENAI_API_KEY` | ❌ | - | Whisper API key (video/audio) |
| `SEARXNG_URL` | ❌ | `http://localhost:8888` | SearXNG instance URL |
| `CACHE_TTL_SECONDS` | ❌ | `86400` | Legacy cache TTL |
| `RUST_LOG` | ❌ | `info` | Log level (error/warn/info/debug) |

---

## Performance Tuning

### Increase Concurrency

Edit `src/main.rs`:

```rust
let scraper = Scraper::new(50);  // Change from 50 to 100
```

Edit `src/api/mod.rs`:

```rust
.buffer_unordered(10)  // Change from 10 to 20 (URL extraction)
```

```rust
.buffer_unordered(5)   // Change from 5 to 10 (LLM analysis)
```

Rebuild and restart.

### Disk Space

**Monitor:**
```bash
du -sh index/cache/
```

**Cleanup:**
```bash
# Manual cleanup
find index/cache -name "*.json" -mtime +7 -delete

# Or reduce TTLs (see Caching docs)
```

### Memory

Synth is memory-efficient (~10-50 MB resident).

**If using Docker:**
```yaml
synth:
  # ...
  deploy:
    resources:
      limits:
        memory: 512M
      reservations:
        memory: 128M
```

---

## Monitoring

### Health Checks

```bash
# Simple check
curl http://localhost:8765/health

# Or use this script
cat > check_synth.sh << 'EOF'
#!/bin/bash
if curl -s http://localhost:8765/health > /dev/null; then
    echo "Synth is healthy"
    exit 0
else
    echo "Synth is down!"
    exit 1
fi
EOF
chmod +x check_synth.sh

# Add to cron
crontab -e
# */5 * * * * /path/to/check_synth.sh || systemctl --user restart synth
```

### Logs

```bash
# Systemd
journalctl --user -u synth -f

# Docker
docker-compose logs -f synth

# Check for errors
journalctl --user -u synth --since today | grep ERROR
```

### Metrics

**Future:** Prometheus integration planned

---

## Backup

### What to Backup

1. **Cache** - `index/cache/` (optional, regenerates)
2. **Config** - `.env` or service file
3. **Binary** - `target/release/synth` (can rebuild)

### Backup Script

```bash
#!/bin/bash
BACKUP_DIR=~/synth_backups/$(date +%Y%m%d)
mkdir -p $BACKUP_DIR

# Backup cache (optional)
tar -czf $BACKUP_DIR/cache.tar.gz index/cache/

# Backup config
cp .env $BACKUP_DIR/
cp ~/.config/systemd/user/synth.service $BACKUP_DIR/

echo "Backup complete: $BACKUP_DIR"
```

---

## Updates

### Update Synth

```bash
# Pull latest
git pull

# Rebuild
cargo build --release

# Restart
systemctl --user restart synth
# Or: docker-compose up -d --build
```

### Update Dependencies

```bash
# Update Rust
rustup update

# Update Cargo dependencies
cargo update

# Rebuild
cargo build --release
```

---

## Troubleshooting

### Service Won't Start

```bash
# Check logs
journalctl --user -u synth --no-pager | tail -50

# Check if port is in use
lsof -i :8765

# Test manually
./target/release/synth
```

### Slow Performance

1. Check cache size: `du -sh index/cache/`
2. Check disk space: `df -h`
3. Check CPU usage: `top`
4. Enable debug logs: `RUST_LOG=debug`

### API Errors

1. Verify API keys are set
2. Check internet connectivity
3. Test Claude API: `curl https://api.anthropic.com/v1/complete`
4. Test Whisper API: `curl https://api.openai.com/v1/audio/transcriptions`

### YouTube Fails

```bash
# Install yt-dlp
pip install -U yt-dlp

# Test manually
yt-dlp --version
yt-dlp https://youtube.com/watch?v=dQw4w9WgXcQ
```

---

## Security

### API Key Protection

**DO NOT:**
- Commit `.env` to git
- Expose API keys in logs
- Share Docker images with keys embedded

**DO:**
- Use environment variables
- Rotate keys regularly
- Use separate keys for dev/prod

### Network Security

**Firewall:**
```bash
# Allow localhost only (default)
# No firewall rules needed

# Allow LAN access
sudo ufw allow from 192.168.0.0/24 to any port 8765

# Public access (use reverse proxy)
# Set up Nginx with SSL
```

---

## Next Steps

- Read [API Documentation](../api/) for endpoint details
- Read [Development Guide](../development/) for customization
- Read [Architecture](../architecture/) for system design
