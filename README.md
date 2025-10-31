# String Space

**Author**: Bill Doughty

---

This Rust project implements a fast word-list database that allows efficient insertion, searching, and storage of unique words along with frequency and last-inserted timestamp tracking.  The project provides a simple server executable with a TCP network API for word inseertion and querying matches. The server supports a fast in-memory string storage mechanism, string searching by prefix, substring, fuzzy subsequence, and Jaro-Winkler similarity matching.  It has a command-line interface for starting and stopping the server in standalone or daemon mode.

The project also includes a Python client package that provides an easy way to connect to the server and use its features.  It also includes a Python `prompt_toolkit` completer for word completion in command line tools.

## Features

- **Efficient String Storage**: Handles large datasets of strings with very fast insertion and lookup times.
- **Prefix and Substring Search**: Supports fast searching for strings by prefix and substring.
- **Fuzzy Search**: Includes a simple implementation of Jaro-Winkler fuzzy search for similarity matching.
- **Fuzzy-Subsequence Search**: Character order-preserving search with flexible spacing for abbreviations and partial matches.
- **Best Completions (Recommended)**: Intelligent search combining multiple algorithms (prefix, substring, fuzzy subsequence, Jaro-Winkler) with relevance scoring. **This is the most recommended method for auto completion** as it provides the highest quality results by intelligently combining multiple search strategies.
- **Frequency and Age Tracking**: Tracks the frequency of word usage and their insertion time (age).
- **Simple TCP API**: Enables remote access to the word-list database via a minimal TCP protocol.
- **Random Word Generation**: Capable of generating a customizable number of random words for benchmarking and testing.
- **Benchmarks**: Includes tools for benchmarking the performance of string insertions and lookups.

## Quick Start Guide for use with [llm_chat_cli](https://github.com/shock/llm_chat_cli)

- Ensure you have `rust` [installed on your system](https://rust-lang.org/tools/install/)
- Clone this repository: `git clone https://github.com/shock/string_space.git`
- Navigate to the repository directory: `cd string_space`
- Build and install the executable:
  - execute `./setup_opt_local_bin.sh`
  - execute `make install`
  - execute `mkdir -p ~/.llm_chat_cli/`
  - execute `string_space start -d ~/.llm_chat_cli/word_list.txt`
  - run the `llm_chat_cli` program (see https://github.com/shock/llm_chat_cli). It should silently connect to the server. You will see a warning message if it cannot connect.
  - execute `string_space stop` to stop the server

**Note**: The `llm_chat_cli` program uses the **best completions** method by default, which is the recommended approach for auto completion as it provides the highest quality results by intelligently combining multiple search strategies.

### Starting the Server Automatically using crond
To run the string_space server in in the background automatically when your machine boots, add the following to your crontab (`crontab -e`):

```crontab
@reboot /opt/local/bin/string_space start -d ~/.llm_chat_cli/word_list.txt > /dev/null 2>&1
```

***NOTE:*** On macOS, you may need to give full-disk access permissions to `cron` in **System Settings > Privacy & Security > Full Disk Access**.  Click the **'+'** icon.  In the Finder window that opens, **SHIFT+CMD+G** to open the search bar, type `/usr/sbin/cron` and enter.  Click **Open** or **OK**.

## Project Structure

The project is organized as follows:

- `src/main.rs`: Main entry point for running the server and handling command-line arguments.
- `src/modules/`:
  - `protocol.rs`: Defines the TCP protocol for handling client connections and processing commands.
  - `string_space.rs`: Implements the core string storage logic.
  - `benchmark.rs`: Performance testing utilities.
  - `utils.rs`: Utility functions for generating random words, timing code execution, and PID management.
  - `word_struct.rs`: Word structure definitions.
- `python/string_space_client/`: Python client package for easy integration into python projects. Provides methods for all server commands including `prefix_search`, `substring_search`, `similar_search`, `fuzzy_subsequence_search`, and `best_completions_search`.
- `python/string_space_completer/`: Python [prompt_toolkit](https://github.com/prompt-toolkit/python-prompt-toolkit) completer package for word completion in command line tools using `prompt_toolkit`.  Used by [llm_chat_cli](https://github.com/shock/llm_chat_cli).
- `tests/`: Integration tests and test runner scripts.

## Prerequisites

Before building the project, ensure that the following are installed:

- [Rust](https://www.rust-lang.org/tools/install)
- TCP networking enabled on your system.

## Usage

To run the project, clone the repository, navigate to the project directory, and use the following commands:

### Daemon vs Foreground Mode

The server can run in two modes:

- **Foreground Mode** (default): Server runs in the current terminal session, displaying logs and output directly. Use this for development and debugging.
- **Daemon Mode** (with `--daemon` flag): Server runs as a background process with proper UNIX daemon conventions (double-fork, session management, I/O redirection). Use this for production deployment.

### Server Management Commands

*Note: Stop, status, and restart commands only apply to servers started in daemon mode.*

- **Start server**:
  ```bash
  cargo run -- start <data-file> --port <port> --host <host> [--daemon]
  ```

- **Stop server** (daemon mode only):
  ```bash
  cargo run -- stop
  ```

- **Check server status** (daemon mode only):
  ```bash
  cargo run -- status
  ```
  *Displays server status, PID, and PID file location*

- **Restart server** (daemon mode only):
  ```bash
  cargo run -- restart <data-file> --port <port> --host <host>
  ```

- **Run benchmarks**:
  ```bash
  cargo run -- benchmark <data-file> --count <COUNT>
  ```

### Command-Line Arguments

- `<data-file>`: The file where the word database will be stored. If the file doesn't exist, it will be created.
- `--port <port>`: The TCP port to bind to (default: 7878).
- `--host <host>`: The host address to bind to (default: `127.0.0.1`).
- `--daemon`: Run server in background as daemon. When omitted, server runs in foreground.
- `--count <COUNT>`: Number of random words for benchmarking. **Warning:** This will overwrite the data file.

Example:

```bash
cargo run -- start data/words.txt --port 7878 --host 127.0.0.1
```

This starts a TCP server listening on `127.0.0.1:7878` and serving requests from the `words.txt` file.

## TCP API

The server listens for client connections on the specified host and port. It supports the following commands, each terminated by an End-of-Text (EOT) byte (`0x04`).  Requests are UTF-8 encoded strings, made up of one or more elements separated by by the ASCII RS (Record Separator) character (`0x1E`).  See the Python client package in `python/string_space_client/` for an example implementation.

### Commands

1. **Insert Words**
   - **Command**: `insert <words...>`
   - **Description**: Inserts one or more words into the word-list database. Words are separated by spaces.
   - **Response**: `OK\nInserted <count> of <total> words`

2. **Search by Prefix**
   - **Command**: `prefix <prefix>`
   - **Description**: Searches for words that start with the given prefix.
   - **Response**: A list of matching words, each on a new line.

3. **Search by Substring**
   - **Command**: `substring <substring>`
   - **Description**: Searches for words that contain the given substring.
   - **Response**: A list of matching words, each on a new line.

4. **Get Similar Words**
   - **Command**: `similar <word> <threshold>`
   - **Description**: Searches for words similar to the provided word, based on a similarity threshold.
   - **Response**: A list of similar words.

5. **Fuzzy-Subsequence Search**
   - **Command**: `fuzzy-subsequence <query>`
   - **Description**: Searches for words where query characters appear in order, but not necessarily consecutively. Useful for abbreviations and partial matches.
   - **Response**: A list of matching words, each on a new line.

6. **Best Completions**
   - **Command**: `best-completions <query> [limit]`
   - **Description**: Finds the best completions for a query using multiple search algorithms (prefix, substring, fuzzy subsequence, and Jaro-Winkler similarity). Returns results sorted by relevance score.
   - **Parameters**:
     - `query` (required): The search query string
     - `limit` (optional): Maximum number of results to return
   - **Response**: A list of matching words with relevance scores, each on a new line.

7. **Additional Operations**
   - **Remove Words**: `remove <words...>` - Remove words from storage
   - **Clear Space**: `clear_space` - Clear all strings
   - **Get All Strings**: `get_all_strings` - Retrieve all stored strings
   - **Check Empty**: `empty` - Check if storage is empty
   - **Get Length**: `len` - Get number of stored strings
   - **Get Capacity**: `capacity` - Get total allocated memory for string storage (in bytes)
   - **Get Data File**: `data_file` - Get data file path

### Response Format

Responses from the server are text-based and end with an EOT byte (`0x04`).

- **Success**: Returns the requested data or an `OK` message.
- **Error**: Returns an error message starting with `ERROR -`.

## Python Client Usage

The Python client package provides easy access to all server commands. **For auto completion, we recommend using `best_completions_search` as it provides the highest quality results.** Here's how to use the `best_completions_search` method:

```python
from string_space_client import StringSpaceClient

# Create client instance
client = StringSpaceClient('127.0.0.1', 7878)

# Basic best completions search
results = client.best_completions_search("hel")
print(results)
# Output: ['help', 'hello', 'helicopter', 'world']

# With custom limit
results = client.best_completions_search("app", limit=5)
print(results)
# Output: ['apple', 'application', 'apply', 'applesauce', 'apparatus']

# Other available methods
prefix_results = client.prefix_search("hel")
substring_results = client.substring_search("world")
fuzzy_results = client.fuzzy_subsequence_search("hl")
similar_results = client.similar_search("hello", 0.8)

# Insert words
client.insert(["hello", "world", "test"])

# Get data file path
data_file = client.data_file()
```

### Key Features of `best_completions_search` (Recommended for Auto Completion):

- **Progressive Algorithm Execution**: Uses multiple search algorithms in priority order
- **Intelligent Scoring**: Combines match type, frequency, and age for relevance ranking
- **Deduplication**: Removes duplicate results across algorithms
- **Configurable Limits**: Customizable result count (1-100)
- **Error Handling**: Graceful handling of connection issues and server errors
- **Highest Quality Results**: **Recommended as the primary method for auto completion** - provides the most relevant completions by intelligently combining multiple search strategies

### Algorithm Execution Order:
1. **Prefix Search** - Exact prefix matches (highest priority)
2. **Fuzzy Subsequence Search** - Character order-preserving matches
3. **Jaro-Winkler Similarity** - Fuzzy similarity matches
4. **Substring Search** - General substring matches

## `prompt_toolkit` Completer Usage

A `prompt_toolkit` Completer is available for auto-completion in `prompt_toolkit` applications.  See example usage in [python/autocompleter.py](python/autocompleter.py).

To try the completer, run the following commands in separate terminals:

```bash
make test-server
```

```bash
uv sync
make auto
```

 The word database will be initialized in tests/data/words.txt if it does not already exist.  Entering words at the prompt will add them to the database.

- **COUNT**: Number of random words to generate and insert.

The benchmark will report the time taken for various operations, including insertion, file writing, and searching by prefix and substring.

## Tests

To run the unit tests:

```bash
cargo test
```

The test suite covers various functionalities, including:

- String insertion.
- Prefix and substring search.
- Sorting and clearing the database.
- File reading and writing.

## Example Output

```
Inserting 1000000 words took 1.23s
Writing strings to file took 0.45s
Found 5 strings with prefix 'hel':
  hello 2
  help 1
  helicopter 1
Finding strings with prefix 'hel' took 0.01s
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
