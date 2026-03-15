#!/bin/bash
#
# Synth - One-Script Installer
# https://github.com/Bentlybro/synth
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Bentlybro/synth/main/install.sh | bash
#   or
#   ./install.sh
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Config
INSTALL_DIR="${SYNTH_INSTALL_DIR:-$HOME/synth}"
SERVICE_NAME="synth"
PORT=8765

# Functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

step() {
    echo ""
    echo -e "${GREEN}==>${NC} $1"
}

# Banner
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}                  ${GREEN}Synth Installer${NC}                          ${BLUE}║${NC}"
echo -e "${BLUE}║${NC}     Universal AI Research Engine with Multi-Modal Support    ${BLUE}║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check OS
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    error "Only Linux is supported. Detected: $OSTYPE"
fi

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    error "Do not run as root. Run as your normal user."
fi

# Step 1: Check prerequisites
step "Checking prerequisites"

# Check for systemd
if ! command -v systemctl &> /dev/null; then
    error "systemd not found. This installer requires systemd."
fi

# Check for curl
if ! command -v curl &> /dev/null; then
    error "curl not found. Install with: sudo apt-get install curl"
fi

# Check for jq
if ! command -v jq &> /dev/null; then
    warn "jq not found. Installing..."
    if command -v apt-get &> /dev/null; then
        sudo apt-get update && sudo apt-get install -y jq
    else
        error "Please install jq manually"
    fi
fi

success "Prerequisites checked"

# Step 2: Install Rust
step "Checking Rust installation"

if ! command -v cargo &> /dev/null; then
    info "Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    success "Rust installed"
else
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    info "Rust already installed: $RUST_VERSION"
fi

# Ensure cargo is in PATH
if ! command -v cargo &> /dev/null; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# Step 3: Install yt-dlp (for video support)
step "Checking yt-dlp installation"

if ! command -v yt-dlp &> /dev/null; then
    info "yt-dlp not found. Installing..."
    if command -v pip3 &> /dev/null; then
        pip3 install --user yt-dlp
        success "yt-dlp installed via pip3"
    elif command -v pip &> /dev/null; then
        pip install --user yt-dlp
        success "yt-dlp installed via pip"
    else
        warn "pip not found. Video transcription will not work without yt-dlp."
        warn "Install with: pip3 install yt-dlp"
    fi
else
    info "yt-dlp already installed"
fi

# Step 4: Clone/Update repository
step "Setting up Synth"

if [ -d "$INSTALL_DIR" ]; then
    info "Synth directory exists at $INSTALL_DIR"
    read -p "Update existing installation? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cd "$INSTALL_DIR"
        git pull
        success "Updated to latest version"
    fi
else
    info "Cloning Synth repository to $INSTALL_DIR"
    git clone https://github.com/Bentlybro/synth.git "$INSTALL_DIR"
    success "Repository cloned"
fi

cd "$INSTALL_DIR"

# Step 5: Configure environment
step "Configuring environment"

ENV_FILE="$INSTALL_DIR/.env"

if [ ! -f "$ENV_FILE" ]; then
    info "Creating .env file"
    
    # Check if keys are in environment
    ANTHROPIC_KEY="${ANTHROPIC_API_KEY:-}"
    OPENAI_KEY="${OPENAI_API_KEY:-}"
    
    # Prompt for keys if not set
    if [ -z "$ANTHROPIC_KEY" ]; then
        echo ""
        echo "Anthropic API key required for Claude analysis."
        echo "Get one at: https://console.anthropic.com/"
        read -p "Enter ANTHROPIC_API_KEY (or press Enter to skip): " ANTHROPIC_KEY
    fi
    
    if [ -z "$OPENAI_KEY" ]; then
        echo ""
        echo "OpenAI API key optional (for video/audio transcription)."
        echo "Get one at: https://platform.openai.com/"
        read -p "Enter OPENAI_API_KEY (or press Enter to skip): " OPENAI_KEY
    fi
    
    # Create .env
    cat > "$ENV_FILE" << EOF
# Synth Configuration
# Required
ANTHROPIC_API_KEY=${ANTHROPIC_KEY}

# Optional (for video/audio transcription)
OPENAI_API_KEY=${OPENAI_KEY}

# SearXNG URL (default: localhost)
SEARXNG_URL=http://localhost:8888

# Logging
RUST_LOG=info
EOF
    
    success "Environment configured"
    
    if [ -z "$ANTHROPIC_KEY" ]; then
        warn "ANTHROPIC_API_KEY not set. You'll need to add it to $ENV_FILE before using Synth."
    fi
else
    info ".env file already exists"
fi

# Step 6: Build Synth
step "Building Synth (this may take a few minutes)"

info "Compiling release build..."
cargo build --release

if [ $? -eq 0 ]; then
    success "Build complete"
else
    error "Build failed. Check the output above for errors."
fi

# Step 7: Set up systemd service
step "Setting up systemd service"

SERVICE_FILE="$HOME/.config/systemd/user/$SERVICE_NAME.service"
mkdir -p "$HOME/.config/systemd/user"

info "Creating systemd service at $SERVICE_FILE"

cat > "$SERVICE_FILE" << EOF
[Unit]
Description=Synth - Universal AI Research Engine
Documentation=https://github.com/Bentlybro/synth
After=network.target

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/target/release/synth
Restart=on-failure
RestartSec=5s

# Load environment from .env
EnvironmentFile=$INSTALL_DIR/.env

[Install]
WantedBy=default.target
EOF

# Reload systemd
systemctl --user daemon-reload

# Enable auto-start
systemctl --user enable "$SERVICE_NAME"

success "Systemd service configured"

# Step 8: Check SearXNG
step "Checking SearXNG"

if curl -s http://localhost:8888 > /dev/null 2>&1; then
    success "SearXNG is running on localhost:8888"
else
    warn "SearXNG not detected on localhost:8888"
    echo ""
    echo "Synth requires SearXNG for web search."
    echo ""
    echo "Quick start with Docker:"
    echo "  docker run -d -p 8888:8080 searxng/searxng:latest"
    echo ""
    echo "Or use the docker-compose.yml in the repo:"
    echo "  cd $INSTALL_DIR && docker-compose up -d searxng"
    echo ""
    read -p "Continue without SearXNG? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        error "Installation cancelled. Set up SearXNG and run this installer again."
    fi
fi

# Step 9: Start service
step "Starting Synth"

systemctl --user start "$SERVICE_NAME"

# Wait for startup
sleep 3

# Check status
if systemctl --user is-active --quiet "$SERVICE_NAME"; then
    success "Synth is running!"
else
    error "Failed to start Synth. Check logs with: journalctl --user -u $SERVICE_NAME"
fi

# Step 10: Test installation
step "Testing installation"

if curl -s http://localhost:$PORT/health > /dev/null 2>&1; then
    success "Health check passed"
else
    warn "Health check failed. Service may still be starting..."
fi

# Final summary
echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║${NC}                 ${BLUE}Installation Complete!${NC}                       ${GREEN}║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}Installation Directory:${NC} $INSTALL_DIR"
echo -e "${BLUE}Service Name:${NC} $SERVICE_NAME"
echo -e "${BLUE}API Endpoint:${NC} http://localhost:$PORT"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo ""
echo "1. Check status:"
echo "   systemctl --user status $SERVICE_NAME"
echo ""
echo "2. View logs:"
echo "   journalctl --user -u $SERVICE_NAME -f"
echo ""
echo "3. Test with a query:"
echo "   curl -X POST http://localhost:$PORT/search \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"query\": \"rust programming\", \"max_pages\": 2}'"
echo ""
echo "4. Configure API keys (if not done):"
echo "   nano $ENV_FILE"
echo "   systemctl --user restart $SERVICE_NAME"
echo ""
echo -e "${YELLOW}Documentation:${NC}"
echo "  README: $INSTALL_DIR/README.md"
echo "  Docs:   $INSTALL_DIR/docs/"
echo "  GitHub: https://github.com/Bentlybro/synth"
echo ""
echo -e "${GREEN}Happy researching! 🚀${NC}"
echo ""
