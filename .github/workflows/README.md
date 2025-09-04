# GitHub Actions

This directory contains automated workflows for the project.

## Available Workflows

### `update-aur.yml` (Recommended)
Primary workflow that updates the AUR package when a new release is created. Uses Ubuntu with arch-install-scripts.

### `update-aur-alternative.yml` (Alternative)
Alternative workflow using an Arch Linux container. More reliable for makepkg operations but may be slower.

## Setup Instructions

To enable AUR automation, you need to:

1. **Generate SSH Key Pair for AUR:**
   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/aur_rsa
   ```

2. **Add Public Key to AUR:**
   - Go to [AUR Account Settings](https://aur.archlinux.org/account/)
   - Add the contents of `~/.ssh/aur_rsa.pub` to your SSH Public Keys

3. **Add Private Key to Repository Secrets:**
   - Go to your GitHub repository Settings → Secrets and variables → Actions
   - Create a new secret named `AUR_SSH_PRIVATE_KEY`
   - Paste the contents of `~/.ssh/aur_rsa` (the private key)

4. **Create Initial AUR Package:**
   ```bash
   # Clone AUR repository (replace username)
   git clone ssh://aur@aur.archlinux.org/proton-game-saves.git
   cd proton-game-saves
   
   # Copy PKGBUILD and generate .SRCINFO
   cp /path/to/your/project/PKGBUILD .
   makepkg --printsrcinfo > .SRCINFO
   
   # Initial commit
   git add PKGBUILD .SRCINFO
   git commit -m "Initial import"
   git push origin master
   ```

## How It Works

1. **Trigger:** Workflow runs when a new release is published on GitHub
2. **Version Update:** Automatically updates `pkgver` in PKGBUILD
3. **Checksum Calculation:** Downloads the release tarball and calculates SHA256
4. **Update Files:** Updates PKGBUILD and regenerates .SRCINFO
5. **Push to AUR:** Commits and pushes changes to the AUR repository

## Workflow Features

- ✅ Automatic version detection from release tags
- ✅ SHA256 checksum calculation
- ✅ PKGBUILD validation before pushing
- ✅ Automatic .SRCINFO generation
- ✅ Descriptive commit messages
- ✅ Error handling and validation
- ✅ Optional GitHub comment notifications

## Troubleshooting

If the workflow fails:

1. **Check SSH Key:** Ensure the private key is correctly added to GitHub secrets
2. **Check AUR Access:** Verify your AUR account has the public key registered
3. **Check PKGBUILD:** Ensure the PKGBUILD is valid and follows AUR guidelines
4. **Check Repository:** Make sure the AUR repository exists and you have push access

The alternative Docker workflow (`update-aur-alternative.yml`) can be used if the primary workflow has issues with makepkg dependencies.
