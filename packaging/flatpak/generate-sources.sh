#!/bin/bash

# Script to generate Flatpak cargo sources
# This script downloads the flatpak-cargo-generator and generates the sources file

set -e

echo "Generating Flatpak cargo sources..."

# Check if python3 is available
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 is required but not found"
    exit 1
fi

# Create virtual environment and install siphash if not available
if ! python3 -c "import siphash" 2>/dev/null; then
    echo "Setting up virtual environment and installing siphash..."
    if [ ! -d "venv" ]; then
        python3 -m venv venv
    fi
    source venv/bin/activate
    echo "Installing required Python packages..."
    pip install siphash aiohttp tomlkit
    echo "Using virtual environment for Python dependencies..."
else
    echo "siphash already available system-wide"
fi

# Download flatpak-cargo-generator if not present
if [ ! -f "flatpak-cargo-generator.py" ]; then
    echo "Downloading flatpak-cargo-generator..."
    curl -o flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
    chmod +x flatpak-cargo-generator.py
fi

# Generate sources from Cargo.lock
echo "Generating sources from Cargo.lock..."

# Use virtual environment python if it exists, otherwise use system python3
if [ -d "venv" ] && [ -f "venv/bin/python" ]; then
    source venv/bin/activate
    python ./flatpak-cargo-generator.py ../../Cargo.lock -o generated-sources.json
else
    python3 ./flatpak-cargo-generator.py ../../Cargo.lock -o generated-sources.json
fi

echo "Generated sources saved to generated-sources.json"
echo "You can now build the Flatpak using: flatpak-builder build io.github.rahatzamancse.ProtonGameSaves.json"
