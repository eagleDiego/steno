# Steno — Meeting Audio Capture

Cross-platform desktop app for automatic meeting audio capture, detection, and relay.

## Architecture

```
steno-app (Tauri 2.x desktop app)
  └── steno-core (Rust library)
        ├── audio/        — Audio capture backends (mic + system audio)
        ├── detection/    — Meeting/activity detection engine
        ├── events/       — Typed event bus (tokio broadcast)
        ├── config/       — TOML configuration management
        └── ui/           — Tauri commands + system tray
```

## Building

### Prerequisites

- **Rust** 1.75+ (stable)
- **Node.js** 20+ and npm

#### Linux
```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf libssl-dev
```

#### macOS
- **Xcode Command Line Tools**: `xcode-select --install`
- **Node.js** (20+): Download from https://nodejs.org or via `brew install node`
- **Rust**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

> **Note**: Tauri 2.x uses the system WebKit (WKWebView) — no additional frameworks needed.
> The macOS build produces a `.dmg` with minimum target macOS 14.0.

#### Windows
Install Microsoft Visual Studio C++ Build Tools and WebView2 (included with Windows 10+).

### Build installers

```bash
# All platforms (auto-detects platform)
make build

# Specific platform
./build/build-linux.sh
./build/build-macos.sh
./build/build-windows.sh
```

Output artifacts in `src-tauri/target/release/bundle/`:
- Linux: `.AppImage`, `.deb`
|macOS: `.dmg` (macOS 14.0+)
- Windows: `.exe` (NSIS)

### Run tests

```bash
cargo test                 # Rust unit + integration tests
cargo test -p steno-core   # Library tests only
```

## CI/CD

See [.github/workflows/build.yml](.github/workflows/build.yml) — runs on push/PR to `main`:
1. **lint-and-test** — format check, clippy, unit tests on all platforms
2. **build-installers** — produces platform installers and runs smoke tests
3. **summary** — aggregates results

Release workflow at [.github/workflows/release.yml](.github/workflows/release.yml) — tags `v*.*.*` produce GitHub Releases with attached installers.

## Smoke tests

```bash
./build/smoke-test.sh <path-to-installer>
```
Validates the installer file, runs installation, and verifies basic launch.

## License

Copyright (c) 2026 NERD.aero. All rights reserved.
