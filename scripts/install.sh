#!/bin/sh
# aipkg installation script
# Compatible with bash, zsh, and fish (when run with sh or bash)
#
# Usage: sh install.sh [--prerelease]
#   --prerelease: Include prerelease versions (default: only stable releases)

set -e

REPO="kleeedolinux/aipkg"
API_URL="https://api.github.com/repos/${REPO}/releases"
INSTALL_DIR="/usr/local/bin"
TEMP_DIR=$(mktemp -d)

# Cleanup function
cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

# Check for required commands
check_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Error: $1 is required but not installed." >&2
        exit 1
    fi
}

# Determine download command
get_download_cmd() {
    if command -v curl >/dev/null 2>&1; then
        echo "curl"
    elif command -v wget >/dev/null 2>&1; then
        echo "wget"
    else
        echo "Error: Neither curl nor wget is installed." >&2
        exit 1
    fi
}

# Download file
download_file() {
    local url="$1"
    local output="$2"
    local cmd=$(get_download_cmd)
    
    if [ "$cmd" = "curl" ]; then
        curl -fsSL -o "$output" "$url"
    else
        wget -q -O "$output" "$url"
    fi
}

# Parse JSON (simple grep/sed approach if jq is not available)
parse_json() {
    local json="$1"
    local key="$2"
    
    if command -v jq >/dev/null 2>&1; then
        echo "$json" | jq -r "$key"
    else
        # Fallback: simple grep/sed parsing (works for simple cases)
        case "$key" in
            ".[0].tag_name")
                echo "$json" | grep -o '"tag_name":"[^"]*"' | head -1 | sed 's/"tag_name":"\([^"]*\)"/\1/'
                ;;
            ".[0].assets[0].browser_download_url")
                echo "$json" | grep -o '"browser_download_url":"[^"]*\.tar\.gz"' | head -1 | sed 's/"browser_download_url":"\([^"]*\)"/\1/'
                ;;
            ".[0].assets[1].browser_download_url")
                echo "$json" | grep -o '"browser_download_url":"[^"]*\.sha256"' | head -1 | sed 's/"browser_download_url":"\([^"]*\)"/\1/'
                ;;
            ".[0].prerelease")
                echo "$json" | grep -o '"prerelease":[^,}]*' | head -1 | grep -q "true" && echo "true" || echo "false"
                ;;
            ".[] | select(.prerelease == false) | .tag_name")
                # Find first non-prerelease
                echo "$json" | grep -B 20 '"prerelease":false' | grep '"tag_name"' | head -1 | sed 's/.*"tag_name":"\([^"]*\)".*/\1/'
                ;;
            ".[] | select(.prerelease == false) | .assets[0].browser_download_url")
                # Find tar.gz for first non-prerelease
                echo "$json" | grep -B 50 '"prerelease":false' | grep '"browser_download_url".*\.tar\.gz' | head -1 | sed 's/.*"browser_download_url":"\([^"]*\)".*/\1/'
                ;;
            *)
                echo "Error: Unsupported JSON key for fallback parser: $key" >&2
                exit 1
                ;;
        esac
    fi
}

# Check prerequisites
check_command tar

# Parse arguments
INCLUDE_PRERELEASE=false
if [ "$1" = "--prerelease" ]; then
    INCLUDE_PRERELEASE=true
fi

echo "Fetching latest release information..."

# Fetch releases from GitHub API
download_file "$API_URL" "$TEMP_DIR/releases.json"
RELEASES_CONTENT=$(cat "$TEMP_DIR/releases.json")

# Find the appropriate release
if [ "$INCLUDE_PRERELEASE" = "true" ]; then
    TAG_NAME=$(parse_json "$RELEASES_CONTENT" ".[0].tag_name")
    DOWNLOAD_URL=$(parse_json "$RELEASES_CONTENT" ".[0].assets[0].browser_download_url")
    IS_PRERELEASE=$(parse_json "$RELEASES_CONTENT" ".[0].prerelease")
else
    # Find first non-prerelease
    TAG_NAME=$(parse_json "$RELEASES_CONTENT" ".[] | select(.prerelease == false) | .tag_name")
    if [ -z "$TAG_NAME" ]; then
        echo "Warning: No stable release found, falling back to latest (may be prerelease)..." >&2
        TAG_NAME=$(parse_json "$RELEASES_CONTENT" ".[0].tag_name")
        DOWNLOAD_URL=$(parse_json "$RELEASES_CONTENT" ".[0].assets[0].browser_download_url")
    else
        DOWNLOAD_URL=$(parse_json "$RELEASES_CONTENT" ".[] | select(.prerelease == false) | .assets[0].browser_download_url")
    fi
fi

if [ -z "$TAG_NAME" ] || [ -z "$DOWNLOAD_URL" ]; then
    echo "Error: Failed to find release information." >&2
    exit 1
fi

echo "Found release: $TAG_NAME"
echo "Downloading: $DOWNLOAD_URL"

# Download the archive
ARCHIVE_NAME=$(basename "$DOWNLOAD_URL")
ARCHIVE_PATH="$TEMP_DIR/$ARCHIVE_NAME"
download_file "$DOWNLOAD_URL" "$ARCHIVE_PATH"

# Verify checksum if available
SHA256_URL=$(parse_json "$RELEASES_CONTENT" ".[0].assets[1].browser_download_url")
if [ -n "$SHA256_URL" ] && echo "$SHA256_URL" | grep -q "\.sha256$"; then
    echo "Verifying checksum..."
    SHA256_FILE="$TEMP_DIR/$(basename "$SHA256_URL")"
    download_file "$SHA256_URL" "$SHA256_FILE"
    
    if command -v sha256sum >/dev/null 2>&1; then
        EXPECTED_CHECKSUM=$(cat "$SHA256_FILE" | awk '{print $1}')
        ACTUAL_CHECKSUM=$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')
        
        if [ "$EXPECTED_CHECKSUM" != "$ACTUAL_CHECKSUM" ]; then
            echo "Error: Checksum verification failed!" >&2
            echo "Expected: $EXPECTED_CHECKSUM" >&2
            echo "Actual:   $ACTUAL_CHECKSUM" >&2
            exit 1
        fi
        echo "Checksum verified."
    else
        echo "Warning: sha256sum not found, skipping checksum verification." >&2
    fi
fi

# Extract archive
echo "Extracting archive..."
cd "$TEMP_DIR"
tar -xzf "$ARCHIVE_NAME"

# Check if binary exists
if [ ! -f "$TEMP_DIR/aipkg" ]; then
    echo "Error: Binary 'aipkg' not found in archive." >&2
    exit 1
fi

# Make binary executable
chmod +x "$TEMP_DIR/aipkg"

# Check for existing installation
EXISTING_BINARY=""
OLD_VERSION=""
if command -v aipkg >/dev/null 2>&1; then
    EXISTING_BINARY=$(command -v aipkg)
    OLD_VERSION=$(aipkg --version 2>/dev/null || echo "unknown")
    echo "Found existing installation:"
    echo "  Location: $EXISTING_BINARY"
    echo "  Version: $OLD_VERSION"
    echo "Replacing with new version..."
elif [ -f "$INSTALL_DIR/aipkg" ]; then
    EXISTING_BINARY="$INSTALL_DIR/aipkg"
    if [ -x "$EXISTING_BINARY" ]; then
        OLD_VERSION=$("$EXISTING_BINARY" --version 2>/dev/null || echo "unknown")
    fi
    echo "Found existing binary at $INSTALL_DIR/aipkg"
    if [ -n "$OLD_VERSION" ] && [ "$OLD_VERSION" != "unknown" ]; then
        echo "  Version: $OLD_VERSION"
    fi
    echo "Replacing with new version..."
fi

# Install to system (will overwrite if exists)
echo "Installing to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    mv -f "$TEMP_DIR/aipkg" "$INSTALL_DIR/aipkg"
else
    echo "Requiring sudo to install to $INSTALL_DIR..."
    sudo mv -f "$TEMP_DIR/aipkg" "$INSTALL_DIR/aipkg"
fi

# Verify installation
if command -v aipkg >/dev/null 2>&1; then
    INSTALLED_VERSION=$(aipkg --version 2>/dev/null || echo "unknown")
    echo ""
    if [ -n "$EXISTING_BINARY" ]; then
        echo "✓ aipkg upgraded successfully!"
        if [ -n "$OLD_VERSION" ] && [ "$OLD_VERSION" != "unknown" ]; then
            echo "  Previous version: $OLD_VERSION"
        fi
        echo "  New version: $INSTALLED_VERSION"
    else
        echo "✓ aipkg installed successfully!"
        echo "  Version: $INSTALLED_VERSION"
    fi
    echo "  Location: $(command -v aipkg)"
else
    echo ""
    if [ -n "$EXISTING_BINARY" ]; then
        echo "✓ aipkg upgraded successfully!"
        if [ -n "$OLD_VERSION" ] && [ "$OLD_VERSION" != "unknown" ]; then
            echo "  Previous version: $OLD_VERSION"
        fi
    else
        echo "✓ aipkg installed successfully!"
    fi
    echo "  Location: $INSTALL_DIR/aipkg"
    echo "  Note: Make sure $INSTALL_DIR is in your PATH"
fi

