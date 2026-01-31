# String Space Database File Format

## Overview

Plain text file format for storing words with optional metadata.

## File Structure

One record per line, fields separated by whitespace.

## Record Format

```
<string> <frequency> <age_days>
```

## Field Definitions

### String (required)
   - The word or phrase to store
   - UTF-8 encoded text
   - Length constraint: 3-50 characters
   - Strings are trimmed of leading/trailing whitespace

### Frequency (optional, defaults to 1)
   - Type: unsigned 16-bit integer (u16)
   - Represents how often the string is used
   - If parsing fails, defaults to 1
   - Incremented when duplicate strings are inserted

### Age Days (optional, defaults to current time)
   - Type: unsigned 32-bit integer (u32)
   - Days since Unix epoch (January 1, 1970)
   - Used for ranking/search relevance
   - If parsing fails, defaults to current time
   - Updated when duplicate strings are inserted

## Examples

### Strings only
```
hello
world
test
```

### With frequency
```
hello 5
world 3
```

### Complete format
```
hello 5 19700
world 3 19705
```

## Implementation Details

Writing:
- Iterates through all stored strings
- Writes each as: `{string} {frequency} {age_days}\n`
- Uses buffered I/O for efficiency

Reading:
- Clears existing data before loading
- Reads line by line
- Splits each line by whitespace
- Parses frequency and age with fallback defaults
- Inserts each string into in-memory data structure

## Key Points

- Flexible parsing: Missing or invalid metadata fields default to sensible values
- UTF-8 encoding: Full Unicode support for international text
- Sorted storage: Strings are stored in sorted order in memory (though not necessarily in file)
- Metadata preserved: Frequency and age information is maintained across save/load cycles
- Whitespace tolerant: Uses whitespace splitting so tabs or multiple spaces work as delimiters
