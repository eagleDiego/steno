#!/usr/bin/env bash
#
# build-all.sh — Detect platform and run the appropriate build script
#
# Usage: ./build/build-all.sh [--release|--debug]
#
set -euo pipefail

BUILD_TYPE="${1:---release}"
cd "$(dirname "$0")/.."

case "$(uname -s)" in
    Linux)
        echo "==> Detected Linux — building AppImage + .deb"
        ./build/build-linux.sh "${BUILD_TYPE}"
        ;;
    Darwin)
        echo "==> Detected macOS — building .dmg"
        ./build/build-macos.sh "${BUILD_TYPE}"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "==> Detected Windows — building .exe"
        ./build/build-windows.sh "${BUILD_TYPE}"
        ;;
    *)
        echo "==> Unknown platform: $(uname -s)"
        echo "    Build scripts exist for each platform individually:"
        echo "      ./build/build-linux.sh"
        echo "      ./build/build-macos.sh"
        echo "      ./build/build-windows.sh"
        exit 1
        ;;
esac
