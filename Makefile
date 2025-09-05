.PHONY: build install uninstall clean test help

# Default target
all: build

# Build the project in release mode
build:
	cargo build --release

# Install the binary to ~/.cargo/bin (user install)
install: build
	cargo install --path .

# Install the binary system-wide (requires sudo)
install-system: build
	sudo cp target/release/queuecast /usr/local/bin/

# Uninstall from user directory
uninstall:
	cargo uninstall queuecast

# Uninstall from system directory
uninstall-system:
	sudo rm -f /usr/local/bin/queuecast

# Clean build artifacts
clean:
	cargo clean

# Development build (debug mode)
dev:
	cargo build

# Run the program (for development)
run:
	cargo run

# Show help
help:
	@echo "Available targets:"
	@echo "  build          - Build the project in release mode"
	@echo "  install        - Install to ~/.cargo/bin (user install)"
	@echo "  install-system - Install to /usr/local/bin (system-wide, requires sudo)"
	@echo "  uninstall      - Remove from ~/.cargo/bin"
	@echo "  uninstall-system - Remove from /usr/local/bin (requires sudo)"
	@echo "  clean          - Clean build artifacts"
	@echo "  dev            - Build in debug mode"
	@echo "  run            - Run the program (development)"
	@echo "  help           - Show this help message"
