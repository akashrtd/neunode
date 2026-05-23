#!/usr/bin/env sh
set -e

# agnetd install script
# Usage: curl -fsSL https://get.neunode.dev | sh
#   or:  curl -fsSL https://get.neunode.dev | sh -s -- --install-dir /usr/local/bin

INSTALL_DIR="${AGNETD_INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_REPO="akashrtd/neunode"

# --- Platform detection ---
detect_platform() {
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  ARCH="$(uname -m)"

  case "$OS" in
    linux)  OS="linux" ;;
    darwin) OS="darwin" ;;
    *)
      echo "Error: Unsupported OS: $OS"
      exit 1
      ;;
  esac

  case "$ARCH" in
    x86_64|amd64) ARCH="x64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *)
      echo "Error: Unsupported architecture: $ARCH"
      exit 1
      ;;
  esac

  TARGET="${OS}-${ARCH}"
}

# --- Parse CLI flags ---
parse_args() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --install-dir)
        INSTALL_DIR="$2"
        shift 2
        ;;
      *)
        echo "Unknown option: $1"
        exit 1
        ;;
    esac
  done
}

# --- Determine version ---
get_version() {
  VERSION="${AGNETD_VERSION:-}"
  if [ -z "$VERSION" ]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" \
      2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')
  fi
  if [ -z "$VERSION" ]; then
    echo "Error: Could not determine latest version"
    echo "  Set AGNETD_VERSION explicitly, e.g.: AGNETD_VERSION=v0.1.0"
    exit 1
  fi
}

# --- Download binary ---
download_binary() {
  DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/agnetd-${TARGET}"
  TMPFILE="$(mktemp)"
  echo "Downloading agnetd ${VERSION} for ${TARGET}..."

  HTTP_CODE=$(curl -fsSL -w "%{http_code}" -o "$TMPFILE" "$DOWNLOAD_URL" 2>/dev/null) || true

  if [ "$HTTP_CODE" != "200" ]; then
    echo "Error: Download failed (HTTP ${HTTP_CODE})"
    echo "  URL: ${DOWNLOAD_URL}"
    rm -f "$TMPFILE"
    exit 1
  fi

  chmod +x "$TMPFILE"
}

# --- Install ---
install_binary() {
  mkdir -p "$INSTALL_DIR"
  mv "$TMPFILE" "${INSTALL_DIR}/agnetd"
  echo "Installed agnetd to ${INSTALL_DIR}/agnetd"
}

# --- PATH hint ---
hint_path() {
  case ":$PATH:" in
    *":$INSTALL_DIR:"*)
      ;;
    *)
      echo ""
      echo "NOTE: ${INSTALL_DIR} is not in your PATH."
      echo "  Add it with:  export PATH=\"${INSTALL_DIR}:\$PATH\""
      echo "  Or add this line to your ~/.bashrc or ~/.zshrc"
      ;;
  esac
}

# --- Main ---
parse_args "$@"
detect_platform
get_version
download_binary
install_binary
hint_path
