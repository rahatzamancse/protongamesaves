#!/bin/bash
# Manual AUR package update script
# Usage: ./packaging/aur/update-aur.sh <version>

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.1"
    exit 1
fi

VERSION="$1"
REPO_URL="https://github.com/rahatzamancse/proton-game-saves"
TARBALL_URL="${REPO_URL}/archive/v${VERSION}.tar.gz"

echo "🔄 Updating AUR package to version ${VERSION}..."

# Check if we're in the right directory
if [ -f "packaging/aur/PKGBUILD" ]; then
    # We're in the project root
    PKGBUILD_PATH="$(pwd)/packaging/aur/PKGBUILD"
elif [ -f "PKGBUILD" ]; then
    # We're in the packaging/aur directory
    PKGBUILD_PATH="$(pwd)/PKGBUILD"
else
    echo "❌ Error: PKGBUILD not found. Run this script from the project root or from packaging/aur/"
    exit 1
fi

# Create temporary directory for AUR operations
TMP_DIR=$(mktemp -d)
AUR_DIR="${TMP_DIR}/proton-game-saves"

# Download and calculate checksum
echo "📥 Downloading release tarball..."
wget -O "${TMP_DIR}/release.tar.gz" "${TARBALL_URL}"
SHA256=$(sha256sum "${TMP_DIR}/release.tar.gz" | cut -d' ' -f1)
echo "✅ SHA256: ${SHA256}"

# Clone AUR repository
echo "📦 Cloning AUR repository..."
git clone ssh://aur@aur.archlinux.org/proton-game-saves.git "${AUR_DIR}"
cd "${AUR_DIR}"

# Copy local PKGBUILD to AUR repository
echo "📋 Copying local PKGBUILD..."
cp "${PKGBUILD_PATH}" PKGBUILD

# Update PKGBUILD
echo "📝 Updating PKGBUILD..."
sed -i "s/^pkgver=.*/pkgver=${VERSION}/" PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD
sed -i "s/^sha256sums=.*/sha256sums=('${SHA256}')/" PKGBUILD

# Generate .SRCINFO
echo "📋 Generating .SRCINFO..."
makepkg --printsrcinfo > .SRCINFO

# Show changes
echo "📋 Changes to be committed:"
git diff

# Ask for confirmation
if [ -n "$CI" ]; then
    echo "CI detected, auto-confirming commit and push."
else
    read -p "🤔 Do you want to commit and push these changes? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "❌ Aborted by user"
        rm -rf "${TMP_DIR}"
        exit 1
    fi
fi

# Commit and push
echo "🚀 Committing and pushing to AUR..."
git add PKGBUILD .SRCINFO
git commit -m "Update to version ${VERSION}

- Updated pkgver to ${VERSION}
- Updated sha256sums
- Reset pkgrel to 1"

git push origin master

# Cleanup
rm -rf "${TMP_DIR}"

echo "✅ AUR package successfully updated to version ${VERSION}!"
echo "📦 Users can now install with: paru -S proton-game-saves"
echo "🔗 View on AUR: https://aur.archlinux.org/packages/proton-game-saves"
