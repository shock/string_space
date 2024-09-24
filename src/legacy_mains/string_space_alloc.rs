use std::env;
use std::time::Instant;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::str;
use rand::Rng;
use rand::distributions::Alphanumeric;

const MIN_CHARS: usize = 3;
const MAX_CHARS: usize = 50;
const INITIAL_HEAP_SIZE: usize = 4096; // 4KB
const ALIGNMENT: usize = 4096; // 4KB alignment

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
        let string_bytes = unsafe {
            std::slice::from_raw_parts(
                self.space.buffer.add(self.pointer),
                self.length
            )
        };
        str::from_utf8(string_bytes).ok().map(String::from)
    }
}

#[derive(Debug)]
struct StringSpace {
    buffer: *mut u8,
    capacity: usize,
    used_bytes: usize,
    active_bytes: usize,
    string_refs: Vec<StringRefInfo>,
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
        let layout = Layout::from_size_align(INITIAL_HEAP_SIZE, ALIGNMENT).unwrap();
        let buffer = unsafe { alloc(layout) };

        Self {
            buffer,
            capacity: INITIAL_HEAP_SIZE,
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

        if self.capacity - self.used_bytes < length {
            self.grow_buffer(length);
        }

        unsafe {
            ptr::copy_nonoverlapping(
                string_bytes.as_ptr(),
                self.buffer.add(self.used_bytes),
                length
            );
        }

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
        let new_capacity = self.capacity + std::cmp::max(self.capacity, required_size);
        let new_layout = Layout::from_size_align(new_capacity, ALIGNMENT).unwrap();
        let new_buffer = unsafe { alloc(new_layout) };

        unsafe {
            ptr::copy_nonoverlapping(self.buffer, new_buffer, self.used_bytes);
            dealloc(self.buffer, Layout::from_size_align(self.capacity, ALIGNMENT).unwrap());
        }

        self.buffer = new_buffer;
        self.capacity = new_capacity;
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
            let string_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.buffer.add(string_ref_info.pointer),
                    string_ref_info.length
                )
            };
            if let Ok(string) = str::from_utf8(string_bytes) {
                println!("String: {}, Frequency: {}", string, string_ref_info.frequency);
            } else {
                println!("Invalid UTF-8 string at pointer {}", string_ref_info.pointer);
            }
        }
    }
}

impl Drop for StringSpace {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.buffer, Layout::from_size_align(self.capacity, ALIGNMENT).unwrap());
        }
    }
}

fn generate_random_words(num_words: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();

    // Preallocate the Vec<String> with the specified capacity
    let mut words = Vec::with_capacity(num_words);

    for _ in 0..num_words {
        let word_length = rng.gen_range(3..=20); // Random length between 3 and 20
        let word: String = (0..word_length)
            .map(|_| rng.sample(Alphanumeric) as char) // Generate random characters
            .collect(); // Collect into a String

        words.push(word); // Add the generated word to the Vec
    }

    words // Return the Vec<String>
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <file> <num>", args[0]);
        std::process::exit(1);
    }
    // let insert_remove_words_count = 100;

    // let file_path = &args[1];
    let num_words: usize = args[2].parse().expect("Invalid number of words");
    let mut space = StringSpace::new();

        // GENRATE RANDOM WORDS
    // let start = Instant::now();
    let mut words = generate_random_words(num_words);
    // let gen_time = start.elapsed();
    words.sort();

    // Insert strings
    for word in words.iter() {
        space.insert_string(word);
    }

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