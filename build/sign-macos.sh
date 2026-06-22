#!/usr/bin/env bash
#
# sign-macos.sh — Codesign and notarize a macOS .app/.dmg for distribution
#
# This script is designed to run on a macOS CI runner (macos-latest)
# AFTER `npm run tauri build -- --bundles dmg` has produced the DMG.
#
# Required environment variables (set as GitHub Secrets):
#
#   APPLE_SIGNING_IDENTITY     Developer ID Application certificate name
#                              e.g. "Developer ID Application: Your Name (TEAM123)"
#
#   APPLE_NOTARIZATION_EMAIL   Apple ID email for notarytool
#   APPLE_NOTARIZATION_PASS    App-specific password for notarytool
#   APPLE_TEAM_ID              Team ID (10-char alphanumeric from dev account)
#
#   Alternatively, use an App Store Connect API key:
#   APPLE_ASC_ISSUER_ID        Issuer ID (UUID)
#   APPLE_ASC_KEY_ID           Key ID
#   APPLE_ASC_KEY              Path to the .p8 API key file
#
# Usage: ./build/sign-macos.sh [--dmg <path-to-dmg>]
#
# If --dmg is not provided, the script searches for a .dmg in the
# standard Tauri macOS bundle output directory.
#

set -euo pipefail

cd "$(dirname "$0")/.."

# ── Config ────────────────────────────────────────────────────────────────
APP_NAME="Steno"
SEARCH_DIR="src-tauri/target/release/bundle/dmg"

# ── Locate the DMG ────────────────────────────────────────────────────────
DMG_PATH=""
if [ "$#" -ge 2 ] && [ "$1" = "--dmg" ]; then
    DMG_PATH="$2"
elif [ -d "$SEARCH_DIR" ]; then
    DMG_PATH=$(ls -t "$SEARCH_DIR"/*.dmg 2>/dev/null | head -1)
fi

if [ -z "$DMG_PATH" ] || [ ! -f "$DMG_PATH" ]; then
    echo "ERROR: No .dmg found. Pass --dmg <path> or ensure Tauri build has run."
    echo "  Searched: $SEARCH_DIR/*.dmg"
    exit 1
fi

echo "==> Steno macOS Signing & Notarization"
echo "    DMG: ${DMG_PATH}"

# ── Check requirements ────────────────────────────────────────────────────
REQUIRED_TOOLS=(codesign spctl xcrun)
for tool in "${REQUIRED_TOOLS[@]}"; do
    if ! command -v "$tool" &>/dev/null; then
        echo "ERROR: Required macOS tool '$tool' not found — are you on macOS?"
        exit 2
    fi
done

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
    echo "WARN: APPLE_SIGNING_IDENTITY not set — will check for ad-hoc signing or skip"
fi

# ── Step 1: Mount the DMG and extract the .app ───────────────────────────
echo ""
echo "--- Step 1: Extract .app from DMG ---"

MOUNT_POINT=$(mktemp -d)
hdiutil attach "$DMG_PATH" -mountpoint "$MOUNT_POINT" -nobrowse -quiet

APP_BUNDLE=""
for candidate in "$MOUNT_POINT"/*.app; do
    if [ -d "$candidate" ]; then
        APP_BUNDLE="$candidate"
        break
    fi
done

if [ -z "$APP_BUNDLE" ]; then
    hdiutil detach "$MOUNT_POINT" -quiet 2>/dev/null || true
    echo "ERROR: No .app bundle found in DMG at $MOUNT_POINT"
    exit 3
fi

echo "    App bundle: ${APP_BUNDLE}"

# Copy .app to a temporary location for signing (avoids permission issues on mounted volume)
TEMP_APP_DIR=$(mktemp -d)
cp -R "$APP_BUNDLE" "$TEMP_APP_DIR/"
APP_BUNDLE_COPY="${TEMP_APP_DIR}/$(basename "$APP_BUNDLE")"

# Detach DMG now that we have a copy
hdiutil detach "$MOUNT_POINT" -quiet

# ── Step 2: Codesign the .app bundle ─────────────────────────────────────
echo ""
echo "--- Step 2: Codesign .app bundle ---"

SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"

echo "    Identity: ${SIGNING_IDENTITY}"

# Deep sign: signs all nested executables, frameworks, and libraries
# --force: re-sign even if already signed
# --verify: verify after signing
# --options runtime: enable Hardened Runtime (required for notarization)
# --timestamp: include a secure timestamp
codesign --deep --force --verify --verbose=4 \
    --options runtime \
    --timestamp \
    --sign "${SIGNING_IDENTITY}" \
    "$APP_BUNDLE_COPY"

# Also sign the DMG itself (optional but good practice)
echo ""
echo "--- Step 2b: Sign DMG ---"
codesign --force --verify --verbose=4 \
    --sign "${SIGNING_IDENTITY}" \
    "$DMG_PATH" 2>/dev/null || echo "    (DMG signing skipped — not all macOS versions support signing disk images)"

# ── Step 3: Verify the signature ──────────────────────────────────────────
echo ""
echo "--- Step 3: Verify code signature ---"

codesign -dv --verbose=4 "$APP_BUNDLE_COPY" 2>&1 || {
    echo "FAIL: codesign verification failed"
    exit 4
}

echo ""
echo "    Code signature verified OK"

# ── Step 4: Create new DMG from the signed .app ──────────────────────────
echo ""
echo "--- Step 4: Re-create DMG with signed .app ---"

DMG_DIR=$(dirname "$DMG_PATH")
DMG_BASENAME=$(basename "$DMG_PATH")
SIGNED_DMG="${DMG_DIR}/signed_${DMG_BASENAME}"

# Remove any existing signed DMG
rm -f "$SIGNED_DMG"

# Create a temporary directory with the app for DMG creation
STAGING_DIR=$(mktemp -d)
cp -R "$APP_BUNDLE_COPY" "$STAGING_DIR/"

# Create a symlink to /Applications for drag-and-drop installer experience
ln -s /Applications "$STAGING_DIR/Applications" 2>/dev/null || true

# Re-create the DMG
hdiutil create -volname "${APP_NAME}" \
    -srcfolder "$STAGING_DIR" \
    -ov -format UDZO \
    -fs HFS+ \
    "$SIGNED_DMG"

# Clean up staging
rm -rf "$STAGING_DIR"
rm -rf "$TEMP_APP_DIR"

echo "    Signed DMG: ${SIGNED_DMG}"

# ── Step 5: Notarization ────────────────────────────────────────────────
echo ""
echo "--- Step 5: Notarization ---"

# Check available credential options
if [ -n "${APPLE_ASC_ISSUER_ID:-}" ] && [ -n "${APPLE_ASC_KEY_ID:-}" ] && [ -n "${APPLE_ASC_KEY:-}" ]; then
    # App Store Connect API key approach (preferred for CI)
    echo "    Using App Store Connect API key for notarization..."
    AUTH_ARGS=("--issuer" "$APPLE_ASC_ISSUER_ID" "--key" "$APPLE_ASC_KEY_ID" "--key-file" "$APPLE_ASC_KEY")
elif [ -n "${APPLE_NOTARIZATION_EMAIL:-}" ] && [ -n "${APPLE_NOTARIZATION_PASS:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
    # Apple ID + app-specific password approach
    echo "    Using Apple ID credentials for notarization..."
    # Store credentials in keychain for notarytool
    xcrun notarytool store-credentials "AC_PASSWORD" \
        --apple-id "$APPLE_NOTARIZATION_EMAIL" \
        --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_NOTARIZATION_PASS" \
        --validate 2>&1 || {
        echo "WARN: Failed to store notarization credentials (may already exist)"
    }
    AUTH_ARGS=("--keychain-profile" "AC_PASSWORD")
else
    echo "WARN: No notarization credentials found — skipping notarization."
    echo "    Set either:"
    echo "      - APPLE_ASC_ISSUER_ID + APPLE_ASC_KEY_ID + APPLE_ASC_KEY  (API key, preferred)"
    echo "      - APPLE_NOTARIZATION_EMAIL + APPLE_NOTARIZATION_PASS + APPLE_TEAM_ID  (Apple ID)"
    echo ""
    echo "    The signed DMG is ready at: ${SIGNED_DMG}"
    echo "    You can notarize manually: xcrun notarytool submit \"${SIGNED_DMG}\" ... --wait"
    # Copy signed DMG back to original location for manual use
    cp "$SIGNED_DMG" "$DMG_PATH"
    echo "    (signed DMG copied to ${DMG_PATH})"
    exit 0
fi

xcrun notarytool submit "$SIGNED_DMG" \
    "${AUTH_ARGS[@]}" \
    --wait \
    --output-format json 2>&1 | tee /tmp/notary-output.json

# Check notarization result
NOTARY_STATUS=$(python3 -c "
import json
with open('/tmp/notary-output.json') as f:
    data = json.load(f)
    # could be at different key paths depending on version
    for key in ['status', 'message']:
        if key in data:
            print(data[key])
            break
    else:
        print('unknown')
" 2>/dev/null || echo "unknown")

if echo "$NOTARY_STATUS" | grep -qiE "accepted|success"; then
    echo ""
    echo "    NOTARIZATION ACCEPTED"
else
    echo ""
    echo "    Notarization result: ${NOTARY_STATUS}"
    echo "    Check logs above for details. The signed DMG is at: ${SIGNED_DMG}"
    # Copy signed DMG back to original location regardless
    cp "$SIGNED_DMG" "$DMG_PATH"
    echo "    (signed DMG copied to ${DMG_PATH})"
    exit 0
fi

# ── Step 6: Staple the notarization ticket ───────────────────────────────
echo ""
echo "--- Step 6: Staple notarization ticket ---"

# First, mount the notarized DMG to staple the .app inside
NOTARY_MOUNT=$(mktemp -d)
hdiutil attach "$SIGNED_DMG" -mountpoint "$NOTARY_MOUNT" -nobrowse -quiet

for candidate in "$NOTARY_MOUNT"/*.app; do
    if [ -d "$candidate" ]; then
        echo "    Stapling: ${candidate}"
        xcrun stapler staple "$candidate" 2>&1
        # Also staple the DMG itself
        xcrun stapler staple "$SIGNED_DMG" 2>&1 || true
        break
    fi
done

hdiutil detach "$NOTARY_MOUNT" -quiet

# Verify staple directly on the DMG
echo ""
echo "--- Step 6b: Verify staple on DMG ---"
xcrun stapler validate "$SIGNED_DMG" 2>&1 && echo "    DMG STAPLE VALID" || echo "    No staple on DMG"

# ── Step 7: Gatekeeper verification ──────────────────────────────────────
echo ""
echo "--- Step 7: Gatekeeper assessment ---"

# Need to mount to assess the .app
GK_MOUNT=$(mktemp -d)
hdiutil attach "$SIGNED_DMG" -mountpoint "$GK_MOUNT" -nobrowse -quiet

for candidate in "$GK_MOUNT"/*.app; do
    if [ -d "$candidate" ]; then
        spctl --assess -vvvv "$candidate" 2>&1 || true
        break
    fi
done

hdiutil detach "$GK_MOUNT" -quiet

# ── Done ──────────────────────────────────────────────────────────────────
echo ""
echo "==========================================="
echo "  macOS Signing & Notarization Complete"
echo "  Signed DMG: ${SIGNED_DMG}"
echo "==========================================="

# Copy the signed+notarized DMG over the original
cp "$SIGNED_DMG" "$DMG_PATH"
echo ""
echo "  Notarized DMG copied to: ${DMG_PATH}"

rm -f /tmp/notary-output.json