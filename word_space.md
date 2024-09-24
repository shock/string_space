# In-memory String Database

## Goal
The main goal of this is to efficiently manage storage of Hundreds of thousands to millions of strings in memory with minimal overhead and fragmentation caused by using hundreds String types.

### Summary Requirements

I Want to build an in-memory database for thousands to millions of strings in Rust. I don't want to use the native String type in rust. I want to manage the memory myself.  The strings that I will be storing will not grow or shrink in size. They will be static in length for the duration of their lifetime. As for the memory storage, I want to allocate one continuous block of bytes to store all of the strings contiguously in memory.  The strings will be stored in Unicode UTF-8 Format.  The minimum string length will be MIN_CHARS characters.  The maximum string length will be MAX_CHARS characters.

I will keep track of the strings using single-byte offset pointers into the heap. The reference pointers will point to the location in the heap. and have the length of the UTF-8 string pointed to in bytes. I want to use a rust structure to track the pointers and the length of the string, As well as some metadata about the string.  I will name it StringRef.  The meta data will include a frequency counter (positive integer), a static sequence of the first characters of the string named prefix, and I may add other static data later.

To manage the strings and make them indexable, sortable, and searchable, I will keep a vector of StringRefs.  The string ref factor will be closely tied to the string memory heap. Strings will be added and removed via the vector.  There will be a container struct to keep track of the StringRefs and the heap.  it will be called StringSpace.

struct StringSpace {
    buffer: Vec<u8>,      // the buffer of unicode characters
    used_bytes: usize,    // the number of bytes in the buffer that are used (index to last used byte)
    active_bytes: usize,  // the number of bytes in the buffer that are active (sum of lengths of all strings)
    string_refs: Vec<StringRef>,
}

struct StringRef {
    pointer: usize,   // points to the byte location in the heap of the string's UTF-8 bytes
    length: usize,    // length of the UTF-8 string in bytes
    prefix: [u8; 12], // first 3 characters of the string (up to 4 bytes per UTF-8 character) for accelerated searching
    frequency: u32,   // frequency counter (meta data)
}

I will use the StringRef struct to track the pointers and the length of the string.  I will also use the prefix field to store the first few 3 characters of the string.  All strings will be a minimum of 3 characters long.

Functions will be used to insert and remove strings from the collection. If a string is removed from the collection, it's StringRef will be removed from the vector without affecting the memory heap.  If a string is inserted into the collection, it will be copied into the memory heap at the end of the active bytes, and a new StringRef will be created and added to the vector with the pointer and length of the new string.  if there is not enough space in the heap to store the string, a new heap larger heap will be allocated, the existing heap will be copied to the new heap, and the StringRefs will be updated to point to the new heap.


## StringRef

StringRef is a struct that contains a pointer to the location in the heap of the string's UTF-8 bytes, the length of the string in bytes, and a prefix of the first 3 characters of the string.  The prefix will be used for accelerated searching.  The prefix will be stored as a static array of bytes for direct access without referencing other memory locations.  The first 3 characters of the string will be stored in the prefix array.  The prefix array will be 12 bytes long, regardless of how many UTF-8 bytes are required to store the 3 characters.

## StringSpace

StringSpace is a struct that contains a buffer of bytes for storing UTF-8 strings, the number of active bytes in the buffer, and a vector of StringRefs.  The buffer will be a contiguous block of memory that will be used to store all of the strings.  The active bytes will be the number of bytes in the buffer that are currently active.  The vector of StringRefs will be used to keep track of the StringRefs in the buffer.  The StringRefs will be added and removed from the vector as strings are inserted and removed from the buffer.

used_bytes is the number of bytes in the buffer that are used, but not necessarily active.  used_bytes can be greater than active_bytes if strings are removed from the collection and their buffer space is no longer being used.  active_bytes is the number of bytes in the buffer that are active, meaning they are being used by the StringRefs in the vector.

## Inserting a String

To insert a string into the StringSpace, the following steps will be taken:

1. Check if there is enough space in the buffer to store the string. If there is not enough space, a new buffer will be allocated and the existing buffer will be copied to the new buffer. The StringRefs will be updated to point to the new buffer.
2. Copy the string into the buffer at the end of the active bytes. The pointer to the string will be calculated by adding the active bytes to the start of the buffer.
3. Create a new StringRef for the string and add it to the vector of StringRefs.
4. Increase the active bytes by the length of the string.

## Removing a String

To remove a string from the StringSpace, the following steps will be taken:

1. Remove the StringRef from the vector of StringRefs.

### Heap Management

The heap will be managed by a struct called StringSpace.  The StringSpace struct will contain a vector of bytes that will be used to store the strings.  The vector will be used to keep track of the active bytes in the heap.  The active bytes will be the number of bytes in the vector that are currently being referenced by the StringRefs in the vector.  The heap will be allocated in blocks of size BLOCK_SIZE.  When initializing the StringSpace, the heap will be allocated with a block of size BLOCK_SIZE.

There must always be enough space in the heap to store one new string of MAX_CHARS characters.

If there is not enough space in the heap to store a new string, a new heap will be allocated with size equal to current size plus BLOCK_SIZE.  The StringRefs will be iterated and their string data will be copied from the old heap to the new heap contiguously and the pointers will be updated to reference the new heap.

If the heap size minus the active bytes is less than BLOCK_SIZE*2, a new heap will be allocated with size equal to the current size minus BLOCK_SIZE.  The StringRefs will be iterated and their string data will be copied from the old heap to the new heap contiguously and the pointers will be updated to reference the new heap.


>>>>>>>>>>>>  GPT <<<<<<<<<<<<<

### Steps for Implementation

1. **Define `StringRef`:**
   - Create a struct to hold metadata for each string:
     - Pointer (offset) to the string in the buffer.
     - Length of the string (in bytes).
     - Prefix of the first 3 characters (up to 12 bytes).
     - Frequency counter.

2. **Define `StringSpace`:**
   - Create a struct to manage the buffer and `StringRef` metadata:
     - `buffer`: A `Vec<u8>` to store the strings contiguously.
     - `used_bytes`: Tracks how much of the buffer is currently allocated (even if some strings are removed).
     - `active_bytes`: Tracks the actual used memory by active strings.
     - `string_refs`: A `Vec<StringRef>` to track each string in the buffer.

3. **Implement String Insertion:**
   - Write a function to add a string:
     - Check if there is enough space in the buffer. If not, resize it.
     - Copy the string to the buffer.
     - Update the `StringRef` for the new string and append it to `string_refs`.
     - Update `used_bytes` and `active_bytes`.

4. **Implement String Removal:**
   - Write a function to remove a string:
     - Remove the corresponding `StringRef` from the `string_refs` vector.
     - Update only `active_bytes`, leaving the memory in the buffer untouched for now.

5. **Heap Management:**
   - Resizing the buffer when there isn’t enough space.

### Code Implementation

```rust
use std::str;

const MIN_CHARS: usize = 3;
const MAX_CHARS: usize = 50;
const BLOCK_SIZE: usize = 1024;

#[derive(Debug)]
struct StringRef {
    pointer: usize,  // Byte offset in the buffer
    length: usize,   // Length of the string in bytes
    prefix: [u8; 12],  // First 3 characters of the string (up to 12 bytes in UTF-8)
    frequency: u32,  // Frequency counter (meta data)
}

#[derive(Debug)]
struct StringSpace {
    buffer: Vec<u8>,           // Buffer to store UTF-8 strings contiguously
    used_bytes: usize,         // Number of bytes used in the buffer (even if some strings are removed)
    active_bytes: usize,       // Number of bytes currently used by active strings
    string_refs: Vec<StringRef>,  // Metadata about each string in the buffer
}

impl StringSpace {
    // Create a new StringSpace with a pre-allocated buffer
    fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(BLOCK_SIZE),
            used_bytes: 0,
            active_bytes: 0,
            string_refs: Vec::new(),
        }
    }

    // Insert a string into the buffer
    fn insert_string(&mut self, string: &str) {
        let string_bytes = string.as_bytes();
        let length = string_bytes.len();

        // Ensure string length is within limits
        assert!(length >= MIN_CHARS, "String too short.");
        assert!(length <= MAX_CHARS, "String too long.");

        // Resize the buffer if necessary
        if self.buffer.len() - self.used_bytes < length {
            self.grow_buffer(length);
        }

        // Copy string into the buffer at the end of active bytes
        self.buffer[self.used_bytes..self.used_bytes + length].copy_from_slice(string_bytes);

        // Create a prefix (up to 12 bytes, first 3 characters)
        let mut prefix = [0u8; 12];
        let prefix_length = std::cmp::min(length, 12);
        prefix[..prefix_length].copy_from_slice(&string_bytes[..prefix_length]);

        // Create StringRef
        let string_ref = StringRef {
            pointer: self.used_bytes,
            length,
            prefix,
            frequency: 1,
        };

        // Add StringRef to the list
        self.string_refs.push(string_ref);

        // Update used and active bytes
        self.used_bytes += length;
        self.active_bytes += length;
    }

    // Remove a string (by index in string_refs)
    fn remove_string(&mut self, index: usize) {
        if index < self.string_refs.len() {
            // Remove the StringRef (but keep the memory unchanged for now)
            let string_ref = self.string_refs.remove(index);
            self.active_bytes -= string_ref.length;
        } else {
            println!("Invalid index for removal.");
        }
    }

    // Grow the buffer by at least the required size
    fn grow_buffer(&mut self, required_size: usize) {
        let new_size = self.buffer.len() + std::cmp::max(BLOCK_SIZE, required_size);
        self.buffer.resize(new_size, 0);
    }

    // Utility function to print the strings in the buffer (for debugging)
    fn print_strings(&self) {
        for string_ref in &self.string_refs {
            let string_bytes = &self.buffer[string_ref.pointer..string_ref.pointer + string_ref.length];
            if let Ok(string) = str::from_utf8(string_bytes) {
                println!("String: {}, Frequency: {}", string, string_ref.frequency);
            } else {
                println!("Invalid UTF-8 string at pointer {}", string_ref.pointer);
            }
        }
    }
}

fn main() {
    let mut space = StringSpace::new();

    // Insert strings
    space.insert_string("hello");
    space.insert_string("world");
    space.insert_string("rust");

    // Print strings
    space.print_strings();

    // Remove a string and print again
    space.remove_string(1);
    println!("After removal:");
    space.print_strings();
}
```

### Code Walkthrough

1. **StringRef Struct:**
   - Holds the pointer (as an offset), the string’s length, a prefix (up to 12 bytes), and a frequency counter.

2. **StringSpace Struct:**
   - Manages the buffer (`Vec<u8>`) and the collection of `StringRef`s.
   - It tracks `used_bytes` (including removed strings) and `active_bytes` (currently used by live strings).

3. **insert_string Method:**
   - Adds a string to the buffer, ensuring there’s enough space. If not, the buffer is resized.
   - The string is copied into the buffer, and a `StringRef` is created to track it.

4. **remove_string Method:**
   - Removes the `StringRef` from the list, but the memory in the buffer is not freed immediately.
   - This simulates a "lazy" memory management strategy.

5. **grow_buffer Method:**
   - Resizes the buffer by growing it by `BLOCK_SIZE` (or more if needed).

### Complexity Evaluation:
- **Memory Handling:** Memory management complexity arises from the need to resize buffers and update pointers (though this is abstracted using Rust's `Vec`).
- **Operations:** Insertion and removal are relatively straightforward. However, memory is not compacted after removal, which may require a separate function to defragment the buffer over time.
- **Ease of Use:** The complexity is manageable, but it increases as more advanced features (like compaction or concurrency) are introduced.

Let me know if you’d like to explore more advanced versions or specific optimizations.