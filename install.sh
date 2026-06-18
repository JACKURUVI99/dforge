#!/usr/bin/env bash
set -euo pipefail

REPO="harishannavisamy/dforge"
BINARY="dforge"
INSTALL_DIR="/usr/local/bin"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}"
echo "  ██████╗ ███████╗ ██████╗ ██████╗  ██████╗ ███████╗"
echo "  ██╔══██╗██╔════╝██╔═══██╗██╔══██╗██╔════╝ ██╔════╝"
echo "  ██║  ██║█████╗  ██║   ██║██████╔╝██║  ███╗█████╗  "
echo "  ██║  ██║██╔══╝  ██║   ██║██╔══██╗██║   ██║██╔══╝  "
echo "  ██████╔╝███████╗╚██████╔╝██║  ██║╚██████╔╝███████╗"
echo "  ╚═════╝ ╚══════╝ ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝"
echo -e "${NC}"
echo "  Serverless Git on IPFS + Blockchain"
echo ""

detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)       echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64)  echo "amd64" ;;
        aarch64|arm64) echo "arm64" ;;
        *) echo "unknown" ;;
    esac
}

OS=$(detect_os)
ARCH=$(detect_arch)

echo -e "${YELLOW}→ Detected: ${OS}/${ARCH}${NC}"

# Try apt install first on Debian/Ubuntu
if [ "$OS" = "linux" ] && command -v apt-get &>/dev/null; then
    echo -e "${YELLOW}→ Trying .deb install...${NC}"
    LATEST=$(curl -sf "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/')
    if [ -n "$LATEST" ]; then
        DEB_URL="https://github.com/${REPO}/releases/download/v${LATEST}/dforge_${LATEST}_amd64.deb"
        TMP=$(mktemp -d)
        curl -fsSL "$DEB_URL" -o "$TMP/dforge.deb"
        sudo dpkg -i "$TMP/dforge.deb" && echo -e "${GREEN}✓ Installed via .deb${NC}" && rm -rf "$TMP" && dforge --version && exit 0
        rm -rf "$TMP"
    fi
fi

# Try snap
if command -v snap &>/dev/null; then
    echo -e "${YELLOW}→ Trying snap install...${NC}"
    sudo snap install dforge --classic && echo -e "${GREEN}✓ Installed via snap${NC}" && dforge --version && exit 0
fi

# Fallback: download binary directly
echo -e "${YELLOW}→ Downloading binary from GitHub releases...${NC}"
LATEST=$(curl -sf "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/')

if [ -z "$LATEST" ]; then
    echo -e "${RED}✗ Could not determine latest version. Check https://github.com/${REPO}/releases${NC}"
    exit 1
fi

if [ "$OS" = "linux" ]; then
    URL="https://github.com/${REPO}/releases/download/v${LATEST}/dforge-linux-amd64"
elif [ "$OS" = "macos" ]; then
    URL="https://github.com/${REPO}/releases/download/v${LATEST}/dforge-macos-universal"
else
    echo -e "${RED}✗ Unsupported OS: ${OS}${NC}"
    echo "  Build from source: cargo install --git https://github.com/${REPO} dforge"
    exit 1
fi

TMP=$(mktemp -d)
curl -fsSL "$URL" -o "$TMP/dforge"
chmod +x "$TMP/dforge"

if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP/dforge" "$INSTALL_DIR/dforge"
else
    sudo mv "$TMP/dforge" "$INSTALL_DIR/dforge"
fi
rm -rf "$TMP"

echo -e "${GREEN}✓ dforge ${LATEST} installed to ${INSTALL_DIR}/dforge${NC}"
echo ""
dforge --version
echo ""
echo "  Get started:"
echo "    dforge init my-project"
echo "    cd my-project && dforge commit -m 'first commit'"
echo "    dforge push"
echo ""
echo "  Explore public repos:"
echo "    dforge explore --list"
