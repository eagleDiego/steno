#!/usr/bin/env bash
#
# build-macos.sh — Build Steno for macOS (.dmg)
#
# Usage: ./build/build-macos.sh [--release|--debug]
#
set -euo pipefail
cd "$(dirname "$0")/.."

BUILD_TYPE="${1:---release}"
PROFILE="release"

if [ "$BUILD_TYPE" = "--debug" ]; then
    PROFILE="debug"
fi

echo "==> Steno macOS Build"
echo "    Profile: ${PROFILE}"

# 1. Install frontend deps if needed
if [ ! -d "node_modules" ]; then
    echo "==> Installing frontend dependencies..."
    npm install
fi

# 2. Build frontend
echo "==> Building frontend..."
npm run build

# 3. Build Tauri app (produces .dmg)
echo "==> Building Tauri app (macOS)..."
npx tauri build --bundles dmg "${BUILD_TYPE}"

# 4. Codesign (optional — requires valid Apple Developer identity)
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
    echo "==> Codesigning .app bundle..."
    codesign --deep --force --verify --verbose \
        --sign "${APPLE_SIGNING_IDENTITY}" \
        src-tauri/target/release/bundle/macos/*.app || true
fi

# 5. Collect artifacts
ARTIFACT_DIR="dist/installers"
mkdir -p "${ARTIFACT_DIR}"

echo "==> Collecting artifacts..."
cp -v src-tauri/target/release/bundle/dmg/*.dmg "${ARTIFACT_DIR}/" 2>/dev/null || true

echo "==> macOS build complete!"
echo "    Artifacts in: ${ARTIFACT_DIR}"
ls -lh "${ARTIFACT_DIR}/" 2>/dev/null || echo "    (no artifacts collected)"
