# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

String Space is a Rust-based word-list database server with efficient string storage, searching capabilities, and a TCP network API. The project includes both a Rust server and Python client components. It's designed for use with [llm_chat_cli](https://github.com/shock/llm_chat_cli) for word completion functionality.

## Architecture

### Core Components

- **Main Server** (`src/main.rs`): CLI interface with subcommands for server management (start, stop, status, restart, benchmark)
- **String Space Module** (`src/modules/string_space/`): Core data structure using custom memory allocation for efficient string storage and retrieval
- **Protocol Module** (`src/modules/protocol.rs`): TCP protocol implementation for client-server communication
- **Benchmark Module** (`src/modules/benchmark.rs`): Performance testing utilities
- **Utils Module** (`src/modules/utils.rs`): Utility functions including PID file management and path expansion
- **Word Struct Module** (`src/modules/word_struct.rs`): Word structure definitions with frequency and age tracking

### Key Features

- **Custom Memory Management**: Uses raw pointer allocation with 4KB alignment for optimal string storage
- **Efficient Search**: Binary search for prefix matching, linear scan for substring search
- **Fuzzy Search**: Jaro-Winkler similarity matching with configurable thresholds
- **Fuzzy-Subsequence Search**: Character order-preserving search with flexible spacing
- **Best Completions**: Intelligent search combining multiple algorithms with progressive execution and dynamic weighting
- **TCP Network Protocol**: ASCII RS (0x1E) as separator, EOT (0x04) as terminator
- **Daemon Mode**: Proper UNIX daemon implementation with PID file management
- **Frequency & Age Tracking**: Word usage frequency and insertion time tracking

## Development Commands

### Building
```bash
cargo build          # Debug build
cargo build --release # Release build
make debug           # Debug build via Makefile
make release         # Release build via Makefile
make install         # Install to /opt/local/bin
```

### Testing
```bash
make test            # Run full test suite including integration tests and unit tests
cargo test           # Run rust unit tests
./tests/run_tests.sh # Run comprehensive test script (requires SS_TEST=true) (same as `make test`)
```

### Running
```bash
# Start server
cargo run -- start <data-file> --port <port> --host <host> [--daemon]

# Example
cargo run -- start test/word_list.txt --port 7878 --host 127.0.0.1

# Using Makefile targets
make server          # Start server in release mode
make client          # Run Python client tests
```

### Benchmarking
```bash
cargo run -- benchmark <data-file> --count <N>
make benchmark       # Run benchmark with 100,000 words
```

### Server Management
```bash
cargo run -- start <data-file>    # Start server
cargo run -- stop                 # Stop server (daemon mode only)
cargo run -- status               # Check server status (daemon mode only)
cargo run -- restart <data-file>  # Restart server (daemon mode only)
```

## Python Integration

### Client Package
- **Location**: `python/string_space_client/`
- **Installation**: `uv sync` installs as editable package
- **Usage**: Import `StringSpaceClient` for TCP communication

### Completer Package
- **Location**: `python/string_space_completer/`
- **Purpose**: [prompt_toolkit](https://github.com/prompt-toolkit/python-prompt-toolkit) completer for word completion
- **Used by**: [llm_chat_cli](https://github.com/shock/llm_chat_cli)

## Protocol Specification

### Request Format
- **Separator**: ASCII RS (0x1E)
- **Terminator**: EOT (0x04)
- **Encoding**: UTF-8

### Commands
- `insert <words...>` - Insert one or more words
- `prefix <prefix>` - Search by prefix
- `substring <substring>` - Search by substring
- `similar <word> <threshold>` - Fuzzy search with Jaro-Winkler
- `fuzzy-subsequence <query>` - Fuzzy subsequence search with character order preservation
- `best-completions <query> [limit]` - Intelligent search combining multiple algorithms with progressive execution and dynamic weighting
- `data-file` - Get data file path
- `remove <words...>` - Remove words from storage
- `clear_space` - Clear all strings
- `get_all_strings` - Retrieve all stored strings
- `empty` - Check if storage is empty
- `len` - Get number of stored strings
- `capacity` - Get allocated memory (bytes)

### Response Format
- **Success**: Requested data or `OK` message
- **Error**: `ERROR -` prefixed message
- **Terminator**: EOT (0x04)

## Development Guidelines

- Follow the engineering guidelines in `ENGINEERING_GUIDELINES.md`
- Keep functions small and focused with clear, descriptive names
- Use comprehensive unit tests for all functionality
- Maintain modular architecture with clear separation of concerns
- Prefer verbose code with comments over dense implementations
- Use unsafe code judiciously with proper memory safety guarantees

## Important Notes

- **PID Files**: Stored in system temp directory for daemon management
- **Benchmark Mode**: Overwrites the specified data file
- **Integration Tests**: Require running server instance on port 9898
- **Test Mode**: Set `SS_TEST=true` environment variable for test scripts
- **String Limits**: Words must be 3-50 characters in length
- **Memory Alignment**: Uses 4KB alignment for optimal performance
- **Daemon Mode**: Proper double-fork implementation with signal handling
- **Database Format**: See [DATA_FORMAT.md](DATA_FORMAT.md) for details on data file structure