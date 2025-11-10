# Gmail MCP Server Makefile
#
# This Makefile provides convenient targets for building, testing, and deploying
# the Gmail MCP Server. The default target is 'build', which builds the local binary.
#
# Available targets:
#   make          - Build the local binary (default)
#   make clean    - Remove build artifacts and binaries
#   make deps     - Fetch Rust dependencies
#   make build    - Build the release binary (depends on deps)
#   make build-dev - Build the debug binary
#   make test     - Run all tests with verbose output
#   make docker   - Build the Docker image
#   make all      - Run clean, deps, test, build, and docker in sequence

.PHONY: clean deps build build-dev test docker all

# Binary name
BINARY_NAME=gmail-mcp-server

# Clean build artifacts
# Removes the compiled binary, dist directory, and Rust build cache
clean:
	@echo "Cleaning build artifacts..."
	@rm -f $(BINARY_NAME)
	@rm -rf dist/
	@cargo clean

# Get dependencies
# Fetches all Rust crate dependencies
deps:
	@echo "Fetching dependencies..."
	@cargo fetch

# Build the release binary
# Compiles the Rust application into a release binary
# Automatically runs 'deps' first to ensure dependencies are up to date
build: deps
	@echo "Building $(BINARY_NAME) (release)..."
	@cargo build --release
	@cp target/release/$(BINARY_NAME) $(BINARY_NAME)

# Build the debug binary
# Compiles the Rust application into a debug binary for development
build-dev: deps
	@echo "Building $(BINARY_NAME) (debug)..."
	@cargo build
	@cp target/debug/$(BINARY_NAME) $(BINARY_NAME)

# Run tests
# Executes all Rust tests in the project with verbose output
test:
	@echo "Running tests..."
	@cargo test --verbose

# Build Docker image
# Creates a Docker image tagged as ghcr.io/mstump/gmail-mcp-server:<short-git-sha>
# Requires Docker to be installed and running
docker:
	@echo "Building Docker image..."
	@SHORT_SHA=$$(git rev-parse --short=7 HEAD); \
	docker build -t ghcr.io/mstump/gmail-mcp-server:$$SHORT_SHA .

# Run all targets: clean, deps, test, build, and docker
# Executes all build steps in sequence for a complete build pipeline
# Order: clean → deps → test → build → docker
all: clean deps test build docker
	@echo "All targets completed successfully!"

# Default target
# When running 'make' without arguments, this target will be executed
.DEFAULT_GOAL := build
