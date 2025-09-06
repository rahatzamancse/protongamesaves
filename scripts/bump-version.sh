#!/bin/bash
set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

NEW_VERSION="$1"
OLD_VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

echo "🔄 Bumping version from $OLD_VERSION to $NEW_VERSION..."

# Update Cargo.toml
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Update PKGBUILD
sed -i "s/^pkgver=.*/pkgver=$NEW_VERSION/" packaging/aur/PKGBUILD

# Update AppStream (add new release, keep old ones)
TODAY=$(date +%Y-%m-%d)
sed -i "s/<release version=\"$OLD_VERSION\"/<release version=\"$NEW_VERSION\" date=\"$TODAY\">\n      <description>\n        <p>Version $NEW_VERSION release</p>\n      </description>\n    </release>\n    <release version=\"$OLD_VERSION\"/" resources/io.github.rahatzamancse.ProtonGameSaves.metainfo.xml

# Update About dialog
sed -i "s/\.version(\".*\")/\.version(\"$NEW_VERSION\")/" src/ui/window.rs

# Update Cargo.lock
cargo check

echo "✅ Version bumped to $NEW_VERSION"
echo "📝 Next steps:"
echo "   1. Update Flatpak manifest URL/sha256 manually (or use External Data Checker)"
echo "   2. git add -A && git commit -m 'Bump version to $NEW_VERSION'"
echo "   3. git tag v$NEW_VERSION && git push origin v$NEW_VERSION"
echo "   4. Create GitHub release (triggers AUR auto-update)"