.PHONY: help build dev test lint clippy format clean install

# Default target
help:
	@echo "Available targets:"
	@echo "  build     - Build the application"
	@echo "  dev       - Start development server"
	@echo "  test      - Run tests"
	@echo "  lint      - Run linting checks"
	@echo "  clippy    - Run clippy checks"
	@echo "  format    - Format code"
	@echo "  clean     - Clean build artifacts"
	@echo "  install   - Install dependencies"

# Build the application
build:
	cargo build --release
	npm run build

# Start development server
dev:
	npm run tauri dev

# Run tests
test:
	cargo test
	npm run test

# Run linting checks
lint:
	npm run lint
	cargo clippy --all-targets --all-features -- -D warnings

# Run clippy checks specifically
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

# Format code
format:
	cargo fmt --all
	npm run format

# Clean build artifacts
clean:
	cargo clean
	rm -rf node_modules
	rm -rf dist
	rm -rf src-tauri/target

# Install dependencies
install:
	npm install
	cargo fetch --locked

# Check for security vulnerabilities
audit:
	npm audit
	cargo audit

# Run all checks before commit
check: lint test
	@echo "All checks passed!"

# Build and run in release mode
run-release:
	cargo build --release
	npm run build
	npm run tauri build

# Development setup
setup:
	@echo "Setting up development environment..."
	npm install
	cargo install clippy
	@echo "Development environment setup complete!"

# Update dependencies
update:
	npm update
	cargo update

# Generate documentation
docs:
	cargo doc --no-deps --open

# Benchmark performance
benchmark:
	cargo bench

# Check code coverage
coverage:
	cargo tarpaulin --out Html

# Run database migrations (if applicable)
migrate:
	cargo run --bin migrate

# Seed database (if applicable)
seed:
	cargo run --bin seed

# Run integration tests
integration-test:
	cargo test --test integration

# Run unit tests only
unit-test:
	cargo test --lib
