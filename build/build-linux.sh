#!/usr/bin/env bash
#
# build-linux.sh — Build Steno for Linux (AppImage + .deb)
#
# Usage: ./build/build-linux.sh [--release|--debug]
#
set -euo pipefail
cd "$(dirname "$0")/.."

BUILD_TYPE="${1:---release}"
PROFILE="release"

if [ "$BUILD_TYPE" = "--debug" ]; then
    PROFILE="debug"
fi

echo "==> Steno Linux Build"
echo "    Profile: ${PROFILE}"

# 1. Install frontend deps if needed
if [ ! -d "node_modules" ]; then
    echo "==> Installing frontend dependencies..."
    npm install
fi

# 2. Build frontend
echo "==> Building frontend..."
npm run build

# 3. Build Tauri app (produces .AppImage + .deb)
echo "==> Building Tauri app (Linux)..."
npx tauri build --bundles appimage,deb "${BUILD_TYPE}"

# 4. Collect artifacts
ARTIFACT_DIR="dist/installers"
mkdir -p "${ARTIFACT_DIR}"

echo "==> Collecting artifacts..."
if [ -d "src-tauri/target/release/bundle" ]; then
    cp -v src-tauri/target/release/bundle/appimage/*.AppImage "${ARTIFACT_DIR}/" 2>/dev/null || true
    cp -v src-tauri/target/release/bundle/deb/*.deb "${ARTIFACT_DIR}/" 2>/dev/null || true
fi

echo "==> Linux build complete!"
echo "    Artifacts in: ${ARTIFACT_DIR}"
ls -lh "${ARTIFACT_DIR}/" 2>/dev/null || echo "    (no artifacts collected)"
