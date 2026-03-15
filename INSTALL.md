# Installation Guide

## 🚀 One-Script Install (Recommended)

The easiest way to install Synth on Linux.

### Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/Bentlybro/synth/main/install.sh | bash
```

**Or download first:**
```bash
wget https://raw.githubusercontent.com/Bentlybro/synth/main/install.sh
chmod +x install.sh
./install.sh
```

### What It Does

The installer automatically:

1. **Checks prerequisites**
   - Verifies systemd is available
   - Checks for curl and jq

2. **Installs dependencies**
   - Rust (if not installed)
   - yt-dlp (for video transcription)

3. **Sets up Synth**
   - Clones repository to `~/synth`
   - Prompts for API keys (ANTHROPIC_API_KEY, OPENAI_API_KEY)
   - Creates `.env` configuration

4. **Builds and configures**
   - Compiles release binary
   - Creates systemd service
   - Enables auto-start on boot

5. **Tests installation**
   - Starts service
   - Runs health check

### Post-Install

After installation completes:

```bash
# Check status
systemctl --user status synth

# View logs
journalctl --user -u synth -f

# Test with a query
curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming", "max_pages": 2}'
```

### With OpenClaw

If you're using OpenClaw, you can install Synth with a simple request:

**Just say:**
> "Install Synth from https://github.com/Bentlybro/synth"

OpenClaw will:
1. Download the installer
2. Run it automatically
3. Handle dependencies
4. Set up the service
5. Confirm when complete

### Configuration

The installer creates a `.env` file at `~/synth/.env`:

```bash
# Required
ANTHROPIC_API_KEY=sk-ant-...

# Optional (for video/audio transcription)
OPENAI_API_KEY=sk-...

# SearXNG URL
SEARXNG_URL=http://localhost:8888

# Logging
RUST_LOG=info
```

Edit this file to update API keys:

```bash
nano ~/synth/.env
systemctl --user restart synth
```

### SearXNG Setup

Synth requires SearXNG for web search. Quick start with Docker:

```bash
docker run -d -p 8888:8080 searxng/searxng:latest
```

Or use docker-compose:

```bash
cd ~/synth
docker-compose up -d searxng
```

### Troubleshooting

**Service won't start:**
```bash
# Check logs
journalctl --user -u synth --no-pager | tail -50

# Verify API keys
cat ~/synth/.env

# Test manually
cd ~/synth
./target/release/synth
```

**Health check fails:**
```bash
# Check if port is in use
lsof -i :8765

# Verify service is running
systemctl --user status synth

# Try manual curl
curl http://localhost:8765/health
```

**Missing dependencies:**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install yt-dlp
pip3 install yt-dlp

# Install jq
sudo apt-get install jq
```

### Updating

To update to the latest version:

```bash
cd ~/synth
git pull
cargo build --release
systemctl --user restart synth
```

Or run the installer again (it will detect existing installation):

```bash
curl -fsSL https://raw.githubusercontent.com/Bentlybro/synth/main/install.sh | bash
```

### Uninstall

```bash
# Stop and disable service
systemctl --user stop synth
systemctl --user disable synth

# Remove service file
rm ~/.config/systemd/user/synth.service
systemctl --user daemon-reload

# Remove installation directory
rm -rf ~/synth
```

---

## Manual Installation

See [README.md](README.md#manual-installation) for manual installation steps.

## Docker Installation

See [docker-compose.yml](docker-compose.yml) for Docker deployment.

---

## Support

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/Bentlybro/synth/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Bentlybro/synth/discussions)
