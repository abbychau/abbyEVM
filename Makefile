# AbbyEVM Makefile

.PHONY: help build test clean run examples fmt clippy docs install dev-setup

help: ## Show this help message
	@echo "AbbyEVM - User-Friendly Ethereum Virtual Machine"
	@echo ""
	@echo "Available commands:"
	@awk 'BEGIN {FS = ":.*##"; printf "\033[36m%-15s\033[0m %s\n", "Command", "Description"} /^[a-zA-Z_-]+:.*?##/ { printf "\033[36m%-15s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: ## Build the project in release mode
	cargo build --release

build-debug: ## Build the project in debug mode
	cargo build

test: ## Run all tests
	cargo test

test-verbose: ## Run tests with verbose output
	cargo test -- --nocapture

clean: ## Clean build artifacts
	cargo clean

run: build ## Run the EVM with simple addition example
	cargo run --release -- execute --example simple-add

run-verbose: build ## Run with verbose output
	cargo run --release -- execute --example simple-add --verbose

examples: build ## Run all examples
	@echo "🧪 Running all examples..."
	cargo run --release -- examples

interactive: build ## Start interactive mode
	cargo run --release -- interactive

analyze: build ## Analyze simple addition bytecode
	cargo run --release -- analyze --bytecode "6001600201"

fmt: ## Format code
	cargo fmt

fmt-check: ## Check code formatting
	cargo fmt -- --check

clippy: ## Run clippy lints
	cargo clippy -- -D warnings

clippy-fix: ## Fix clippy issues automatically
	cargo clippy --fix

docs: ## Generate documentation
	cargo doc --open

install: build ## Install the binary
	cargo install --path .

dev-setup: ## Set up development environment
	rustup component add rustfmt clippy
	@echo "Development environment ready!"

# Example commands
demo: build ## Run a complete demo
	@echo "🚀 AbbyEVM Demo"
	@echo "==============="
	@echo ""
	@echo "1. Simple Addition (1 + 2):"
	cargo run --release -- execute --example simple-add
	@echo ""
	@echo "2. Simple Multiplication (2 * 3):"
	cargo run --release -- execute --example simple-mul
	@echo ""
	@echo "3. Storage Operations:"
	cargo run --release -- execute --example storage
	@echo ""
	@echo "4. Bytecode Analysis:"
	cargo run --release -- analyze --bytecode "6001600201"

quick-test: ## Quick test suite
	cargo test --lib

integration-test: build ## Run integration tests
	@echo "Testing CLI interface..."
	@echo "6001600201" > /tmp/test_bytecode.bin
	cargo run --release -- execute --file /tmp/test_bytecode.bin
	@rm -f /tmp/test_bytecode.bin
	@echo "Integration tests passed! ✅"

benchmark: build ## Run simple benchmarks
	@echo "Benchmarking EVM operations..."
	@time cargo run --release -- execute --bytecode "6001600201" > /dev/null
	@time cargo run --release -- execute --example storage > /dev/null
	@echo "Benchmark complete!"

check-all: fmt-check clippy test ## Run all checks (formatting, linting, tests)
	@echo "All checks passed! ✅"

# Development helpers
watch-test: ## Watch for changes and run tests
	cargo watch -x test

watch-build: ## Watch for changes and rebuild
	cargo watch -x build

size: build ## Show binary size
	@ls -lh target/release/abby_evm | awk '{print "Binary size: " $$5}'

# Docker support (if you want to add it later)
docker-build: ## Build Docker image
	docker build -t abby-evm .

docker-run: docker-build ## Run in Docker
	docker run --rm -it abby-evm execute --example simple-add

# Release preparation
pre-release: clean check-all build docs ## Prepare for release
	@echo "Release preparation complete! 🚀"

# Default target
all: check-all build examples ## Run checks, build, and examples
