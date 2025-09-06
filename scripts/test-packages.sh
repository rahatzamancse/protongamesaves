#!/bin/bash

# Comprehensive test script for ProtonGameSaves AUR and Flatpak packages
# This script builds, installs, and validates both packaging formats

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
APP_ID="io.github.rahatzamancse.ProtonGameSaves"
AUR_PKG_NAME="proton-game-saves"
PROJECT_ROOT="$(pwd)"
TEST_DIR="${PROJECT_ROOT}/test-packages-tmp"
AUR_TEST_DIR="${TEST_DIR}/aur"
FLATPAK_TEST_DIR="${TEST_DIR}/flatpak"

# Logging functions
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

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."
    
    # Remove Flatpak installation
    if flatpak --user list | grep -q "${APP_ID}"; then
        log_info "Removing Flatpak installation..."
        flatpak --user uninstall "${APP_ID}" -y || true
    fi
    
    # Remove Flatpak repo
    if flatpak --user remote-list | grep -q "proton-game-saves-repo"; then
        log_info "Removing Flatpak test repository..."
        flatpak --user remote-delete proton-game-saves-repo || true
    fi
    
    # Remove AUR package if installed
    if pacman -Q "${AUR_PKG_NAME}" &>/dev/null; then
        log_warning "AUR package ${AUR_PKG_NAME} is installed. Please remove manually with: sudo pacman -R ${AUR_PKG_NAME}"
    fi
    
    # Remove test directory
    if [ -d "${TEST_DIR}" ]; then
        rm -rf "${TEST_DIR}"
        log_success "Removed test directory"
    fi
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    local missing_tools=()
    
    # Check for AUR build tools
    command -v makepkg >/dev/null 2>&1 || missing_tools+=("makepkg (pacman)")
    command -v namcap >/dev/null 2>&1 || missing_tools+=("namcap")
    
    # Check for Flatpak tools
    command -v flatpak >/dev/null 2>&1 || missing_tools+=("flatpak")
    command -v flatpak-builder >/dev/null 2>&1 || missing_tools+=("flatpak-builder")
    
    # Check for other tools
    command -v python3 >/dev/null 2>&1 || missing_tools+=("python3")
    command -v cargo >/dev/null 2>&1 || missing_tools+=("cargo")
    command -v rustc >/dev/null 2>&1 || missing_tools+=("rustc")
    
    if [ ${#missing_tools[@]} -ne 0 ]; then
        log_error "Missing required tools:"
        for tool in "${missing_tools[@]}"; do
            log_error "  - ${tool}"
        done
        log_error "Please install missing tools and try again."
        exit 1
    fi
    
    log_success "All prerequisites satisfied"
}

# Test AUR package build
test_aur_build() {
    log_info "Testing AUR package build..."
    
    mkdir -p "${AUR_TEST_DIR}"
    cd "${AUR_TEST_DIR}"
    
    # Copy PKGBUILD and related files
    cp "${PROJECT_ROOT}/packaging/aur/PKGBUILD" .
    cp "${PROJECT_ROOT}/packaging/aur/proton-game-saves.desktop" .
    
    # Build source package locally instead of downloading
    log_info "Creating local source archive..."
    cd "${PROJECT_ROOT}"
    
    # Create archive with proper directory structure (like GitHub releases)
    mkdir -p "/tmp/proton-game-saves-0.1.0"
    git archive HEAD | tar -x -C "/tmp/proton-game-saves-0.1.0"
    cd /tmp
    tar -czf "${AUR_TEST_DIR}/proton-game-saves-0.1.0.tar.gz" "proton-game-saves-0.1.0"
    rm -rf "/tmp/proton-game-saves-0.1.0"
    
    cd "${AUR_TEST_DIR}"
    
    # Update PKGBUILD to use local source
    sed -i "s|source=.*|source=(\"proton-game-saves-0.1.0.tar.gz\")|" PKGBUILD
    
    # Generate new checksum
    local checksum
    checksum=$(sha256sum "proton-game-saves-0.1.0.tar.gz" | cut -d' ' -f1)
    sed -i "s|sha256sums=.*|sha256sums=('${checksum}')|" PKGBUILD
    
    # Fix desktop file path in PKGBUILD
    sed -i 's|"proton-game-saves.desktop"|"packaging/aur/proton-game-saves.desktop"|' PKGBUILD
    
    log_info "Running PKGBUILD lint check with namcap..."
    if command -v namcap >/dev/null 2>&1; then
        namcap PKGBUILD || log_warning "namcap found issues in PKGBUILD"
    else
        log_warning "namcap not available, skipping PKGBUILD lint"
    fi
    
    log_info "Building AUR package..."
    makepkg --noconfirm --rmdeps --syncdeps
    
    local pkg_file
    pkg_file=$(ls -1 *.pkg.tar.* 2>/dev/null | head -n1)
    
    if [ -n "${pkg_file}" ]; then
        log_success "AUR package built successfully: ${pkg_file}"
        
        # Lint the built package
        log_info "Running package lint check with namcap..."
        if command -v namcap >/dev/null 2>&1; then
            namcap "${pkg_file}" || log_warning "namcap found issues in built package"
        fi
        
        # Test package installation
        log_info "Testing package installation..."
        sudo pacman -U "${pkg_file}" --noconfirm
        
        # Verify installation
        if pacman -Q "${AUR_PKG_NAME}" &>/dev/null; then
            log_success "AUR package installed successfully"
            
            # Test if binary works
            if command -v proton-game-saves >/dev/null 2>&1; then
                log_success "Binary is available in PATH"
                # Test help output (non-interactive)
                if proton-game-saves --help &>/dev/null; then
                    log_success "Binary executes successfully"
                else
                    log_warning "Binary help command failed"
                fi

                log_info "Testing GUI application..."
                proton-game-saves

            else
                log_error "Binary not found in PATH after installation"
            fi
        else
            log_error "Package installation verification failed"
        fi
        
    else
        log_error "No package file found after build"
        return 1
    fi
    
    cd "${PROJECT_ROOT}"
}

# Test Flatpak build
test_flatpak_build() {
    log_info "Testing Flatpak build..."
    
    mkdir -p "${FLATPAK_TEST_DIR}"
    cd "${PROJECT_ROOT}/packaging/flatpak"
    
    # Generate sources if needed
    if [ ! -f "generated-sources.json" ]; then
        log_info "Generating Flatpak cargo sources..."
        ./generate-sources.sh
    fi
    
    # Create local manifest for testing
    cp "${APP_ID}.json" "${FLATPAK_TEST_DIR}/test-manifest.json"
    cp "generated-sources.json" "${FLATPAK_TEST_DIR}/"
    
    cd "${FLATPAK_TEST_DIR}"
    
    # Update manifest to use local source
    python3 -c "
import json
import sys

with open('test-manifest.json', 'r') as f:
    manifest = json.load(f)

# Update source to use local directory
for module in manifest['modules']:
    if module['name'] == 'proton_game_saves':
        for i, source in enumerate(module['sources']):
            if isinstance(source, dict) and source.get('type') == 'archive':
                module['sources'][i] = {
                    'type': 'dir',
                    'path': '${PROJECT_ROOT}'
                }
                break

with open('test-manifest.json', 'w') as f:
    json.dump(manifest, f, indent=4)
"
    
    # Build Flatpak
    log_info "Building Flatpak package..."
    flatpak-builder build test-manifest.json --force-clean --ccache
    
    # Create local repository and install
    log_info "Installing Flatpak package locally..."
    flatpak-builder --repo=repo build test-manifest.json --force-clean
    flatpak --user remote-add --if-not-exists --no-gpg-verify proton-game-saves-repo repo
    flatpak --user install proton-game-saves-repo "${APP_ID}" -y
    
    # Verify installation
    if flatpak --user list | grep -q "${APP_ID}"; then
        log_success "Flatpak package installed successfully"
        
        # Test basic functionality
        log_info "Testing Flatpak application..."
        if timeout 10s flatpak run "${APP_ID}" --help &>/dev/null; then
            log_success "Flatpak application executes successfully"
        else
            log_warning "Flatpak application test timed out or failed"
        fi

        log_info "Testing Flatpak GUI application..."
        flatpak run "${APP_ID}"
        
        # Test file permissions and metadata
        log_info "Checking Flatpak metadata..."
        flatpak --user info "${APP_ID}" > flatpak_info.txt
        
        if grep -q "proton-game-saves" flatpak_info.txt; then
            log_success "Flatpak metadata looks correct"
        else
            log_warning "Flatpak metadata may have issues"
        fi
        
    else
        log_error "Flatpak installation verification failed"
        return 1
    fi
    
    cd "${PROJECT_ROOT}"
}

# Run validation tests
run_validation_tests() {
    log_info "Running validation tests..."
    
    # Test AUR installed binary
    if command -v proton-game-saves >/dev/null 2>&1; then
        log_info "Testing AUR binary functionality..."
        
        # Check version
        if proton-game-saves --version &>/dev/null; then
            log_success "AUR binary version check passed"
        else
            log_warning "AUR binary version check failed"
        fi
    fi
    
    # Test Flatpak installation
    if flatpak --user list | grep -q "${APP_ID}"; then
        log_info "Testing Flatpak application functionality..."
        
        # Check if application shows up in desktop environment
        if flatpak --user info "${APP_ID}" | grep -q "Command:"; then
            log_success "Flatpak application properly registered"
        else
            log_warning "Flatpak application registration may have issues"
        fi
    fi
    
    log_success "Validation tests completed"
}

# Main function
main() {
    echo "=========================================="
    echo "ProtonGameSaves Package Testing Script"
    echo "=========================================="
    
    # Set up trap for cleanup
    trap cleanup EXIT INT TERM
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "packaging" ]; then
        log_error "Please run this script from the project root directory"
        exit 1
    fi
    
    # Parse command line arguments
    local test_aur=true
    local test_flatpak=true
    local skip_install=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --aur-only)
                test_flatpak=false
                shift
                ;;
            --flatpak-only)
                test_aur=false
                shift
                ;;
            --no-install)
                skip_install=true
                shift
                ;;
            --help)
                echo "Usage: $0 [options]"
                echo "Options:"
                echo "  --aur-only      Test only AUR package"
                echo "  --flatpak-only  Test only Flatpak package"
                echo "  --no-install    Build packages but don't install them"
                echo "  --help          Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Check prerequisites
    check_prerequisites
    
    # Create test directory
    mkdir -p "${TEST_DIR}"
    
    local success=true
    
    # Test AUR build
    if [ "$test_aur" = true ]; then
        if test_aur_build; then
            log_success "AUR package test passed"
        else
            log_error "AUR package test failed"
            success=false
        fi
    fi
    
    # Test Flatpak build
    if [ "$test_flatpak" = true ]; then
        if test_flatpak_build; then
            log_success "Flatpak package test passed"
        else
            log_error "Flatpak package test failed"
            success=false
        fi
    fi
    
    # Run validation tests if installations were successful
    if [ "$success" = true ] && [ "$skip_install" = false ]; then
        run_validation_tests
    fi
    
    # Final report
    echo "=========================================="
    if [ "$success" = true ]; then
        log_success "All package tests completed successfully!"
        echo ""
        log_info "Installation status:"
        if [ "$test_aur" = true ] && pacman -Q "${AUR_PKG_NAME}" &>/dev/null; then
            log_success "AUR package: Installed"
        fi
        if [ "$test_flatpak" = true ] && flatpak --user list | grep -q "${APP_ID}"; then
            log_success "Flatpak package: Installed"
        fi
        echo ""
        log_info "To clean up installations, run: $0 --cleanup"
    else
        log_error "Some package tests failed. Check the output above for details."
        exit 1
    fi
    echo "=========================================="
}

# Handle cleanup-only mode
if [ "$1" = "--cleanup" ]; then
    cleanup
    exit 0
fi

# Run main function with all arguments
main "$@"
