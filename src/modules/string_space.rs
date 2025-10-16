use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter, BufRead};
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};

const MIN_CHARS: usize = 3;
const MAX_CHARS: usize = 50;
const INITIAL_HEAP_SIZE: usize = 4096*256; // 4KB
const ALIGNMENT: usize = 4096; // 4KB alignment

pub struct StringSpace {
    inner: StringSpaceInner,
}

struct StringSpaceInner {
    buffer: *mut u8,
    capacity: usize,
    used_bytes: usize,
    string_refs: Vec<StringRefInfo>,
}

#[derive(Debug, Clone)]
struct StringRefInfo {
    pointer: usize,
    length: usize,
    meta: StringMeta,
}

type TFreq = u16;
type TAgeDays = u32;

#[derive(Debug, Clone)]
pub struct StringMeta {
    pub frequency: TFreq,
    pub age_days: TAgeDays,
}

// #[derive(Debug, Clone)]
#[allow(unused)]
pub struct StringRef {
    pub string: String,
    pub meta: StringMeta,
}

impl StringSpace {
    pub fn new() -> Self {
        Self {
            inner: StringSpaceInner::new(),
        }
    }

    #[allow(unused)]
    pub fn insert_string(&mut self, string: &str, frequency: TFreq) -> Result<(), &'static str> {
        if string.len() < MIN_CHARS || string.len() > MAX_CHARS {
            return Err("String length out of bounds");
        }
        self.inner.insert_string(string, frequency, None)
    }

    #[allow(unused)]
    pub fn find_by_prefix(&self, prefix: &str) -> Vec<StringRef> {
        self.inner.find_by_prefix(prefix)
    }

    #[allow(unused)]
    pub fn get_similar_words(&self, word: &str, cutoff: Option<f64>) -> Vec<StringRef> {
        let cutoff = cutoff.unwrap_or(0.6);
        if word.len() < 2 {
            return Vec::new();
        }
        let possibilities = self.inner.find_by_prefix_no_sort(word[0..1].to_string().as_str());
        // let matches = get_close_matches_levenshtein(word, &possibilities, threshold);
        let matches = get_close_matches(word, &possibilities, 15, cutoff);
        matches
    }

    #[allow(unused)]
    pub fn write_to_file(&self, file_path: &str) -> io::Result<()> {
        self.inner.write_to_file(file_path)
    }

    #[allow(unused)]
    pub fn read_from_file(&mut self, file_path: &str) -> io::Result<()> {
        self.inner.read_from_file(file_path)
    }

    #[allow(unused)]
    pub fn print_strings(&self) {
        self.inner.print_strings();
    }

    #[allow(unused)]
    pub fn empty(&self) -> bool {
        self.inner.empty()
    }

    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[allow(unused)]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[allow(unused)]
    pub fn clear_space(&mut self) {
        self.inner.clear_space()
    }

    #[allow(unused)]
    pub fn sort(&mut self) {
        self.inner.sort()
    }

    #[allow(unused)]
    pub fn find_with_substring(&self, substring: &str) -> Vec<StringRef> {
        self.inner.find_with_substring(substring)
    }

    #[allow(unused)]
    pub fn fuzzy_subsequence_search(&self, query: &str) -> Vec<StringRef> {
        self.inner.fuzzy_subsequence_search(query)
    }

    #[allow(unused)]
    pub fn get_all_strings(&self) -> Vec<StringRef> {
        self.inner.get_all_strings()
    }

}

impl StringRefInfo {
    #[allow(unused)]
    fn string_ref(&self, string_space_inner: &StringSpaceInner) -> &str {
        let string_bytes = unsafe {
            std::slice::from_raw_parts(
                string_space_inner.buffer.add(self.pointer),
                self.length
            )
        };
        str::from_utf8(string_bytes).unwrap()
    }
}

impl StringSpaceInner {
    fn new() -> Self {
        let layout = Layout::from_size_align(INITIAL_HEAP_SIZE, ALIGNMENT).unwrap();
        let buffer = unsafe { alloc(layout) };

        Self {
            buffer,
            capacity: INITIAL_HEAP_SIZE,
            used_bytes: 0,
            string_refs: Vec::new(),
        }
    }

    fn sort(&mut self) {
        // Sort the string_refs by the strings they point to
        self.string_refs.sort_by(|a, b| {
            let a_str = unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    self.buffer.add(a.pointer),
                    a.length
                ))
            };
            let b_str = unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    self.buffer.add(b.pointer),
                    b.length
                ))
            };
            a_str.cmp(b_str)
        });
    }

    fn empty(&self) -> bool {
        self.string_refs.is_empty()
    }

    fn len(&self) -> usize {
        self.string_refs.len()
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn clear_space(&mut self) {
        self.string_refs.clear();
        self.used_bytes = 0;
    }

    fn binary_search<F>(&self, target: &[u8], compare: F) -> usize
    where
        F: Fn(&[u8], &[u8]) -> std::cmp::Ordering,
    {
        let mut left = 0;
        let mut right = self.string_refs.len();

        while left < right {
            let mid = left + (right - left) / 2;
            let ref_info = &self.string_refs[mid];
            let existing_string_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.buffer.add(ref_info.pointer),
                    ref_info.length
                )
            };

            match compare(existing_string_bytes, target) {
                std::cmp::Ordering::Less => left = mid + 1,
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => right = mid,
            }
        }

        left
    }

    fn insert_string(&mut self, string: &str, frequency: TFreq, age_days: Option<TAgeDays>) -> Result<(), &'static str> {
        let string_bytes = string.as_bytes();

        let index = self.binary_search(string_bytes, |a, b| a.cmp(b));

        if index < self.string_refs.len() {
            let existing_ref = &self.string_refs[index];
            let existing_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.buffer.add(existing_ref.pointer),
                    existing_ref.length
                )
            };
            if existing_bytes == string_bytes {
                self.string_refs[index].meta.frequency += frequency;
                self.string_refs[index].meta.age_days = days_since_epoch();
                return Ok(());
            }
        }

        let length = string.len();

        if self.capacity - self.used_bytes < length {
            self.grow_buffer(length);
        }

        unsafe {
            ptr::copy_nonoverlapping(
                string.as_ptr(),
                self.buffer.add(self.used_bytes),
                length
            );
        }

        let string_ref_info = StringRefInfo {
            pointer: self.used_bytes,
            length,
            meta: StringMeta {
                frequency: frequency,
                age_days: age_days.unwrap_or_else(|| days_since_epoch()),
            }
        };

        self.string_refs.insert(index, string_ref_info);

        self.used_bytes += length;
        Ok(())
    }

    fn get_all_strings(&self) -> Vec<StringRef> {
        let mut results = Vec::new();
        for ref_info in &self.string_refs {
            let string_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.buffer.add(ref_info.pointer),
                    ref_info.length
                )
            };

            results.push(StringRef {
                string: String::from_utf8(string_bytes.to_vec()).unwrap(),
                meta: ref_info.meta.clone(),
            });
        }
        results
    }

    fn find_by_prefix(&self, search_prefix: &str) -> Vec<StringRef> {
        let mut results = self.find_by_prefix_no_sort(search_prefix);
        // reverse sort the results by frequency
        results.sort_by(|a, b| b.meta.frequency.cmp(&a.meta.frequency));
        results
    }

    fn find_by_prefix_alpha_sort(&self, search_prefix: &str) -> Vec<StringRef> {
        let mut results = self.find_by_prefix_no_sort(search_prefix);
        results.sort_by(|a, b| a.string.cmp(&b.string));
        results
    }

    fn find_by_prefix_no_sort(&self, search_prefix: &str) -> Vec<StringRef> {
        let search_bytes = search_prefix.as_bytes();
        let search_len = search_bytes.len();
        let mut results = Vec::new();
        if search_prefix.is_empty() {
            return results;
        }

        // find the index of the string that is or preceeds the first string matching the prefix
        let start_index = self.binary_search(search_bytes, |string, prefix| {
            let prefix_len = std::cmp::min(string.len(), prefix.len());
            let compare = string[..prefix_len].cmp(&prefix[..prefix_len]);
            if compare == std::cmp::Ordering::Equal {
                // if the comparison is equal, check if the string is longer than the prefix
                if string.len() > prefix.len() { return std::cmp::Ordering::Greater; }
            }
            compare
        });

        let mut matched_once = false;
        for index in start_index..self.string_refs.len() {
            let ref_info = &self.string_refs[index];
            let string_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.buffer.add(ref_info.pointer),
                    ref_info.length
                )
            };

            // if the string is shorter than the prefix, skip it
            if string_bytes.len() < search_len {
                continue;
            }

            if &string_bytes[..search_len] == &search_bytes[..search_len] {
                matched_once = true;
                if let Ok(string) = std::str::from_utf8(string_bytes) {
                    results.push(StringRef {
                        string: string.to_string(),
                        meta: ref_info.meta.clone(),
                    });
                } else {
                    println!("Invalid UTF-8 string at pointer {}", ref_info.pointer);
                }
            } else {
                if matched_once {
                    // No longer matching, break out of the loop
                    break;
                }
            }
        }
        results
    }

    fn grow_buffer(&mut self, required_size: usize) {
        // println!("Doubling buffer...");
        let new_capacity = self.capacity + std::cmp::max(self.capacity, required_size);
        let new_layout = Layout::from_size_align(new_capacity, ALIGNMENT).unwrap();
        let new_buffer = unsafe { alloc(new_layout) };

        let mut new_used_bytes = 0;
        for ref_info in self.string_refs.iter_mut() {
            let old_slice = unsafe {
                std::slice::from_raw_parts(self.buffer.add(ref_info.pointer), ref_info.length)
            };
            unsafe {
                ptr::copy_nonoverlapping(
                    old_slice.as_ptr(),
                    new_buffer.add(new_used_bytes),
                    ref_info.length
                );
            }
            ref_info.pointer = new_used_bytes;
            new_used_bytes += ref_info.length;
        }

        unsafe {
            dealloc(self.buffer, Layout::from_size_align(self.capacity, ALIGNMENT).unwrap());
        }

        self.buffer = new_buffer;
        self.capacity = new_capacity;
        self.used_bytes = new_used_bytes;
    }

    fn find_with_substring(&self, substring: &str) -> Vec<StringRef> {
        let mut results = Vec::new();
        if substring.is_empty() {
            return results;
        }

        for string_ref_info in &self.string_refs {
            let string_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.buffer.add(string_ref_info.pointer),
                    string_ref_info.length
                )
            };

            if let Ok(string) = std::str::from_utf8(string_bytes) {
                if string.contains(substring) {
                    results.push(StringRef {
                        string: string.to_string(),
                        meta: string_ref_info.meta.clone(),
                    });
                }
            } else {
                println!("Invalid UTF-8 string at pointer {}", string_ref_info.pointer);
            }
        }

        // reverse sort the results by frequency
        results.sort_by(|a, b| b.meta.frequency.cmp(&a.meta.frequency));
        results
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
                println!("String: {}, TFreq: {}", string, string_ref_info.meta.frequency);
            } else {
                println!("Invalid UTF-8 string at pointer {}", string_ref_info.pointer);
            }
        }
    }

    fn write_to_file(&self, file_path: &str) -> io::Result<()> {
        let file = File::create(file_path)?;
        let mut writer = BufWriter::new(file);
        for string_ref_info in &self.string_refs {
            let string_bytes = unsafe {
                std::slice::from_raw_parts(
                    self.buffer.add(string_ref_info.pointer),
                    string_ref_info.length
                )
            };
            if let Ok(string) = str::from_utf8(string_bytes) {
                writeln!(writer, "{} {} {}", string, string_ref_info.meta.frequency, string_ref_info.meta.age_days)?;
            } else {
                println!("Invalid UTF-8 string at pointer {}", string_ref_info.pointer);
            }
        }
        Ok(())
    }

    fn read_from_file(&mut self, file_path: &str) -> io::Result<()> {

        self.clear_space();

        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();

        while reader.read_until(b'\n', &mut buffer)? > 0 {
            let input = str::from_utf8(&buffer).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8")
            })?;

            // split the input using whitespace as the delimiter
            let mut parts: Vec<&str> = input.split_whitespace().collect();
            while parts.len() < 3 {
                parts.push("");
            }
            let string = parts[0];
            let frequency = parts[1].parse::<TFreq>().unwrap_or_else(|_| {1});
            let age_days = parts[2].parse::<TAgeDays>().unwrap_or_else(|_| {days_since_epoch()});
            let _ = self.insert_string(string.trim(), frequency, Some(age_days));

            buffer.clear();
        }
        Ok(())
    }

    fn fuzzy_subsequence_search(&self, query: &str) -> Vec<StringRef> {
        // Empty query handling: return empty vector for empty queries
        // This is consistent with existing search method behavior where empty queries yield no matches
        if query.is_empty() {
            return Vec::new();
        }

        // Use prefix filtering like get_similar_words for performance
        // Identical implementation to get_similar_words()
        let possibilities = self.find_by_prefix_no_sort(query[0..1].to_string().as_str());

        let mut matches: Vec<(StringRef, f64)> = Vec::new();

        for candidate in possibilities {
            if let Some(match_indices) = is_subsequence(query, &candidate.string) {
                let score = score_match_span(&match_indices, &candidate.string);
                matches.push((candidate, score));
            }
        }

        // Sort by score (ascending - lower scores are better), then frequency (descending), then age (descending)
        matches.sort_by(|a, b| {
            a.1.partial_cmp(&b.1).unwrap()
                // .then(b.0.meta.frequency.cmp(&a.0.meta.frequency))
                // .then(b.0.meta.age_days.cmp(&a.0.meta.age_days))
        });

        // Limit to top 10 results AFTER all sorting is complete
        // This ensures the best 10 matches are selected based on the full sorting criteria
        matches.truncate(10);

        // print the matches for debugging
        println!("Matches post-sorting:");
        for (string_ref, score) in &matches {
            println!("String: {}, Score: {}, TFreq: {}, AgeDays: {}", string_ref.string, score, string_ref.meta.frequency, string_ref.meta.age_days);
        }

        matches.into_iter().map(|(string_ref, _)| string_ref).collect()
    }

}

impl Drop for StringSpaceInner {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.buffer, Layout::from_size_align(self.capacity, ALIGNMENT).unwrap());
        }
    }
}

// MARK: Private Functions

fn days_since_epoch() -> TAgeDays {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() / (60 * 60 * 24)).try_into().unwrap()
}

use strsim::levenshtein;

#[allow(unused)]
// this is fast and simple, but not easy to use because it scores based on the Levenshtein distance
// which is difficult to normalize.
fn get_close_matches_levenshtein(query: &str, possibilities: &[StringRef], threshold: usize) -> Vec<StringRef> {
    let mut matches: Vec<(String, usize, TFreq, TAgeDays)> = Vec::new();

    for string_ref in possibilities {
        let word = string_ref.string.as_str();
        let distance = levenshtein(query, word);
        if distance <= threshold {
            matches.push((word.to_string(), distance, string_ref.meta.frequency, string_ref.meta.age_days));
        }
    }

    // Sort matches by age (youngest first)
    matches.sort_by_key(|(_, _, _, age_days)| std::cmp::Reverse(*age_days));

    // Sort matches by frequency (highest first)
    matches.sort_by_key(|(_, _, frequency, _)| std::cmp::Reverse(*frequency));

    // Sort matches by distance (lower distance first)
    matches.sort_by_key(|(_, distance, _, _)| *distance);

    // map to StringRef and return
    matches.into_iter().map(|(word, _, frequency, age_days)| StringRef {
        string: word,
        meta: StringMeta {
            frequency,
            age_days,
        },
    }).collect()
}

use std::collections::HashSet;

#[allow(unused)]
// this is fast, simple and quite functional, but not quite as good as the Jaro-Winkler
// metric.
// you also need to double the score to approximate a range equivalent to the Jaro-Winkler
// metric.
fn similar(a: &str, b: &str) -> f64 {
    let a_len = a.len();
    let b_len = b.len();
    let mut matches = 0;

    let a_chars: HashSet<_> = a.chars().collect();
    let b_chars: HashSet<_> = b.chars().collect();

    for c in a_chars.intersection(&b_chars) {
        matches += a.matches(*c).count().min(b.matches(*c).count());
    }

    if a_len == 0 || b_len == 0 {
        return 0.0;
    }

    matches as f64 / (a_len + b_len) as f64
}

use jaro_winkler::jaro_winkler;

fn get_close_matches(word: &str, possibilities: &[StringRef], n: usize, cutoff: f64) -> Vec<StringRef> {
    let mut matches: Vec<(String, u32, TFreq, TAgeDays)> = Vec::new();

    for string_ref in possibilities {
        let possibility = string_ref.string.as_str();
        // let score = similar(word, possibility) * 2.0;
        let score = jaro_winkler(word, possibility);
        if score > cutoff {
            // println!("word: {}, possibility: {}, score: {}", word, possibility, score);
            matches.push((possibility.to_string(), (score*4294967296.0) as u32, string_ref.meta.frequency, string_ref.meta.age_days));
        }
    }

    // Sort matches by score (higher score first)
    matches.sort_by_key(|(_, score, _, _)| std::cmp::Reverse(*score));

    matches.truncate(n); // Limit to the top n matches

    // Sort matches by age (youngest first)
    matches.sort_by_key(|(_, _, _, age_days)| std::cmp::Reverse(*age_days));

    // Sort matches by frequency (highest first)
    matches.sort_by_key(|(_, _, frequency, _)| std::cmp::Reverse(*frequency));

    // map to StringRef and return
    let matches: Vec<StringRef> = matches.into_iter().map(|(word, _, frequency, age_days)| StringRef {
        string: word,
        meta: StringMeta {
            frequency,
            age_days,
        },
    }).collect();

    matches
}

fn is_subsequence(query: &str, candidate: &str) -> Option<Vec<usize>> {
    let mut query_chars = query.chars();
    let mut current_char = query_chars.next();
    let mut match_indices = Vec::new();

    // UTF-8 Character Handling: Use chars() iterator for proper Unicode character-by-character matching
    // This correctly handles multi-byte UTF-8 sequences like emoji, accented characters, etc.
    for (i, c) in candidate.chars().enumerate() {
        if current_char == Some(c) {
            match_indices.push(i);
            current_char = query_chars.next();
            if current_char.is_none() {
                return Some(match_indices);
            }
        }
    }

    None
}

fn score_match_span(match_indices: &[usize], candidate: &str) -> f64 {
    if match_indices.is_empty() {
        return f64::MAX;
    }
    let span_length = (match_indices.last().unwrap() - match_indices.first().unwrap() + 1) as f64;
    let candidate_length = candidate.len() as f64;
    span_length + (candidate_length * 0.1)
}

// MARK: Unit Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_string_space() {
        let ss = StringSpace::new();
        assert!(ss.empty());
        assert_eq!(ss.len(), 0);
        assert_eq!(ss.capacity(), INITIAL_HEAP_SIZE);
    }

    #[test]
    fn test_insert_string() {
        let mut ss = StringSpace::new();
        assert!(ss.insert_string("hello", 1).is_ok());
        assert_eq!(ss.len(), 1);
        assert!(!ss.empty());
    }

    #[test]
    fn test_insert_duplicate_string() {
        let mut ss = StringSpace::new();
        assert!(ss.insert_string("hello", 1).is_ok());
        assert!(ss.insert_string("hello", 1).is_ok());
        assert_eq!(ss.len(), 1);
        let results = ss.find_by_prefix("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].meta.frequency, 2);
    }

    #[test]
    fn test_insert_string_length_bounds() {
        let mut ss = StringSpace::new();
        assert!(ss.insert_string("ab", 1).is_err());  // Too short
        assert!(ss.insert_string("a".repeat(51).as_str(), 1).is_err());  // Too long
        assert!(ss.insert_string("abc", 1).is_ok());  // Just right
        assert!(ss.insert_string("a".repeat(50).as_str(), 1).is_ok());  // Maximum length
    }

    #[test]
    fn test_find_with_substring() {
        let mut ss = StringSpace::new();
        ss.insert_string("hello", 1).unwrap();
        ss.insert_string("world", 2).unwrap();
        ss.insert_string("low", 2).unwrap();
        ss.insert_string("blow", 1).unwrap();

        let results = ss.find_with_substring("lo");
        assert_eq!(results.len(), 3);
        assert!(results[0].string == "low");
        assert!(results[1].string == "blow");
        assert!(results[2].string == "hello");
    }

    #[test]
    fn test_find_with_substring_empty_substring() {
        let mut ss = StringSpace::new();
        ss.insert_string("hello", 1).unwrap();
        ss.insert_string("world", 2).unwrap();
        ss.insert_string("low", 2).unwrap();
        ss.insert_string("blow", 1).unwrap();

        let results = ss.find_with_substring("");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_clear_space() {
        let mut ss = StringSpace::new();
        ss.insert_string("hello", 1).unwrap();
        ss.insert_string("world", 1).unwrap();
        assert_eq!(ss.len(), 2);

        ss.clear_space();
        assert!(ss.empty());
        assert_eq!(ss.len(), 0);
    }

    #[test]
    fn test_sort() {
        let mut ss = StringSpace::new();
        ss.insert_string("zebra", 1).unwrap();
        ss.insert_string("apple", 1).unwrap();
        ss.insert_string("banana", 1).unwrap();

        ss.sort();
        let results = ss.get_all_strings();
        assert_eq!(ss.len(), 3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].string, "apple");
        assert_eq!(results[1].string, "banana");
        assert_eq!(results[2].string, "zebra");
    }

    #[test]
    fn test_write_and_read_file() -> io::Result<()> {
        let mut ss = StringSpace::new();
        ss.insert_string("hello", 1).unwrap();
        ss.insert_string("world", 2).unwrap();

        ss.write_to_file("test/test_output.txt")?;

        let mut new_ss = StringSpace::new();
        new_ss.read_from_file("test/test_output.txt")?;

        assert_eq!(new_ss.len(), 2);
        let results = new_ss.get_all_strings();
        assert!(results.iter().any(|r| r.string == "hello" && r.meta.frequency == 1));
        assert!(results.iter().any(|r| r.string == "world" && r.meta.frequency == 2));

        std::fs::remove_file("test/test_output.txt")?;
        Ok(())
    }

    #[test]
    fn test_grow_buffer() {
        let mut ssi = StringSpaceInner::new();
        let long_string = "a".repeat(INITIAL_HEAP_SIZE+1);
        assert!(ssi.insert_string(&long_string, 1, None).is_ok());
        assert!(ssi.capacity() > INITIAL_HEAP_SIZE);
    }

    mod find_by_prefix {
        use super::*;

        #[test]
        fn test_find_by_prefix() {
            let mut ss = StringSpace::new();
            ss.insert_string("apple", 1).unwrap();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 2).unwrap();
            ss.insert_string("helicopter", 1).unwrap();
            ss.insert_string("world", 1).unwrap();

            let results = ss.find_by_prefix("hel");
            for r in &results {
                println!("found string: {}", r.string);
            }
            assert_eq!(results.len(), 3);
            // results are sorted by reverse frequency primary, then alphanumeric order secondary
            assert!(results[0].string == "help");  // highest frequency
            assert!(results[1].string == "helicopter");  // lower same frequency alphabetically first
            assert!(results[2].string == "hello");  // lowest same frequency alphabetically last
        }

        #[test]
        fn prefix_excludes() {
            // this is to verify a bug-fix where the prefix search was returning strings that
            // were a prefix of the prefix being searched for! ;)
            let mut ss = StringSpace::new();
            ss.insert_string("test", 1).unwrap();
            ss.insert_string("testing", 1).unwrap();

            let results = ss.find_by_prefix("testi");
            for r in &results {
                println!("found string: {}", r.string);
            }
            assert_eq!(results.len(), 1);
            assert!(results[0].string == "testing");
        }

        #[test]  // test find_by_prefix with empty prefix
        fn test_find_by_prefix_empty_prefix() {
            let mut ss = StringSpace::new();
            ss.insert_string("apple", 1).unwrap();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 2).unwrap();
            ss.insert_string("helicopter", 1).unwrap();
            ss.insert_string("world", 1).unwrap();

            let results = ss.find_by_prefix("");
            assert_eq!(results.len(), 0);
        }
    }

    mod get_similar_words {
        use super::*;

        #[test]
        fn test_get_similar_words() {
            let mut ssi = StringSpaceInner::new();
            ssi.insert_string("apple", 1, Some(0)).unwrap();
            ssi.insert_string("hello1", 1, Some(1)).unwrap();
            ssi.insert_string("hello2", 1, Some(2)).unwrap();
            ssi.insert_string("hello3", 3, Some(2)).unwrap();
            ssi.insert_string("help", 1, Some(0)).unwrap();
            ssi.insert_string("helicopter", 1, Some(0)).unwrap();
            ssi.insert_string("world", 1, Some(0)).unwrap();

            let ss = StringSpace {
                inner: ssi,
            };
            let results = ss.get_similar_words("hell", Some(0.6));
            for r in &results {
                println!("found string: {}", r.string);
            }
            assert_eq!(results.len(), 5);
            assert!(results[0].string == "hello3");  // highest frequency
            assert!(results[1].string == "hello2");  // highest age
            assert!(results[2].string == "hello1");  // same frequency and age
            assert!(results[3].string == "help");  // lower score
            assert!(results[4].string == "helicopter");  // lowest score
        }
    }

    mod fuzzy_subsequence_search {
        use super::*;

        #[test]
        fn test_fuzzy_subsequence_search() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 5).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            let results = ss.fuzzy_subsequence_search("hel");
            assert_eq!(results.len(), 3);

            // Results should be sorted by score (ascending), then frequency (descending), then age (descending)
            // "hello" and "help" should have better scores than "helicopter"
            assert!(results[0].string == "hello" || results[0].string == "help");
            assert!(results[1].string == "hello" || results[1].string == "help");
            assert_eq!(results[2].string, "helicopter");
        }

        #[test]
        fn test_fuzzy_subsequence_search_empty_query() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 1).unwrap();

            let results = ss.fuzzy_subsequence_search("");
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_fuzzy_subsequence_search_no_matches() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 1).unwrap();

            let results = ss.fuzzy_subsequence_search("xyz");
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_basic_subsequence_matching() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();

            let results = ss.fuzzy_subsequence_search("hl");
            assert_eq!(results.len(), 3);
            // All three "h" words match "hl" as subsequence
            // Results are sorted by score (ascending), then frequency (descending)
            // "help" has highest frequency (3), then "hello" (1), then "helicopter" (1)
            // "helicopter" has worst score due to longer span
            assert!(results[0].string == "help");
            assert!(results[1].string == "hello");
            assert!(results[2].string == "helicopter");
        }

        #[test]
        fn test_non_matching_sequences() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            let results = ss.fuzzy_subsequence_search("xyz");
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_exact_matches() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            let results = ss.fuzzy_subsequence_search("hello");
            assert_eq!(results.len(), 1);
            assert!(results[0].string == "hello");
        }

        #[test]
        fn test_utf8_character_handling() {
            let mut ss = StringSpace::new();
            ss.insert_string("café", 1).unwrap();
            ss.insert_string("naïve", 2).unwrap();
            ss.insert_string("über", 3).unwrap();

            let results = ss.fuzzy_subsequence_search("cf");
            assert_eq!(results.len(), 1);
            assert!(results[0].string == "café");

            let results = ss.fuzzy_subsequence_search("nv");
            assert_eq!(results.len(), 1);
            assert!(results[0].string == "naïve");
        }

        #[test]
        fn test_result_ranking_verification() {
            let mut ss = StringSpace::new();
            // Insert strings with different frequencies and ages
            ss.insert_string("hello", 1).unwrap();  // frequency 1
            ss.insert_string("help", 3).unwrap();   // frequency 3
            ss.insert_string("helicopter", 2).unwrap(); // frequency 2

            let results = ss.fuzzy_subsequence_search("hl");
            assert_eq!(results.len(), 3);
            // Results should be sorted by score (ascending), then frequency (descending), then age (descending)
            // "hello" and "help" should have similar scores, but "help" has higher frequency
            // "helicopter" should have worse score due to longer span
            assert!(results[0].string == "help");
            assert!(results[1].string == "hello");
            assert!(results[2].string == "helicopter");
        }

        #[test]
        fn test_abbreviation_matching() {
            let mut ss = StringSpace::new();
            ss.insert_string("openai/gpt-4o-2024-08-06", 1).unwrap();
            ss.insert_string("openai/gpt-5", 2).unwrap();
            ss.insert_string("anthropic/claude-3-opus", 3).unwrap();

            let results = ss.fuzzy_subsequence_search("og4");
            assert_eq!(results.len(), 1);
            assert!(results[0].string == "openai/gpt-4o-2024-08-06");

            let results = ss.fuzzy_subsequence_search("ogp5");
            assert_eq!(results.len(), 1);
            assert!(results[0].string == "openai/gpt-5");
        }

        #[test]
        fn test_public_api_fuzzy_subsequence_search() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();

            // Test public API method
            let results = ss.fuzzy_subsequence_search("hl");
            assert_eq!(results.len(), 3);
            assert!(results[0].string == "help");  // Higher frequency
            assert!(results[1].string == "hello"); // Lower frequency
            assert!(results[2].string == "helicopter"); // Worst score due to longer span
        }

        #[test]
        fn test_public_api_empty_query() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Test empty query handling through public API
            let results = ss.fuzzy_subsequence_search("");
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_public_api_no_matches() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Test no matches through public API
            let results = ss.fuzzy_subsequence_search("xyz");
            assert_eq!(results.len(), 0);
        }
    }
}