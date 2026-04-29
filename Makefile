# MailBox Ultra — common developer tasks.
# Run `make` (or `make help`) for the full list.

.DEFAULT_GOAL := help

# Resolve cargo from PATH, then fall back to the standard rustup install
# location so `make run` works in shells that haven't sourced ~/.cargo/env.
CARGO       ?= $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)
BIN         ?= mailbox-ultra
HOST_TRIPLE := $(shell rustc -vV | awk '/host:/ { print $$2 }')
HOST_APP    := target/$(HOST_TRIPLE)/release/MailBoxUltra.app

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Targets:\n"} /^[a-zA-Z0-9_-]+:.*##/ {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ---- iteration ----

.PHONY: run
run: ## Run via cargo (debug build, no .app shell). Fastest iteration.
	$(CARGO) run

.PHONY: launch
launch: app ## Build the .app and open it (the real Mac install experience)
	open $(HOST_APP)

.PHONY: build
build: ## Debug build
	$(CARGO) build

.PHONY: release
release: ## Optimised release build (bare binary, no .app)
	$(CARGO) build --release

# ---- gates ----

.PHONY: test
test: ## Run unit + integration tests
	$(CARGO) test --all-features

.PHONY: test-watch
test-watch: ## Re-run tests on file changes (requires cargo-watch)
	$(CARGO) watch -x 'test --all-features'

.PHONY: fmt
fmt: ## Format code
	$(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## Verify formatting (CI parity)
	$(CARGO) fmt --all -- --check

.PHONY: clippy
clippy: ## Run clippy with -D warnings
	$(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: lint
lint: fmt-check clippy ## fmt-check + clippy (what CI runs)

.PHONY: check
check: lint test ## Lint + test — full pre-commit gate

# Coverage exclusions: GUI rendering is hard to drive deterministically from
# a unit test runner; we still cover the protocol + storage + lifecycle core.
COVERAGE_IGNORE := src/(main|gui)/.*

.PHONY: coverage
coverage: ## Line coverage summary via cargo-llvm-cov
	$(CARGO) llvm-cov --lib --tests \
		--ignore-filename-regex='$(COVERAGE_IGNORE)' \
		--summary-only

.PHONY: coverage-html
coverage-html: ## HTML coverage report at target/llvm-cov/html/index.html
	$(CARGO) llvm-cov --lib --tests \
		--ignore-filename-regex='$(COVERAGE_IGNORE)' \
		--html

.PHONY: clean
clean: ## Remove build artifacts (preserves icon/AppIcon.icns)
	$(CARGO) clean
	rm -f lcov.info

# ---- macOS bundling ----

# File-target so `make icon` is a no-op when icon.svg hasn't changed.
icon/AppIcon.icns: icon/icon.svg
	$(CARGO) run --bin icon-gen --features icon-tool
	iconutil -c icns icon/AppIcon.iconset -o $@

.PHONY: icon
icon: icon/AppIcon.icns ## Rasterise icon/icon.svg → icon/AppIcon.icns (incremental)

.PHONY: app
app: icon/AppIcon.icns ## Build MailBoxUltra.app for the host architecture
	./mac/build-app.sh

.PHONY: app-arm
app-arm: icon/AppIcon.icns ## Build MailBoxUltra.app for Apple Silicon
	./mac/build-app.sh aarch64-apple-darwin

.PHONY: app-x86
app-x86: icon/AppIcon.icns ## Build MailBoxUltra.app for Intel Macs
	./mac/build-app.sh x86_64-apple-darwin

.PHONY: app-universal
app-universal: icon/AppIcon.icns ## Universal MailBoxUltra.app (Intel + Apple Silicon)
	./mac/build-app-universal.sh

.PHONY: dmg
dmg: app ## Package the host-arch .app into a DMG (auto-builds .app if needed)
	./mac/build-dmg.sh

.PHONY: dmg-arm
dmg-arm: app-arm ## Package the Apple Silicon .app into a DMG
	./mac/build-dmg.sh aarch64-apple-darwin

.PHONY: dmg-x86
dmg-x86: app-x86 ## Package the Intel .app into a DMG
	./mac/build-dmg.sh x86_64-apple-darwin

.PHONY: dmg-universal
dmg-universal: app-universal ## Universal DMG (Intel + Apple Silicon, signed if APPLE_CERT_ID is set)
	@echo "(universal DMG is produced by build-app-universal.sh in $(dir $(HOST_APP)).." \
	  "/universal-apple-darwin/release/)"

# ---- dev: simulate inbound mail ----

.PHONY: simulate
simulate: ## Fire every realistic scenario at the running app once
	./scripts/simulate.py

.PHONY: simulate-list
simulate-list: ## List the scenarios in scripts/simulate.py
	./scripts/simulate.py --list

.PHONY: simulate-burst
simulate-burst: ## Fire 200 throwaway messages to test the ring buffer
	./scripts/simulate.py burst -n 200

.PHONY: simulate-all
simulate-all: ## Run every scenario including burst
	./scripts/simulate.py --all
