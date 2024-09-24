use std::str;

const MIN_CHARS: usize = 3;
const MAX_CHARS: usize = 50;
const BLOCK_SIZE: usize = 1024;

#[derive(Debug)]
struct StringRef<'a> {
    space: &'a StringSpace,
    pointer: usize,  // Byte offset in the buffer
    length: usize,   // Length of the string in bytes
    prefix: [u8; 12],  // First 3 characters of the string (up to 12 bytes in UTF-8)
    frequency: u32,  // Frequency counter (meta data)
}

impl<'a> StringRef<'a> {
    fn get_string(&self) -> Option<String> {
        let string_bytes = &self.space.buffer[self.pointer..self.pointer + self.length];
        str::from_utf8(string_bytes).ok().map(String::from)
    }
}

#[derive(Debug)]
struct StringSpace {
    buffer: Vec<u8>,           // Buffer to store UTF-8 strings contiguously
    used_bytes: usize,         // Number of bytes used in the buffer (even if some strings are removed)
    active_bytes: usize,       // Number of bytes currently used by active strings
    string_refs: Vec<StringRefInfo>,  // Metadata about each string in the buffer
}

#[derive(Debug)]
struct StringRefInfo {
    pointer: usize,
    length: usize,
    prefix: [u8; 12],
    frequency: u32,
}

impl StringSpace {
    fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(BLOCK_SIZE),
            used_bytes: 0,
            active_bytes: 0,
            string_refs: Vec::new(),
        }
    }

    fn insert_string(&mut self, string: &str) {
        let string_bytes = string.as_bytes();
        let length = string_bytes.len();

        assert!(length >= MIN_CHARS, "String too short.");
        assert!(length <= MAX_CHARS, "String too long.");

        if self.buffer.len() - self.used_bytes < length {
            self.grow_buffer(length);
        }

        self.buffer[self.used_bytes..self.used_bytes + length].copy_from_slice(string_bytes);

        let mut prefix = [0u8; 12];
        let prefix_length = std::cmp::min(length, 12);
        prefix[..prefix_length].copy_from_slice(&string_bytes[..prefix_length]);

        let string_ref_info = StringRefInfo {
            pointer: self.used_bytes,
            length,
            prefix,
            frequency: 1,
        };

        self.string_refs.push(string_ref_info);

        self.used_bytes += length;
        self.active_bytes += length;
    }

    fn grow_buffer(&mut self, required_size: usize) {
        let new_size = self.buffer.len() + std::cmp::max(BLOCK_SIZE, required_size);
        self.buffer.resize(new_size, 0);
    }

    fn find_by_prefix<'a>(&'a self, search_prefix: &str) -> Vec<StringRef<'a>> {
        let search_bytes = search_prefix.as_bytes();
        let search_len = search_bytes.len();
        let mut result = Vec::new();

        for string_ref_info in &self.string_refs {
            let prefix_len = std::cmp::min(12, std::cmp::min(string_ref_info.length, search_len));
            if &string_ref_info.prefix[..prefix_len] == &search_bytes[..prefix_len] {
                result.push(StringRef {
                    space: self,
                    pointer: string_ref_info.pointer,
                    length: string_ref_info.length,
                    prefix: string_ref_info.prefix,
                    frequency: string_ref_info.frequency,
                });
            }
        }

        result
    }

    fn print_strings(&self) {
        for string_ref_info in &self.string_refs {
            if let Ok(string) = str::from_utf8(&self.buffer[string_ref_info.pointer..string_ref_info.pointer + string_ref_info.length]) {
                println!("String: {}, Frequency: {}", string, string_ref_info.frequency);
            } else {
                println!("Invalid UTF-8 string at pointer {}", string_ref_info.pointer);
            }
        }
    }
}

fn main() {
    let mut space = StringSpace::new();

    // Insert strings
    space.insert_string("hello");
    space.insert_string("helicopter");
    space.insert_string("help");
    space.insert_string("harmony");
    space.insert_string("hero");
    space.insert_string("rust");

    // Print all strings
    space.print_strings();

    // Search by prefix
    let found_strings = space.find_by_prefix("he");
    println!("Strings with prefix 'he':");
    for string_ref in found_strings {
        if let Some(string) = string_ref.get_string() {
            println!("{}", string);
        }
    }
}