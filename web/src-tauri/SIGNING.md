# macOS Signing Guide

## Overview

This guide explains how to build and distribute the macOS application for public use.

## Signing Options

### Option 1: No Signing (Free, Worst UX)

Users will see "Unverified Developer" warning and need to:
1. Right-click the app → "Open"
2. Or run: `xattr -d com.apple.quarantine NeoMind.app`

```bash
cargo tauri build --bundles dmg
```

### Option 2: Apple ID Signing (Free, Recommended)

**Best for open source projects.**

1. Create an application-specific password:
   - Go to https://appleid.apple.com
   - Sign in with your Apple ID
   - Security → App-Specific Passwords → Generate
   - Label: "NeoMind Tauri"

2. Sign the app:
```bash
npm run tauri build --bundles dmg

# Sign with your Apple ID
codesign --force --deep --sign "Developer ID Application: Your Name" \
  target/release/bundle/macos/NeoMind.app

# Verify
codesign --verify --verbose target/release/bundle/macos/NeoMind.app
```

### Option 3: Developer Certificate ($99/year, Best UX)

**Best for official releases.**

1. Join [Apple Developer Program](https://developer.apple.com/programs/)
2. Create a "Developer ID Application" certificate
3. Download and install the certificate
4. Update `tauri.conf.json`:
```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Your Name (XXXXXXXXXX)"
    }
  }
}
```

5. Build:
```bash
npm run tauri build --bundles dmg
```

## For GitHub Releases

### Automated Signing with GitHub Actions

Create `.github/workflows/release-macos.yml`:

```yaml
name: macOS Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          cd web
          npm ci

      - name: Build and sign
        env:
          APPLE_CERTIFICATE_BASE64: ${{ secrets.APPLE_CERTIFICATE_BASE64 }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
        run: |
          cd web
          npm run tauri build --bundles dmg

      - name: Upload to GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: web/src-tauri/target/release/bundle/dmg/*.dmg
```

### Manual Release

1. Build the DMG:
```bash
cd web
npm run tauri build --bundles dmg
```

2. Create a GitHub Release:
```bash
gh release create v1.0.0 \
  --title "NeoMind v1.0.0" \
  --notes "Release notes here" \
  web/src-tauri/target/release/bundle/dmg/*.dmg
```

## User Installation Instructions

### Unsigned / Ad-hoc Signed

**Method 1: Right-click**
1. Download `NeoMind-x.x.x.dmg`
2. Open the DMG
3. Right-click `NeoMind.app` → "Open"
4. Click "Open" in the dialog

**Method 2: Terminal**
```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /Applications/NeoMind.app

# Open the app
open /Applications/NeoMind.app
```

### Signed (Developer Certificate)

Just double-click the DMG and drag to Applications!

## Verification

Users can verify the signature:
```bash
codesign -v -v /Applications/NeoMind.app
```

## Common Issues

### "NeoMind.app is damaged and can't be opened"

This means the app was built on a different macOS version. Rebuild on the target OS or use universal binary.

### "App can't be opened because it is from an unidentified developer"

The user needs to:
1. Go to System Settings → Privacy & Security
2. Click "Open Anyway" next to the app message
