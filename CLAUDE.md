# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

String Space is a Rust-based word-list database server with efficient string storage, searching capabilities, and a TCP network API. The project includes both a Rust server and Python client components.

## Architecture

### Core Components

- **Main Server** (`src/main.rs`): CLI interface with subcommands for server management (start, stop, status, restart, benchmark)
- **String Space Module** (`src/modules/string_space.rs`): Core data structure for efficient string storage and retrieval
- **Protocol Module** (`src/modules/protocol.rs`): TCP protocol implementation for client-server communication
- **Benchmark Module** (`src/modules/benchmark.rs`): Performance testing utilities
- **Utils Module** (`src/modules/utils.rs`): Utility functions including PID file management

### Key Features

- Efficient in-memory string storage with prefix and substring search
- Jaro-Winkler fuzzy search for similarity matching
- TCP-based network protocol using ASCII RS (0x1E) as separator and EOT (0x04) as terminator
- Daemon mode with PID file management
- Frequency and age tracking for words

## Development Commands

### Building
```bash
cargo build          # Debug build
cargo build --release # Release build
make debug           # Debug build via Makefile
make release         # Release build via Makefile
```

### Testing
```bash
cargo test           # Run unit tests
make test            # Run full test suite including integration tests
./tests/run_tests.sh # Run comprehensive test script
```

### Running
```bash
# Start server
cargo run -- start <data-file> --port <port> --host <host> [--daemon]

# Example
cargo run -- start test/word_list.txt --port 7878 --host 127.0.0.1

# Using Makefile targets
make server          # Start server in release mode
make client          # Run Python client
```

### Benchmarking
```bash
cargo run -- benchmark <data-file> --count <N>
make benchmark       # Run benchmark with 100,000 words
```

### Server Management
```bash
cargo run -- start <data-file>    # Start server
cargo run -- stop                 # Stop server
cargo run -- status               # Check server status
cargo run -- restart <data-file>  # Restart server
```

## Python Client

The project includes a Python client package in `python/string_space_client/` that can be installed as an editable package:

```bash
uv sync              # Install dependencies including editable client
python client.py     # Run client tests
```

## Protocol Specification

- **Request Format**: UTF-8 strings separated by ASCII RS (0x1E), terminated by EOT (0x04)
- **Commands**: `insert`, `prefix`, `substring`, `similar`
- **Response Format**: Text-based, terminated by EOT (0x04)

## Development Guidelines

- Follow the engineering guidelines in `ENGINEERING_GUIDELINES.md`
- Keep functions small and focused with clear, descriptive names
- Use comprehensive unit tests for all functionality
- Maintain modular architecture with clear separation of concerns
- Prefer verbose code with comments over dense implementations

## Important Notes

- The server uses PID files for daemon management (stored in system temp directory)
- Benchmark mode will overwrite the specified data file
- Integration tests require a running server instance on port 9898
- Set `SS_TEST=true` environment variable when running test scripts to enable test mode