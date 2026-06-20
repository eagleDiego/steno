#!/usr/bin/env bash
#
# smoke-test.sh — Verify that a built Steno installer works correctly
#
# This script should be run on a clean machine (CI runner) and tests:
#   1. The installer exits successfully
#   2. The installed binary exists and is executable
#   3. The app can start and respond to basic CLI commands
#
# Usage: ./build/smoke-test.sh <path-to-installer>
#
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <path-to-installer>"
    echo ""
    echo "Examples:"
    echo "  $0 dist/installers/steno_0.1.0_amd64.deb"
    echo "  $0 dist/installers/Steno_0.1.0_x64-setup.exe"
    echo "  $0 dist/installers/Steno_0.1.0_x64.dmg"
    exit 1
fi

INSTALLER="$1"
PASS=0
FAIL=0

check() {
    local desc="$1"
    shift
    if "$@" >/tmp/smoke-test.log 2>&1; then
        echo "  PASS: ${desc}"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: ${desc}"
        echo "    Log: $(tail -5 /tmp/smoke-test.log | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

echo "==> Steno Smoke Test"
echo "    Installer: ${INSTALLER}"
echo ""

# --- Phase 1: Installer exists and is valid ---
echo "--- Phase 1: Installer validation ---"

check "Installer file exists"       test -f "${INSTALLER}"
check "Installer is non-empty"      test -s "${INSTALLER}"

EXT="${INSTALLER##*.}"

case "${EXT}" in
    deb)
        check "Installer is valid .deb"   dpkg-deb --info "${INSTALLER}" >/dev/null 2>&1
        ;;
    AppImage)
        check "Installer is valid AppImage"  test -x "${INSTALLER}" && file "${INSTALLER}" | grep -q "ELF"
        ;;
    exe)
        check "Installer is valid .exe"   file "${INSTALLER}" | grep -qiE "PE32|executable"
        ;;
    dmg)
        check "Installer is valid .dmg"   file "${INSTALLER}" | grep -qi "disk image"
        ;;
    rpm)
        check "Installer is valid .rpm"   rpm -K "${INSTALLER}" >/dev/null 2>&1 || true
        ;;
    *)
        echo "  WARN: Unknown extension '${EXT}' — skipping format validation"
        ;;
esac

# --- Phase 2: Install ---
echo ""
echo "--- Phase 2: Installation ---"

INSTALL_LOG=$(mktemp)

case "${EXT}" in
    deb)
        check "Install .deb package"  sudo dpkg -i "${INSTALLER}" >"${INSTALL_LOG}" 2>&1
        check "Binary exists"         which steno-app || which steno
        ;;
    AppImage)
        # AppImage is self-contained, no installation needed
        chmod +x "${INSTALLER}"
        check "AppImage is executable" test -x "${INSTALLER}"
        ;;
    exe)
        # Windows: run installer silently
        INSTALL_DIR="${PROGRAMFILES:-C:/Program Files}/Steno"
        check "Run NSIS installer"    "${INSTALLER}" /S >"${INSTALL_LOG}" 2>&1 || true
        check "Binary exists"         test -f "${INSTALL_DIR}/Steno.exe"
        ;;
    dmg)
        # macOS: mount DMG and check app
        MOUNT_POINT=$(mktemp -d)
        check "Mount .dmg"            hdiutil attach "${INSTALLER}" -mountpoint "${MOUNT_POINT}" >/dev/null 2>&1
        check "App bundle exists"     test -d "${MOUNT_POINT}/Steno.app"
        hdiutil detach "${MOUNT_POINT}" >/dev/null 2>&1 || true
        ;;
esac

# --- Phase 3: Basic launch verification ---
echo ""
echo "--- Phase 3: Launch verification ---"

case "${EXT}" in
    deb)
        # Run in headless mode if supported, or just verify binary exits cleanly
        APP_BIN=$(which steno-app 2>/dev/null || which steno 2>/dev/null || echo "")
        if [ -n "${APP_BIN}" ]; then
            check "Binary is executable" test -x "${APP_BIN}"
            check "Binary outputs version" ${APP_BIN} --version >/dev/null 2>&1 || true
        fi
        ;;
    AppImage)
        # Run the AppImage with --help to verify it launches
        check "AppImage launches" timeout 5 "${INSTALLER}" --help >/dev/null 2>&1 || true
        ;;
esac

# --- Results ---
echo ""
echo "==========================================="
echo " Smoke Test Results"
echo "   Passed: ${PASS}"
echo "   Failed: ${FAIL}"
echo "==========================================="

if [ "${FAIL}" -gt 0 ]; then
    echo "FAIL: ${FAIL} check(s) failed"
    exit 1
else
    echo "PASS: All checks passed"
    exit 0
fi
