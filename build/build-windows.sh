#!/usr/bin/env bash
#
# build-windows.sh — Build Steno for Windows (.exe via NSIS)
#
# Usage: ./build/build-windows.sh [--release|--debug]
#
set -euo pipefail
cd "$(dirname "$0")/.."

BUILD_TYPE="${1:---release}"
PROFILE="release"

if [ "$BUILD_TYPE" = "--debug" ]; then
    PROFILE="debug"
fi

echo "==> Steno Windows Build"
echo "    Profile: ${PROFILE}"

echo "    (Running on Windows — cross-compilation not natively supported from Linux)"

# 1. Install frontend deps if needed
if [ ! -d "node_modules" ]; then
    echo "==> Installing frontend dependencies..."
    npm install
fi

# 2. Build frontend
echo "==> Building frontend..."
npm run build

# 3. Build Tauri app (produces .exe via NSIS)
echo "==> Building Tauri app (Windows)..."
npx tauri build --bundles nsis "${BUILD_TYPE}"

# 4. Collect artifacts
ARTIFACT_DIR="dist/installers"
mkdir -p "${ARTIFACT_DIR}"

echo "==> Collecting artifacts..."
# Tauri NSIS output goes to target/release/bundle/nsis/
cp -v src-tauri/target/release/bundle/nsis/Steno_*.exe "${ARTIFACT_DIR}/" 2>/dev/null || true
cp -v src-tauri/target/release/bundle/msi/Steno_*.msi "${ARTIFACT_DIR}/" 2>/dev/null || true
# Also check for wix (older Tauri convention)
cp -v src-tauri/target/release/bundle/wix/*.msi "${ARTIFACT_DIR}/" 2>/dev/null || true

echo "==> Windows build complete!"
echo "    Artifacts in: ${ARTIFACT_DIR}"
ls -lh "${ARTIFACT_DIR}/" 2>/dev/null || echo "    (no artifacts collected)"
