.PHONY: all build release test clean install uninstall package aur-test desktop-validate

# Default target
all: build

# Build the project in debug mode
build:
	cargo build

# Build the project in release mode
release:
	cargo build --release

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean
	rm -f *.pkg.tar.zst

# Install the application system-wide (requires root)
install: release
	install -Dm755 target/release/proton_game_saves /usr/bin/protongamesaves
	install -Dm644 protongamesaves.desktop /usr/share/applications/protongamesaves.desktop
	install -Dm644 README.md /usr/share/doc/protongamesaves/README.md
	install -Dm644 LICENSE /usr/share/licenses/protongamesaves/LICENSE

# Uninstall the application
uninstall:
	rm -f /usr/bin/protongamesaves
	rm -f /usr/share/applications/protongamesaves.desktop
	rm -rf /usr/share/doc/protongamesaves
	rm -rf /usr/share/licenses/protongamesaves

# Build Arch package
package:
	makepkg -sf

# Test AUR package build
aur-test:
	makepkg -si

# Update package checksums
checksums:
	updpkgsums

# Update .SRCINFO file
srcinfo:
	makepkg --printsrcinfo > .SRCINFO

# Validate desktop file
desktop-validate:
	desktop-file-validate protongamesaves.desktop

# Lint the code
lint:
	cargo clippy -- -D warnings

# Format the code
format:
	cargo fmt

# Check if code is properly formatted
format-check:
	cargo fmt -- --check

# Run all checks before submission
check: format-check lint test desktop-validate

# Prepare for AUR submission
aur-prepare: checksums srcinfo check

# Show help
help:
	@echo "Available targets:"
	@echo "  build          - Build in debug mode"
	@echo "  release        - Build in release mode"
	@echo "  test           - Run tests"
	@echo "  clean          - Clean build artifacts"
	@echo "  install        - Install system-wide (requires root)"
	@echo "  uninstall      - Remove from system"
	@echo "  package        - Build Arch package"
	@echo "  aur-test       - Test AUR package build"
	@echo "  checksums      - Update package checksums"
	@echo "  srcinfo        - Update .SRCINFO file"
	@echo "  desktop-validate - Validate desktop file"
	@echo "  lint           - Run clippy linter"
	@echo "  format         - Format code"
	@echo "  format-check   - Check if code is formatted"
	@echo "  check          - Run all checks"
	@echo "  aur-prepare    - Prepare for AUR submission"
	@echo "  help           - Show this help"
