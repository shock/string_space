# String Space

**Author**: Bill Doughty

---

This Rust project implements a word-list database that allows efficient insertion, searching, and storage of words, along with a simple TCP network API for remote access. The project includes a fast in-memory string storage mechanism, string searching by prefix or substring, and TCP commands for interacting with the word database.

## Features

- **Efficient String Storage**: Handles large datasets of strings with very fast insertion and lookup times.
- **Prefix and Substring Search**: Supports fast searching for strings by prefix and substring.
- **Fuzzy Search**: Includes a simple implementation of Jaro-Winkler fuzzy search for similarity matching.
- **Frequency and Age Tracking**: Tracks the frequency of word usage and their insertion time (age).
- **Simple TCP API**: Enables remote access to the word-list database via a minimal TCP protocol.
- **Random Word Generation**: Capable of generating a customizable number of random words for benchmarking and testing.
- **Benchmarks**: Includes tools for benchmarking the performance of string insertions and lookups.

## Project Structure

The project is organized as follows:

- `src/main.rs`: Main entry point for running the server and handling command-line arguments.
- `src/modules/`:
  - `protocol.rs`: Defines the TCP protocol for handling client connections and processing commands.
  - `string_space.rs`: Implements the core string storage logic.
  - `utils.rs`: Utility functions for generating random words and timing code execution.

## Prerequisites

Before running the project, ensure that the following are installed:

- [Rust](https://www.rust-lang.org/tools/install)
- TCP networking enabled on your system.

## Usage

To run the project, clone the repository, navigate to the project directory, and run the following command:

```bash
cargo run -- <data-file> --port <port> --host <host> [--benchmark <COUNT>]
```

### Command-Line Arguments

- `<data-file>`: The file where the word database will be stored. If the file doesn't exist, it will be created.
- `--port <port>`: The TCP port to bind to (default: 7878).
- `--host <host>`: The host address to bind to (default: `127.0.0.1`).
- `--benchmark <COUNT>`: If provided, runs a benchmark with `COUNT` random words. **Warning:** This will overwrite the data file.

Example:

```bash
cargo run -- data/words.txt --port 7878 --host 127.0.0.1
```

This starts a TCP server listening on `127.0.0.1:7878` and serving requests from the `words.txt` file.

## TCP API

The server listens for client connections on the specified host and port. It supports the following commands, each terminated by an End-of-Text (EOT) byte (`0x04`).  Requests are UTF-8 encoded strings, made up of one or more elements separated by by the ASCII RS (Record Separator) character (`0x1E`).  See client.py for an example implementation.

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

### Response Format

Responses from the server are text-based and end with an EOT byte (`0x04`).

- **Success**: Returns the requested data or an `OK` message.
- **Error**: Returns an error message starting with `ERROR -`.

## Benchmarks

You can run benchmarks to measure the performance of word insertion and lookup:

```bash
cargo run -- <data-file> --benchmark <COUNT>
```

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
