#!/bin/bash

# Script to get SHA256 checksum for ProtonGameSaves source tarball
# Usage: ./get-flatpak-checksum.sh <version>
# Example: ./get-flatpak-checksum.sh v0.1.0

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if version argument is provided
if [ $# -eq 0 ]; then
    log_error "No version specified!"
    echo "Usage: $0 <version>"
    echo "Example: $0 v0.1.0"
    exit 1
fi

VERSION="$1"
REPO_URL="https://github.com/rahatzamancse/ProtonGameSaves"
TARBALL_NAME="ProtonGameSaves-${VERSION}.tar.gz"
DOWNLOAD_URL="${REPO_URL}/archive/${VERSION}.tar.gz"

log_info "Getting checksum for ProtonGameSaves ${VERSION}"
log_info "Repository: ${REPO_URL}"
log_info "Download URL: ${DOWNLOAD_URL}"

# Check if curl is available
if ! command -v curl &> /dev/null; then
    log_error "curl is required but not found"
    exit 1
fi

# Check if sha256sum is available
if ! command -v sha256sum &> /dev/null; then
    log_error "sha256sum is required but not found"
    exit 1
fi

# Download the tarball
log_info "Downloading ${TARBALL_NAME}..."
if curl -L -f -o "${TARBALL_NAME}" "${DOWNLOAD_URL}"; then
    log_success "Download completed"
else
    log_error "Failed to download ${TARBALL_NAME}"
    log_error "Please check if version ${VERSION} exists in the repository"
    exit 1
fi

# Calculate checksum
log_info "Calculating SHA256 checksum..."
CHECKSUM=$(sha256sum "${TARBALL_NAME}" | cut -d' ' -f1)

# Display results
echo
echo "========================================="
echo "  ProtonGameSaves ${VERSION} Checksum"
echo "========================================="
echo "Version:  ${VERSION}"
echo "File:     ${TARBALL_NAME}"
echo "URL:      ${DOWNLOAD_URL}"
echo "SHA256:   ${CHECKSUM}"
echo "========================================="
echo

log_success "Checksum: ${CHECKSUM}"

# Clean up
log_info "Cleaning up downloaded file..."
rm "${TARBALL_NAME}"
log_success "Cleanup completed"

log_info "You can now use this checksum in your Flatpak manifest:"
echo "  \"sha256\": \"${CHECKSUM}\""
