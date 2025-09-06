#!/bin/bash

# Script to build and test ProtonGameSaves Flatpak locally
# Based on the guide from https://belmoussaoui.com/blog/8-how-to-flatpak-a-rust-application/

set -e

APP_ID="io.github.rahatzamancse.ProtonGameSaves"
MANIFEST_FILE="$APP_ID.json"
BUILD_DIR="build"
REPO_DIR="repo"

echo "Building ProtonGameSaves Flatpak..."

# Check if flatpak-builder is available
if ! command -v flatpak-builder &> /dev/null; then
    echo "Error: flatpak-builder is required but not found"
    echo "Please install flatpak-builder: sudo dnf install flatpak-builder  # or equivalent for your distro"
    exit 1
fi

# Generate sources if not present
if [ ! -f "generated-sources.json" ]; then
    echo "Generating cargo sources..."
    ./generate-sources.sh
fi

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf "$BUILD_DIR" "$REPO_DIR"

# Build the Flatpak
echo "Building Flatpak..."
flatpak-builder "$BUILD_DIR" "$MANIFEST_FILE" --force-clean --ccache

# Install locally for testing
echo "Installing Flatpak locally..."
flatpak-builder --repo="$REPO_DIR" "$BUILD_DIR" "$MANIFEST_FILE" --force-clean
flatpak --user remote-add --if-not-exists --no-gpg-verify proton-game-saves-repo "$REPO_DIR"
flatpak --user install proton-game-saves-repo "$APP_ID" -y

echo ""
echo "✅ Flatpak built and installed successfully!"
echo ""
echo "To run the application:"
echo "  flatpak run $APP_ID"
echo ""
echo "To uninstall:"
echo "  flatpak --user uninstall $APP_ID"
echo ""
echo "To remove the repo:"
echo "  flatpak --user remote-delete proton-game-saves-repo"
