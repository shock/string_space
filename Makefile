
# Makefile for building the python-inliner executable


# Rust toolchain
CARGO = cargo

# Targets
BUILD_DIR = target
DEBUG_DIR = $(BUILD_DIR)/debug
RELEASE_DIR = $(BUILD_DIR)/release
TARGET ?= /opt/local/bin

# Executable name
EXECUTABLE = string_space

# Default target
all: debug test

# Build the project in debug mode
debug:
	$(CARGO) build

# Build the project in release mode
release: test
	$(CARGO) build --release

# Clean up build artifacts
clean:
	$(CARGO) clean

test:
	tests/run_tests.sh

auto:
	tests/autocompleter.py

install: release
	cp $(RELEASE_DIR)/$(EXECUTABLE) $(TARGET)

benchmark: release
	RUST_BACKTRACE=full $(CARGO) run --release -- test/text.file -b 100000

server: release
	RUST_BACKTRACE=full $(RELEASE_DIR)/$(EXECUTABLE) start test/word_list.txt

client:
	python client.py

# Phony targets
.PHONY: all debug release clean test client
