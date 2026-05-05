#!/usr/bin/env bash
set -euo pipefail

REPO="l3ocifer/openferris"
BINARY="ferris"
INSTALL_DIR="${FERRIS_INSTALL_DIR:-/usr/local/bin}"

# ── Detect platform ─────────────────────────────────────────────────────

detect_platform() {
    local os arch target

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)   os="unknown-linux-gnu" ;;
        Darwin)  os="apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "On Windows, download the .exe from GitHub Releases:"
            echo "  https://github.com/${REPO}/releases/latest"
            exit 1
            ;;
        *)
            echo "Unsupported OS: $os"
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)
            echo "Unsupported architecture: $arch"
            exit 1
            ;;
    esac

    echo "${arch}-${os}"
}

# ── Fetch latest release tag ────────────────────────────────────────────

latest_tag() {
    if command -v curl &>/dev/null; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/'
    else
        echo "error: curl or wget required" >&2
        exit 1
    fi
}

# ── Download helper ─────────────────────────────────────────────────────

download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    else
        echo "error: curl or wget required" >&2
        return 1
    fi
}

# ── Main ────────────────────────────────────────────────────────────────

main() {
    echo "OpenFerris Installer"
    echo "────────────────────"
    echo

    local target
    target="$(detect_platform)"
    echo "Platform: ${target}"

    local tag
    tag="$(latest_tag)"
    if [ -z "$tag" ]; then
        echo "error: could not determine latest release"
        exit 1
    fi
    echo "Version:  ${tag}"

    local asset="${BINARY}-${target}"
    local url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
    local sums_url="https://github.com/${REPO}/releases/download/${tag}/SHA256SUMS.txt"
    local tmpdir
    tmpdir="$(mktemp -d)"
    local tmpfile="${tmpdir}/${BINARY}"
    local sumsfile="${tmpdir}/SHA256SUMS.txt"

    echo "Downloading ${BINARY}..."
    download "$url" "$tmpfile"
    chmod +x "$tmpfile"

    if download "$sums_url" "$sumsfile" 2>/dev/null; then
        echo "Verifying SHA256 checksum..."
        local expected actual
        expected="$(grep " ${asset}\$" "$sumsfile" | awk '{print $1}')"
        if [ -z "$expected" ]; then
            echo "warning: ${asset} not in SHA256SUMS.txt; skipping verification"
        else
            if command -v sha256sum &>/dev/null; then
                actual="$(sha256sum "$tmpfile" | awk '{print $1}')"
            elif command -v shasum &>/dev/null; then
                actual="$(shasum -a 256 "$tmpfile" | awk '{print $1}')"
            else
                echo "warning: no sha256 tool found; skipping verification"
                actual="$expected"
            fi
            if [ "$expected" != "$actual" ]; then
                echo "error: checksum mismatch"
                echo "  expected: $expected"
                echo "  actual:   $actual"
                rm -rf "$tmpdir"
                exit 1
            fi
            echo "Checksum OK."
        fi
    else
        echo "warning: SHA256SUMS.txt not available; skipping verification"
    fi

    echo "Installing to ${INSTALL_DIR}/${BINARY}..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "$tmpfile" "${INSTALL_DIR}/${BINARY}"
    else
        sudo mv "$tmpfile" "${INSTALL_DIR}/${BINARY}"
    fi

    rm -rf "$tmpdir"

    echo
    echo "Installed ${BINARY} ${tag} to ${INSTALL_DIR}/${BINARY}"
    echo
    echo "Get started:"
    echo "  ferris start"
    echo
}

main "$@"
