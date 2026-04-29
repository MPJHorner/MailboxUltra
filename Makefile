# MailBoxUltra -- common developer tasks.
# Run `make help` (or just `make`) for the list.

.DEFAULT_GOAL := help

# Resolve cargo from PATH, then fall back to the standard rustup install
# location so `make run` works in shells that haven't sourced ~/.cargo/env.
CARGO ?= $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)
BIN   ?= mailbox-ultra

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Targets:\n"} /^[a-zA-Z_-]+:.*##/ {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: run
run: ## Run in dev mode (cargo run, no release optimisations)
	$(CARGO) run

.PHONY: run-release
run-release: release ## Run the release binary (faster startup)
	./target/release/$(BIN)

.PHONY: build
build: ## Debug build
	$(CARGO) build

.PHONY: release
release: ## Optimised release build
	$(CARGO) build --release

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
check: lint test ## Lint + test -- full pre-commit gate

# Coverage exclusions: GUI rendering is hard to drive deterministically
# from a unit test runner; we still cover the protocol + storage core.
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
clean: ## Remove build artifacts
	$(CARGO) clean
	rm -f lcov.info

# ---- macOS bundling ----

.PHONY: icon
icon: ## Rasterise icon/icon.svg, then compile icon/AppIcon.icns
	$(CARGO) run --bin icon-gen --features icon-tool
	iconutil -c icns icon/AppIcon.iconset -o icon/AppIcon.icns

.PHONY: app
app: ## Build MailBoxUltra.app for the host architecture
	./mac/build-app.sh

.PHONY: app-x86
app-x86: ## Build MailBoxUltra.app for Intel Macs
	./mac/build-app.sh x86_64-apple-darwin

.PHONY: app-arm
app-arm: ## Build MailBoxUltra.app for Apple Silicon
	./mac/build-app.sh aarch64-apple-darwin

.PHONY: app-universal
app-universal: ## Build a universal MailBoxUltra.app (Intel + Apple Silicon)
	./mac/build-app-universal.sh

.PHONY: dmg
dmg: ## Package the host-arch .app into a DMG
	./mac/build-dmg.sh

.PHONY: dmg-arm
dmg-arm: ## Package the Apple Silicon .app into a DMG
	./mac/build-dmg.sh aarch64-apple-darwin

.PHONY: dmg-x86
dmg-x86: ## Package the Intel .app into a DMG
	./mac/build-dmg.sh x86_64-apple-darwin
