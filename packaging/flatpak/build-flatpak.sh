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

# Clean previous builds and remote
echo "Cleaning previous builds and remote..."
rm -rf "$BUILD_DIR" "$REPO_DIR"

# Remove existing remote if it exists (ignore errors)
echo "Removing existing remote repository..."
flatpak --user remote-delete proton-game-saves-repo 2>/dev/null || true

# Build the Flatpak
echo "Building Flatpak..."
if ! flatpak-builder "$BUILD_DIR" "$MANIFEST_FILE" --force-clean --ccache; then
    echo "Error: Failed to build Flatpak"
    exit 1
fi

# Install locally for testing
echo "Creating local repository..."
if ! flatpak-builder --repo="$REPO_DIR" "$BUILD_DIR" "$MANIFEST_FILE" --force-clean; then
    echo "Error: Failed to create Flatpak repository"
    exit 1
fi

echo "Adding local repository..."
if ! flatpak --user remote-add --if-not-exists --no-gpg-verify proton-game-saves-repo "$REPO_DIR"; then
    echo "Error: Failed to add remote repository"
    echo "Repository directory contents:"
    ls -la "$REPO_DIR"
    exit 1
fi

echo "Installing from local repository..."
if ! flatpak --user install proton-game-saves-repo "$APP_ID" -y; then
    echo "Error: Failed to install Flatpak"
    exit 1
fi

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
