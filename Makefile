
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

auto-server:
	touch test/auto_words.txt; \
	cargo run -- start --port=7879 --host=127.0.0.1 test/auto_words.txt

auto-client:
	python/autocompleter.py

install: release
	cp $(RELEASE_DIR)/$(EXECUTABLE) $(TARGET)

benchmark: release
	RUST_BACKTRACE=full $(CARGO) run --release -- test/text.file -b 100000

server: release
	RUST_BACKTRACE=full $(RELEASE_DIR)/$(EXECUTABLE) start test/word_list.txt

# TypeScript integration tests: builds server, starts daemon, runs all ts_*.ts scripts, stops server
ts-test: cargo-build
	tests/run_ts_tests.sh

# Lua integration tests: builds server, starts daemon, runs lua test script, stops server
lua-test: cargo-build
	tests/run_lua_tests.sh

cargo-build:
	$(CARGO) build

client:
	python client.py

# Phony targets
.PHONY: all debug release clean test ts-test lua-test client auto-server cargo-build