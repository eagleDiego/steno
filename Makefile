.PHONY: all build build-linux build-macos build-windows deps dev clean smoke-test

all: build

# ── Build shortcuts ──────────────────────────────────────────────

build:
	./build/build-all.sh

build-linux:
	./build/build-linux.sh

build-macos:
	./build/build-macos.sh

build-windows:
	./build/build-windows.sh

# ── Dev ──────────────────────────────────────────────────────────

deps:
	npm install

dev:
	npm run dev

# ── CI helpers ───────────────────────────────────────────────────

smoke-test:
	@echo "Usage: make smoke-test INSTALLER=path/to/installer"
	./build/smoke-test.sh $(INSTALLER)

# ── Clean ────────────────────────────────────────────────────────

clean:
	rm -rf node_modules dist
	cd src-tauri && cargo clean
	cd .. && cargo clean 2>/dev/null || true

.PHONY: test
test:
	cd .. && cargo test 2>/dev/null || cargo test
