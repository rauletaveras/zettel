# Main Makefile for Zettelkasten CLI
SHELL := /bin/bash
.PHONY: help build test install clean docs release

# Configuration
CARGO := cargo
TARGET_DIR := target
INSTALL_PREFIX := /usr/local
VERSION := $(shell grep '^version' crates/zettel-cli/Cargo.toml | cut -d'"' -f2)

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Development
build: ## Build all crates
	$(CARGO) build --workspace

build-release: ## Build optimized release binaries
	$(CARGO) build --workspace --release

test: ## Run all tests
	$(CARGO) test --workspace

test-integration: ## Run integration tests
	./tests/run-integration-tests.sh

bench: ## Run performance benchmarks
	$(CARGO) bench --workspace

# Quality assurance
check: ## Run clippy and formatting checks
	$(CARGO) clippy --workspace -- -D warnings
	$(CARGO) fmt --check

fmt: ## Format code
	$(CARGO) fmt

audit: ## Security audit
	$(CARGO) audit

# Documentation
docs: ## Build documentation
	$(CARGO) doc --workspace --no-deps
	mdbook build docs/book

docs-serve: ## Serve documentation locally
	mdbook serve docs/book

man-pages: ## Generate man pages from markdown
	for page in docs/man/*.1.md; do \
		pandoc -s -t man "$$page" -o "$${page%.md}"; \
	done

# Installation
install: build-release ## Install to system
	install -Dm755 $(TARGET_DIR)/release/zettel $(INSTALL_PREFIX)/bin/zettel
	install -Dm755 $(TARGET_DIR)/release/zettel-lsp $(INSTALL_PREFIX)/bin/zettel-lsp
	install -Dm644 docs/man/zettel.1 $(INSTALL_PREFIX)/share/man/man1/zettel.1
	install -Dm644 scripts/completions/zettel.bash $(INSTALL_PREFIX)/share/bash-completion/completions/zettel
	install -Dm644 scripts/completions/zettel.zsh $(INSTALL_PREFIX)/share/zsh/site-functions/_zettel
	install -Dm644 scripts/completions/zettel.fish $(INSTALL_PREFIX)/share/fish/vendor_completions.d/zettel.fish

install-dev: build ## Install development version
	$(CARGO) install --path crates/zettel-cli --force
	$(CARGO) install --path crates/zettel-lsp --force

uninstall: ## Uninstall from system
	rm -f $(INSTALL_PREFIX)/bin/zettel
	rm -f $(INSTALL_PREFIX)/bin/zettel-lsp
	rm -f $(INSTALL_PREFIX)/share/man/man1/zettel.1
	rm -f $(INSTALL_PREFIX)/share/bash-completion/completions/zettel
	rm -f $(INSTALL_PREFIX)/share/zsh/site-functions/_zettel
	rm -f $(INSTALL_PREFIX)/share/fish/vendor_completions.d/zettel.fish

# Shell completions
completions: ## Generate shell completions
	mkdir -p scripts/completions
	$(TARGET_DIR)/release/zettel completions bash > scripts/completions/zettel.bash
	$(TARGET_DIR)/release/zettel completions zsh > scripts/completions/zettel.zsh
	$(TARGET_DIR)/release/zettel completions fish > scripts/completions/zettel.fish

# Packaging
package-deb: build-release ## Build Debian package
	./scripts/packaging/debian/build-deb.sh $(VERSION)

package-rpm: build-release ## Build RPM package
	./scripts/packaging/rpm/build-rpm.sh $(VERSION)

package-homebrew: ## Update Homebrew formula
	./scripts/packaging/homebrew/update-formula.sh $(VERSION)

package-arch: ## Build Arch Linux package
	./scripts/packaging/arch/build-pkg.sh $(VERSION)

# Release
release: test build-release docs completions ## Prepare release
	git tag -a v$(VERSION) -m "Release v$(VERSION)"
	./scripts/create-release-artifacts.sh $(VERSION)

release-upload: ## Upload release to GitHub
	gh release create v$(VERSION) \
		--title "v$(VERSION)" \
		--notes-file CHANGELOG.md \
		target/release/zettel-*

# Cleanup
clean: ## Clean build artifacts
	$(CARGO) clean
	rm -rf docs/book/book
	rm -f docs/man/*.1

clean-all: clean ## Clean everything including caches
	rm -rf ~/.cargo/registry/cache/
