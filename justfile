# Justfile for Zettelkasten CLI - modern alternative to Makefile
# Usage: just <command>

# Configuration
cargo := "cargo"
target_dir := "target"
version := `grep '^version' crates/zettel-cli/Cargo.toml | cut -d'"' -f2`

# Default recipe
default:
    @just --list

# Development commands
build:
    {{cargo}} build --workspace

build-release:
    {{cargo}} build --workspace --release

test:
    {{cargo}} test --workspace

test-integration:
    ./tests/run-integration-tests.sh

watch:
    {{cargo}} watch -x "build --workspace"

# Code quality
check:
    {{cargo}} clippy --workspace -- -D warnings
    {{cargo}} fmt --check

fmt:
    {{cargo}} fmt

audit:
    {{cargo}} audit

# Benchmarks and performance
bench:
    {{cargo}} bench --workspace

profile binary="zettel" args="":
    {{cargo}} build --release
    perf record --call-graph=dwarf {{target_dir}}/release/{{binary}} {{args}}
    perf report

# Documentation
docs:
    {{cargo}} doc --workspace --no-deps --open

docs-book:
    mdbook build docs/book

docs-serve:
    mdbook serve docs/book

# Installation
install: build-release
    {{cargo}} install --path crates/zettel-cli --force
    {{cargo}} install --path crates/zettel-lsp --force

# Shell completions
completions: build-release
    mkdir -p scripts/completions
    {{target_dir}}/release/zettel completions bash > scripts/completions/zettel.bash
    {{target_dir}}/release/zettel completions zsh > scripts/completions/zettel.zsh
    {{target_dir}}/release/zettel completions fish > scripts/completions/zettel.fish

# Testing with different configurations
test-minimal:
    ZETTEL_CONFIG=tests/configs/minimal.toml {{cargo}} test

test-advanced:
    ZETTEL_CONFIG=tests/configs/advanced.toml {{cargo}} test

test-large-vault:
    ./tools/benchmark.sh large-vault

# Packaging
package-all: build-release completions
    ./scripts/create-packages.sh {{version}}

# Release workflow
pre-release: test check build-release docs-book completions
    echo "Ready for release {{version}}"

release tag=version: pre-release
    git tag -a v{{tag}} -m "Release v{{tag}}"
    git push origin v{{tag}}

# Development setup
setup:
    rustup component add clippy rustfmt
    cargo install mdbook
    cargo install cargo-watch
    cargo install cargo-audit

# Clean up
clean:
    {{cargo}} clean

clean-all: clean
    rm -rf docs/book/book
    rm -f scripts/completions/*
