# MailBoxUltra -- common developer tasks.
# Run `make help` (or just `make`) for the list.

.DEFAULT_GOAL := help

# Resolve cargo from PATH, then fall back to the standard rustup install
# location so `make run` works in shells that haven't sourced ~/.cargo/env.
CARGO ?= $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)
BIN   ?= mailbox-ultra

# Override on the CLI: `make run SMTP_PORT=2525 UI_PORT=8888`.
SMTP_PORT ?=
UI_PORT   ?=
RUN_ARGS  ?=

SMTP_PORT_FLAG := $(if $(SMTP_PORT),-s $(SMTP_PORT),)
UI_PORT_FLAG   := $(if $(UI_PORT),-u $(UI_PORT),)

.PHONY: help
help: ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Targets:\n"} /^[a-zA-Z_-]+:.*##/ {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: run
run: ## Run in dev mode (cargo run, no release optimisations)
	$(CARGO) run -- $(SMTP_PORT_FLAG) $(UI_PORT_FLAG) $(RUN_ARGS)

.PHONY: run-release
run-release: release ## Run the release binary (faster startup)
	./target/release/$(BIN) $(SMTP_PORT_FLAG) $(UI_PORT_FLAG) $(RUN_ARGS)

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

# Files that are not testable from the lib are excluded from coverage so the
# number reflects the testable surface, matching what Codecov ignores.
COVERAGE_IGNORE := src/(main|assets|update|entrypoint)\.rs

.PHONY: coverage
coverage: ## Line coverage summary via cargo-llvm-cov (matches CI exclusions)
	$(CARGO) llvm-cov --lib --tests \
		--ignore-filename-regex='$(COVERAGE_IGNORE)' \
		--summary-only

.PHONY: coverage-html
coverage-html: ## HTML coverage report at target/llvm-cov/html/index.html
	$(CARGO) llvm-cov --lib --tests \
		--ignore-filename-regex='$(COVERAGE_IGNORE)' \
		--html

.PHONY: install
install: ## Install the binary into ~/.cargo/bin
	$(CARGO) install --path .

.PHONY: clean
clean: ## Remove build artifacts
	$(CARGO) clean
	rm -f lcov.info

SMOKE_SMTP := $(or $(SMTP_PORT),1025)
SMOKE_UI   := $(or $(UI_PORT),8025)

.PHONY: smoke
smoke: release ## Quick end-to-end smoke test against a fresh release binary
	@./target/release/$(BIN) -s $(SMOKE_SMTP) -u $(SMOKE_UI) --no-cli > /tmp/mbu-smoke.log 2>&1 & echo $$! > /tmp/mbu-smoke.pid
	@sleep 1
	@echo "-> SMTP send via swaks/sendmail not assumed; hit the API instead"
	@curl -sS http://127.0.0.1:$(SMOKE_UI)/api/messages | head -c 200; echo
	@kill $$(cat /tmp/mbu-smoke.pid); rm -f /tmp/mbu-smoke.pid /tmp/mbu-smoke.log
	@echo "smoke OK"
