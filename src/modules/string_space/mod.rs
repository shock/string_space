//! StringSpace Module - Efficient String Storage and Search
//!
//! This module provides a high-performance string storage and search system with
//! multiple search algorithms including prefix matching, fuzzy subsequence search,
//! Jaro-Winkler similarity, and substring search.
//!
//! # Features
//! - **Custom Memory Management**: 4KB-aligned memory allocation for optimal performance
//! - **Multiple Search Algorithms**: Progressive algorithm execution with dynamic weighting
//! - **Unicode Support**: Full UTF-8 character handling
//! - **Metadata Integration**: Frequency and age-based ranking
//! - **Performance Optimizations**: Early termination and smart filtering
//!
//! # Algorithm Overview
//! The system uses a progressive execution strategy:
//! 1. **Prefix Search** (O(log n)) - Fast exact prefix matching
//! 2. **Fuzzy Subsequence** (O(n) with early exit) - Character order-preserving search
//! 3. **Jaro-Winkler** (O(n) with early exit) - Typo correction and similarity
//! 4. **Substring Search** (O(n)) - Fallback for longer queries
//!
//! # Performance Characteristics
//! - **Small datasets** (< 1,000 words): < 1ms per query
//! - **Medium datasets** (1,000-10,000 words): < 10ms per query
//! - **Large datasets** (> 10,000 words): < 100ms per query
//! - **Memory usage**: O(n) with 4KB alignment overhead

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter, BufRead};
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};
use std::cmp::Ordering;

mod tests;

const MIN_CHARS: usize = 3;
const MAX_CHARS: usize = 50;
const INITIAL_HEAP_SIZE: usize = 4096*256; // 4KB
const ALIGNMENT: usize = 4096; // 4KB alignment

/// High-performance string storage and search system.
///
/// Provides efficient string storage with multiple search algorithms including
/// prefix matching, fuzzy subsequence search, Jaro-Winkler similarity, and substring search.
///
/// # Examples
/// ```
/// use string_space::StringSpace;
///
/// let mut ss = StringSpace::new();
/// ss.insert_string("hello", 1).unwrap();
/// ss.insert_string("help", 2).unwrap();
///
/// // Find prefix matches
/// let results = ss.find_by_prefix("hel");
/// assert_eq!(results.len(), 2);
///
/// // Use best_completions for intelligent search
/// let completions = ss.best_completions("hl", Some(10));
/// assert!(completions.len() >= 2);
/// ```
///
/// # Performance
/// - Insertion: O(log n) for sorted insertion
/// - Prefix search: O(log n)
/// - Fuzzy search: O(n) with early termination
/// - Memory: O(n) with 4KB alignment overhead
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

/// Search algorithm types used in the best_completions system.
///
/// Each algorithm has different strengths and is weighted dynamically
/// based on query length and characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlgorithmType {
    /// Prefix matching - fast exact prefix search (O(log n))
    Prefix,
    /// Fuzzy subsequence - character order-preserving search (O(n))
    FuzzySubseq,
    /// Jaro-Winkler similarity - typo correction and similarity (O(n))
    JaroWinkler,
    /// Substring matching - fallback for longer queries (O(n))
    Substring,
}

/// Alternative score from other algorithms for the same string
#[derive(Debug, Clone)]
pub struct AlternativeScore {
    pub algorithm: AlgorithmType,
    pub normalized_score: f64,
}

// Query length categories for dynamic weighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryLengthCategory {
    VeryShort,  // 1-2 characters
    Short,      // 3-4 characters
    Medium,     // 5-6 characters
    Long,       // 7+ characters
}

impl QueryLengthCategory {
    fn from_query(query: &str) -> Self {
        match query.len() {
            1..=2 => QueryLengthCategory::VeryShort,
            3..=4 => QueryLengthCategory::Short,
            5..=6 => QueryLengthCategory::Medium,
            _ => QueryLengthCategory::Long,
        }
    }
}

/// Helper struct for storing algorithm scores
#[derive(Debug, Clone)]
pub struct AlgorithmScore {
    pub algorithm: AlgorithmType,
    pub raw_score: f64,
    pub normalized_score: f64,
}

impl AlgorithmScore {
    pub fn new(algorithm: AlgorithmType, raw_score: f64, normalized_score: f64) -> Self {
        Self {
            algorithm,
            raw_score,
            normalized_score,
        }
    }
}

// Dynamic weight tables for each query length category
struct AlgorithmWeights {
    prefix: f64,
    fuzzy_subseq: f64,
    jaro_winkler: f64,
    substring: f64,
}

impl AlgorithmWeights {
    fn for_category(category: QueryLengthCategory) -> Self {
        let fuzzy_multiplier = 1.;      // Adjust this to boost fuzzy_subseq
        let jaro_multiplier = 0.0;       // Adjust this to boost jaro_winkler

        match category {
            QueryLengthCategory::VeryShort => {
                let orig_prefix = 0.45;
                let orig_fuzzy = 0.35;
                let orig_jaro = 0.15;
                let orig_substring = 0.05;

                let fuzzy_subseq = orig_fuzzy * fuzzy_multiplier;
                let jaro_winkler = orig_jaro * jaro_multiplier;
                let remaining = 1.0 - (fuzzy_subseq + jaro_winkler);
                let sum_others = orig_prefix + orig_substring;
                let prefix = orig_prefix * (remaining / sum_others);
                let substring = orig_substring * (remaining / sum_others);

                AlgorithmWeights {
                    prefix,
                    fuzzy_subseq,
                    jaro_winkler,
                    substring,
                }
            }
            QueryLengthCategory::Short => {
                let orig_prefix = 0.40;
                let orig_fuzzy = 0.30;
                let orig_jaro = 0.20;
                let orig_substring = 0.10;

                let fuzzy_subseq = orig_fuzzy * fuzzy_multiplier;
                let jaro_winkler = orig_jaro * jaro_multiplier;
                let remaining = 1.0 - (fuzzy_subseq + jaro_winkler);
                let sum_others = orig_prefix + orig_substring;
                let prefix = orig_prefix * (remaining / sum_others);
                let substring = orig_substring * (remaining / sum_others);

                AlgorithmWeights {
                    prefix,
                    fuzzy_subseq,
                    jaro_winkler,
                    substring,
                }
            }
            QueryLengthCategory::Medium => {
                let orig_prefix = 0.35;
                let orig_fuzzy = 0.25;
                let orig_jaro = 0.25;
                let orig_substring = 0.15;

                let fuzzy_subseq = orig_fuzzy * fuzzy_multiplier;
                let jaro_winkler = orig_jaro * jaro_multiplier;
                let remaining = 1.0 - (fuzzy_subseq + jaro_winkler);
                let sum_others = orig_prefix + orig_substring;
                let prefix = orig_prefix * (remaining / sum_others);
                let substring = orig_substring * (remaining / sum_others);

                AlgorithmWeights {
                    prefix,
                    fuzzy_subseq,
                    jaro_winkler,
                    substring,
                }
            }
            QueryLengthCategory::Long => {
                let orig_prefix = 0.25;
                let orig_fuzzy = 0.20;
                let orig_jaro = 0.35;
                let orig_substring = 0.20;

                let fuzzy_subseq = orig_fuzzy * fuzzy_multiplier;
                let jaro_winkler = orig_jaro * jaro_multiplier;
                let remaining = 1.0 - (fuzzy_subseq + jaro_winkler);
                let sum_others = orig_prefix + orig_substring;
                let prefix = orig_prefix * (remaining / sum_others);
                let substring = orig_substring * (remaining / sum_others);

                AlgorithmWeights {
                    prefix,
                    fuzzy_subseq,
                    jaro_winkler,
                    substring,
                }
            }
        }
    }
}

/// Represents a candidate string with scoring information from multiple algorithms
#[derive(Debug, Clone)]
pub struct ScoreCandidate {
    pub string_ref: StringRef,
    pub algorithm: AlgorithmType,
    pub raw_score: f64,
    pub normalized_score: f64,
    pub final_score: f64,
    pub alternative_scores: Vec<AlternativeScore>,
}

impl ScoreCandidate {
    pub fn new(string_ref: StringRef, algorithm: AlgorithmType, raw_score: f64, normalized_score: f64) -> Self {
        Self {
            string_ref,
            algorithm,
            raw_score,
            normalized_score,
            final_score: 0.0,
            alternative_scores: Vec::new(),
        }
    }

    /// Add an alternative score from another algorithm
    pub fn add_alternative_score(&mut self, algorithm: AlgorithmType, normalized_score: f64) {
        self.alternative_scores.push(AlternativeScore {
            algorithm,
            normalized_score,
        });
    }

}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct StringRef {
    pub string: String,
    pub meta: StringMeta,
}

impl StringSpace {
    /// Creates a new empty StringSpace instance.
    ///
    /// # Examples
    /// ```
    /// use string_space::StringSpace;
    ///
    /// let ss = StringSpace::new();
    /// assert!(ss.empty());
    /// assert_eq!(ss.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            inner: StringSpaceInner::new(),
        }
    }

    /// Inserts a string into the storage with the given frequency.
    ///
    /// Strings must be between 3 and 50 characters in length. If the string
    /// already exists, its frequency is incremented and age is updated.
    ///
    /// # Arguments
    /// * `string` - The string to insert (3-50 characters)
    /// * `frequency` - Initial frequency count
    ///
    /// # Returns
    /// * `Ok(())` on successful insertion
    /// * `Err("String length out of bounds")` if length constraints are violated
    ///
    /// # Examples
    /// ```
    /// use string_space::StringSpace;
    ///
    /// let mut ss = StringSpace::new();
    /// ss.insert_string("hello", 1).unwrap();
    /// assert_eq!(ss.len(), 1);
    /// ```
    #[allow(unused)]
    pub fn insert_string(&mut self, string: &str, frequency: TFreq) -> Result<(), &'static str> {
        if string.len() < MIN_CHARS || string.len() > MAX_CHARS {
            return Err("String length out of bounds");
        }
        self.inner.insert_string(string, frequency, None)
    }

    /// Insert a string with specific frequency and age metadata
    ///
    /// This is primarily for testing purposes to reproduce specific scenarios
    /// with known metadata values.
    #[allow(unused)]
    pub fn insert_string_with_age(&mut self, string: &str, frequency: TFreq, age_days: TAgeDays) -> Result<(), &'static str> {
        if string.len() < MIN_CHARS || string.len() > MAX_CHARS {
            return Err("String length out of bounds");
        }
        self.inner.insert_string(string, frequency, Some(age_days))
    }

    /// Finds strings that start with the given prefix, sorted by frequency.
    ///
    /// Uses binary search for O(log n) performance. Results are sorted by
    /// frequency in descending order.
    ///
    /// # Arguments
    /// * `prefix` - The prefix to search for
    ///
    /// # Returns
    /// * `Vec<StringRef>` - Strings matching the prefix, sorted by frequency
    ///
    /// # Examples
    /// ```
    /// use string_space::StringSpace;
    ///
    /// let mut ss = StringSpace::new();
    /// ss.insert_string("hello", 5).unwrap();
    /// ss.insert_string("help", 3).unwrap();
    /// ss.insert_string("helicopter", 1).unwrap();
    ///
    /// let results = ss.find_by_prefix("hel");
    /// assert_eq!(results.len(), 3);
    /// assert_eq!(results[0].string, "hello"); // Highest frequency
    /// ```
    #[allow(unused)]
    pub fn find_by_prefix(&self, prefix: &str) -> Vec<StringRef> {
        self.inner.find_by_prefix(prefix)
    }

    /// Finds similar words using Jaro-Winkler similarity.
    ///
    /// Searches for words similar to the input using character-based similarity
    /// with a configurable cutoff threshold. Uses prefix filtering for performance.
    ///
    /// # Arguments
    /// * `word` - The word to find similar matches for
    /// * `cutoff` - Optional similarity threshold (0.0-1.0), defaults to 0.7
    ///
    /// # Returns
    /// * `Vec<StringRef>` - Similar words sorted by similarity, frequency, and age
    ///
    /// # Examples
    /// ```
    /// use string_space::StringSpace;
    ///
    /// let mut ss = StringSpace::new();
    /// ss.insert_string("hello", 1).unwrap();
    /// ss.insert_string("help", 2).unwrap();
    /// ss.insert_string("helicopter", 1).unwrap();
    ///
    /// let results = ss.get_similar_words("hell", Some(0.6));
    /// assert!(results.len() >= 2);
    /// ```
    #[allow(unused)]
    pub fn get_similar_words(&self, word: &str, cutoff: Option<f64>) -> Vec<StringRef> {
        let cutoff = cutoff.unwrap_or(0.7);
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

    /// Finds the best completions for a query using multiple search algorithms.
    ///
    /// This is the main intelligent search method that combines multiple algorithms
    /// using progressive execution and dynamic weighting based on query length.
    ///
    /// # Algorithm Strategy
    /// 1. **Prefix Search** (O(log n)) - Fast exact prefix matching
    /// 2. **Fuzzy Subsequence** (O(n) with early exit) - Character order-preserving search
    /// 3. **Jaro-Winkler** (O(n) with early exit) - Typo correction and similarity
    /// 4. **Substring Search** (O(n)) - Fallback for longer queries
    ///
    /// # Dynamic Weighting
    /// Algorithm weights are dynamically adjusted based on query length:
    /// - **Very Short (1-2 chars)**: Prefix (45%), Fuzzy (35%), Jaro (15%), Substring (5%)
    /// - **Short (3-4 chars)**: Prefix (40%), Fuzzy (30%), Jaro (20%), Substring (10%)
    /// - **Medium (5-6 chars)**: Balanced weights across all algorithms
    /// - **Long (7+ chars)**: Jaro (35%), Prefix (25%), Substring (20%), Fuzzy (20%)
    ///
    /// # Arguments
    /// * `query` - The search query (1-50 characters, alphanumeric for single char)
    /// * `limit` - Optional maximum number of results, defaults to 15
    ///
    /// # Returns
    /// * `Vec<StringRef>` - Best completions sorted by relevance score
    ///
    /// # Examples
    /// ```
    /// use string_space::StringSpace;
    ///
    /// let mut ss = StringSpace::new();
    /// ss.insert_string("hello", 10).unwrap();
    /// ss.insert_string("help", 15).unwrap();
    /// ss.insert_string("helicopter", 5).unwrap();
    /// ss.insert_string("world", 20).unwrap();
    ///
    /// // Prefix matching
    /// let results = ss.best_completions("hel", Some(10));
    /// assert!(results.len() >= 3);
    ///
    /// // Fuzzy subsequence (abbreviation matching)
    /// let results = ss.best_completions("hl", Some(10));
    /// assert!(results.len() >= 3);
    ///
    /// // Typo correction
    /// let results = ss.best_completions("wrold", Some(10));
    /// assert!(results.len() >= 1);
    /// ```
    ///
    /// # Performance
    /// - **Small datasets** (< 1,000 words): < 1ms
    /// - **Medium datasets** (1,000-10,000 words): < 10ms
    /// - **Large datasets** (> 10,000 words): < 100ms
    ///
    /// # Limitations
    /// - Query must be 1-50 characters
    /// - Single character queries must be alphanumeric
    /// - Control characters are rejected
    /// - Empty queries return empty results
    #[allow(unused)]
    pub fn best_completions(&self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
        self.inner.best_completions(query, limit)
    }

    /// Performs fuzzy subsequence search across the entire database.
    ///
    /// Searches for strings where the query appears as a subsequence (characters
    /// in order but not necessarily consecutive). Uses smart filtering and early
    /// termination for performance.
    ///
    /// # Arguments
    /// * `query` - The query to search for as a subsequence
    /// * `target_count` - Target number of results for early termination
    /// * `score_threshold` - Minimum normalized score (0.0-1.0)
    ///
    /// # Returns
    /// * `Vec<StringRef>` - Matching strings with scores above threshold
    ///
    /// # Examples
    /// ```
    /// use string_space::StringSpace;
    ///
    /// let mut ss = StringSpace::new();
    /// ss.insert_string("hello", 1).unwrap();
    /// ss.insert_string("help", 2).unwrap();
    /// ss.insert_string("helicopter", 1).unwrap();
    ///
    /// let results = ss.fuzzy_subsequence_full_database("hl", 10, 0.5);
    /// assert!(results.len() >= 3);
    /// ```
    #[allow(unused)]
    pub fn fuzzy_subsequence_full_database(
        &self,
        query: &str,
        target_count: usize,
        score_threshold: f64
    ) -> Vec<StringRef> {
        let results = self.inner.fuzzy_subsequence_full_database(query, target_count, score_threshold);
        results.into_iter().map(|result| result.string_ref).collect()
    }

    /// Performs Jaro-Winkler similarity search across the entire database.
    ///
    /// Searches for strings similar to the query using character-based similarity.
    /// Uses smart filtering and early termination for performance.
    ///
    /// # Arguments
    /// * `query` - The query to find similar matches for
    /// * `target_count` - Target number of results for early termination
    /// * `similarity_threshold` - Minimum similarity score (0.0-1.0)
    ///
    /// # Returns
    /// * `Vec<StringRef>` - Similar strings with scores above threshold
    ///
    /// # Examples
    /// ```
    /// use string_space::StringSpace;
    ///
    /// let mut ss = StringSpace::new();
    /// ss.insert_string("hello", 1).unwrap();
    /// ss.insert_string("help", 2).unwrap();
    /// ss.insert_string("helicopter", 1).unwrap();
    ///
    /// let results = ss.jaro_winkler_full_database("hell", 10, 0.7);
    /// assert!(results.len() >= 2);
    /// ```
    #[allow(unused)]
    pub fn jaro_winkler_full_database(
        &self,
        query: &str,
        target_count: usize,
        similarity_threshold: f64
    ) -> Vec<StringRef> {
        let results = self.inner.jaro_winkler_full_database(query, target_count, similarity_threshold);
        results.into_iter().map(|result| result.string_ref).collect()
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

    // Wrapper method for prefix search - returns unsorted results for best_completions system
    fn prefix_search(&self, query: &str) -> Vec<StringRef> {
        self.find_by_prefix_no_sort(query)
    }

    fn scored_prefix_search(&self, query: &str) -> Vec<ScoreCandidate> {
        let mut results = Vec::new();
        let matches = self.find_by_prefix(query);
        for string_ref in matches {
            let score = self.calculate_prefix_score(&string_ref, &query);
            if score.is_none() {
                continue;
            }
            let score = score.unwrap();
            results.push(ScoreCandidate::new(string_ref, AlgorithmType::Prefix, score.raw_score, score.normalized_score));
        }
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

    fn best_completions(&self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
        let limit = limit.unwrap_or(15);

        // Early return for empty database
        if self.empty() {
            return Vec::new();
        }

        // Validate query with detailed error handling
        if let Err(err) = validate_query(query) {
            // Log validation errors for debugging (in production, this would use proper logging)
            println!("Query validation failed: {}", err);
            return Vec::new();
        }

        // Handle very short queries with special care
        if query.len() == 1 {
            return self.handle_single_character_query(query, limit);
        }

        // Use progressive algorithm execution to get initial candidates
        let all_candidates = self.progressive_algorithm_execution(query, limit);

        // Otherwise, collect detailed scores from all algorithms
        println!("Collecting detailed scores for candidates...");
        let scored_candidates = all_candidates;

        // Handle case where no candidates were found
        if scored_candidates.is_empty() {
            return Vec::new();
        }

        // Merge duplicate candidates and calculate final scores
        let merged_candidates = merge_and_score_candidates(scored_candidates, query, self);

        // Sort by final score
        let mut ranked_candidates = merged_candidates;
        rank_candidates_by_score(&mut ranked_candidates);

        // bubble prefix matches to the top, primary sort case-insensitive, secondary sort case-sensitive
        ranked_candidates.sort_by(|a, b| {
            let lower_query = query.to_lowercase();
            let a_is_prefix = a.string_ref.string.to_lowercase().starts_with(&lower_query);
            let b_is_prefix = b.string_ref.string.to_lowercase().starts_with(&lower_query);
            if a_is_prefix && !b_is_prefix {
                Ordering::Less
            } else if !a_is_prefix && b_is_prefix {
                Ordering::Greater
            } else {
                let a_is_prefix = a.string_ref.string.starts_with(&query);
                let b_is_prefix = b.string_ref.string.starts_with(&query);
                if a_is_prefix && !b_is_prefix {
                    Ordering::Less
                } else if !a_is_prefix && b_is_prefix {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            }
        });

        // // Limit to top 10 results
        // ranked_candidates.truncate(limit);

        // get the limit number of ScoreCandidates from ranked_candidates
        let results: Vec<ScoreCandidate> = ranked_candidates.iter().take(limit).cloned().collect();
        print_debug_score_candidates(&results);
        // Apply limit and return
        limit_and_convert_results(ranked_candidates, limit)
    }

    /// Handle single character queries with special logic
    fn handle_single_character_query(&self, query: &str, limit: usize) -> Vec<StringRef> {
        // For single character queries, use prefix search only
        // This avoids expensive fuzzy matching for very short queries
        let results = self.prefix_search(query);

        // Sort by frequency (descending) to provide most relevant results
        let mut sorted_results = results;
        sorted_results.sort_by(|a, b| b.meta.frequency.cmp(&a.meta.frequency));

        sorted_results.into_iter().take(limit).collect()
    }

    /// Get maximum string length in the database
    fn get_max_string_length(&self) -> usize {
        self.get_all_strings()
            .iter()
            .map(|s| s.string.len())
            .max()
            .unwrap_or(0)
    }

    /// Collect detailed scores for candidates from all algorithms
    fn collect_detailed_scores(&self, query: &str, candidates: &[StringRef]) -> Vec<ScoreCandidate> {
        let mut scored_candidates = Vec::new();

        // First pass: collect raw fuzzy subsequence scores for proper normalization
        let mut fuzzy_raw_scores = Vec::new();
        let mut fuzzy_candidates = Vec::new();

        for string_ref in candidates {
            if let Some(raw_score) = self.calculate_fuzzy_subsequence_raw_score(string_ref, query) {
                fuzzy_raw_scores.push(raw_score);
                fuzzy_candidates.push(string_ref);
            }
        }

        // Calculate global min/max for fuzzy subsequence normalization
        let (fuzzy_min, fuzzy_max) = if !fuzzy_raw_scores.is_empty() {
            let min_score = fuzzy_raw_scores.iter().fold(f64::MAX, |a, &b| a.min(b));
            let max_score = fuzzy_raw_scores.iter().fold(f64::MIN, |a, &b| a.max(b));

            // Handle edge cases like full-database search does
            if (max_score - min_score).abs() < f64::EPSILON {
                if fuzzy_raw_scores.len() > 1 {
                    (0.0, 1.0)
                } else {
                    (min_score - 1.0, max_score + 1.0)
                }
            } else if (max_score - min_score) < 0.1 {
                let mid = (min_score + max_score) / 2.0;
                (mid - 0.5, mid + 0.5)
            } else {
                (min_score, max_score)
            }
        } else {
            (0.0, 1.0) // Default range if no fuzzy candidates
        };

        // Second pass: calculate all scores with proper normalization
        for string_ref in candidates {
            // Calculate scores from all algorithms for this candidate
            let prefix_score = self.calculate_prefix_score(string_ref, query);
            let fuzzy_score = self.calculate_fuzzy_subsequence_score_with_range(string_ref, query, fuzzy_min, fuzzy_max);
            let jaro_score = self.calculate_jaro_winkler_score(string_ref, query);
            let substring_score = self.calculate_substring_score(string_ref, query);

            // Create candidate with the best algorithm score
            let (best_algorithm, best_score) = self.select_best_algorithm_score(
                prefix_score.clone(), fuzzy_score.clone(), jaro_score.clone(), substring_score.clone()
            );

            let mut candidate = ScoreCandidate::new(
                string_ref.clone(),
                best_algorithm,
                best_score.raw_score,
                best_score.normalized_score
            );

            // Add alternative scores from other algorithms
            if let Some(score) = prefix_score {
                if score.algorithm != best_algorithm {
                    candidate.add_alternative_score(score.algorithm, score.normalized_score);
                }
            }
            if let Some(score) = fuzzy_score {
                if score.algorithm != best_algorithm {
                    candidate.add_alternative_score(score.algorithm, score.normalized_score);
                }
            }
            if let Some(score) = jaro_score {
                if score.algorithm != best_algorithm {
                    candidate.add_alternative_score(score.algorithm, score.normalized_score);
                }
            }
            if let Some(score) = substring_score {
                if score.algorithm != best_algorithm {
                    candidate.add_alternative_score(score.algorithm, score.normalized_score);
                }
            }

            scored_candidates.push(candidate);
        }

        scored_candidates
    }

    /// Calculate prefix match score with case-sensitive support
    fn calculate_prefix_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
        let candidate = string_ref.string.as_str();

        // Case-sensitive prefix matching
        if candidate.starts_with(query) {
            // Perfect prefix match gets maximum score
            Some(AlgorithmScore::new(
                AlgorithmType::Prefix,
                1.0,  // raw score
                1.0   // normalized score
            ))
        } else {
            // Case-insensitive prefix matching
            if candidate.to_lowercase().starts_with(&query.to_lowercase()) {
                // Slightly lower score for case-insensitive match
                Some(AlgorithmScore::new(
                    AlgorithmType::Prefix,
                    0.9999,  // raw score
                    0.9999   // normalized score
                ))
            } else {
                None
            }

        }
    }


    /// Calculate fuzzy subsequence raw score without normalization
    fn calculate_fuzzy_subsequence_raw_score(&self, string_ref: &StringRef, query: &str) -> Option<f64> {
        let candidate = string_ref.string.as_str();

        // Apply smart filtering to skip unpromising candidates
        if should_skip_candidate_fuzzy(candidate.len(), query.len()) {
            return None;
        }

        // Check if query is a subsequence of candidate
        let query_chars: Vec<char> = query.chars().collect();
        let candidate_chars: Vec<char> = candidate.chars().collect();

        if is_subsequence_chars(&query_chars, &candidate_chars).is_none() {
            return None;
        }

        // Calculate match span score (lower is better)
        let raw_score = score_match_span_chars(&query_chars, &candidate_chars);
        Some(raw_score)
    }

    /// Calculate fuzzy subsequence score with provided normalization range
    fn calculate_fuzzy_subsequence_score_with_range(
        &self,
        string_ref: &StringRef,
        query: &str,
        min_score: f64,
        max_score: f64
    ) -> Option<AlgorithmScore> {
        let candidate = string_ref.string.as_str();

        // Apply smart filtering to skip unpromising candidates
        if should_skip_candidate_fuzzy(candidate.len(), query.len()) {
            return None;
        }

        // Check if query is a subsequence of candidate
        let query_chars: Vec<char> = query.chars().collect();
        let candidate_chars: Vec<char> = candidate.chars().collect();

        if is_subsequence_chars(&query_chars, &candidate_chars).is_none() {
            return None;
        }

        // Calculate match span score (lower is better)
        let raw_score = score_match_span_chars(&query_chars, &candidate_chars);

        // Use provided normalization range
        let normalized_score = normalize_fuzzy_score(raw_score, min_score, max_score);

        Some(AlgorithmScore::new(
            AlgorithmType::FuzzySubseq,
            raw_score,
            normalized_score
        ))
    }

    /// Calculate Jaro-Winkler similarity score with threshold
    fn calculate_jaro_winkler_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
        let candidate = string_ref.string.as_str();

        // Apply smart filtering to skip unpromising candidates
        if should_skip_candidate(candidate.len(), query.len()) {
            return None;
        }

        // Calculate Jaro-Winkler similarity (already normalized 0.0-1.0)
        let similarity = jaro_winkler(query, candidate);

        // Optimized: Apply more restrictive threshold for better performance
        // Only include high-quality matches
        if similarity < 0.7 {
            return None;
        }

        Some(AlgorithmScore::new(
            AlgorithmType::JaroWinkler,
            similarity,  // raw score
            similarity   // normalized score (same for Jaro-Winkler)
        ))
    }

    /// Calculate substring match score with position normalization
    fn calculate_substring_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
        let candidate = string_ref.string.as_str();

        // Find substring position
        if let Some(position) = candidate.find(query) {
            // Calculate position-based score (earlier matches are better)
            let max_position = candidate.len().saturating_sub(query.len());
            let normalized_score = normalize_substring_score(position, max_position);

            Some(AlgorithmScore::new(
                AlgorithmType::Substring,
                position as f64,  // raw score (position)
                normalized_score  // normalized score
            ))
        } else {
            None
        }
    }

    /// Select the best algorithm score for a candidate
    fn select_best_algorithm_score(
        &self,
        prefix_score: Option<AlgorithmScore>,
        fuzzy_score: Option<AlgorithmScore>,
        jaro_score: Option<AlgorithmScore>,
        substring_score: Option<AlgorithmScore>
    ) -> (AlgorithmType, AlgorithmScore) {
        let mut best_score = None;
        let mut best_algorithm = AlgorithmType::Prefix; // Default fallback

        // Compare all available scores and select the best one
        if let Some(score) = prefix_score {
            if best_score.map_or(true, |best: f64| score.normalized_score > best) {
                best_score = Some(score.normalized_score);
                best_algorithm = score.algorithm;
            }
        }

        if let Some(score) = fuzzy_score {
            if best_score.map_or(true, |best: f64| score.normalized_score > best) {
                best_score = Some(score.normalized_score);
                best_algorithm = score.algorithm;
            }
        }

        if let Some(score) = jaro_score {
            if best_score.map_or(true, |best: f64| score.normalized_score > best) {
                best_score = Some(score.normalized_score);
                best_algorithm = score.algorithm;
            }
        }

        if let Some(score) = substring_score {
            if best_score.map_or(true, |best: f64| score.normalized_score > best) {
                best_score = Some(score.normalized_score);
                best_algorithm = score.algorithm;
            }
        }

        // Create the best score object
        let best_score_value = best_score.unwrap_or(0.0);
        let best_score_obj = AlgorithmScore::new(
            best_algorithm,
            best_score_value,  // raw score
            best_score_value   // normalized score
        );

        (best_algorithm, best_score_obj)
    }

    // Full-database fuzzy subsequence search with early termination
    fn fuzzy_subsequence_full_database(
        &self,
        query: &str,
        target_count: usize,
        score_threshold: f64
    ) -> Vec<ScoreCandidate> {
        // Empty query handling: return empty vector for empty queries
        if query.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let all_strings = self.get_all_strings();

        // Track min/max scores for normalization
        let mut min_score = f64::MAX;
        let mut max_score = f64::MIN;
        let mut scores = Vec::new();

        // First pass: collect scores for normalization
        for string_ref in &all_strings {
            if let Some(score) = self.score_fuzzy_subsequence(string_ref, query) {
                min_score = min_score.min(score);
                max_score = max_score.max(score);
                scores.push((string_ref.clone(), score));
            }
        }

        // Handle edge case where all scores are the same or very close
        if (max_score - min_score).abs() < f64::EPSILON {
            // If all scores are identical, treat them all as perfect matches
            // But only if we have multiple candidates - if only one candidate, use its score as reference
            if scores.len() > 1 {
                min_score = 0.0;
                max_score = 1.0;
            } else {
                // Single candidate: use a reasonable range around the score
                min_score = min_score - 1.0;
                max_score = max_score + 1.0;
            }
        } else if (max_score - min_score) < 0.1 {
            // If scores are very close together, expand the range to provide better differentiation
            let mid = (min_score + max_score) / 2.0;
            min_score = mid - 0.5;
            max_score = mid + 0.5;
        }

        // Second pass: apply normalization and threshold filtering
        for (string_ref, raw_score) in scores {
            let normalized_score = normalize_fuzzy_score(raw_score, min_score, max_score);
            // let string_copy = string_ref.string.clone();
            // println!("Normalized score for {}: {} (raw: {}, min: {}, max: {})", string_copy, normalized_score, raw_score, min_score, max_score);

            // For fuzzy subsequence: lower normalized scores are better (lower raw scores are better)
            // So we want to keep candidates with normalized scores ABOVE the threshold
            // (since better matches have higher normalized scores)
            if normalized_score >= score_threshold {
                let score_candidate = ScoreCandidate {
                    string_ref: string_ref.clone(),
                    algorithm: AlgorithmType::FuzzySubseq,
                    raw_score,
                    normalized_score,
                    alternative_scores: Vec::new(),
                    final_score: 0.0,
                };

                results.push(score_candidate);
                // println!("Added {} to results", string_copy);

                // Early termination: stop if we have enough high-quality candidates
                if results.len() >= target_count * 2 {
                    break;
                }
            }
        }

        results
    }

    // Full-database Jaro-Winkler similarity search with early termination
    fn jaro_winkler_full_database(
        &self,
        query: &str,
        target_count: usize,
        similarity_threshold: f64
    ) -> Vec<ScoreCandidate> {
        let mut results: Vec<ScoreCandidate> = Vec::new();
        let all_strings = self.get_all_strings();

        for string_ref in all_strings {
            let candidate = string_ref.string.as_str();

            // Apply smart filtering to skip unpromising candidates
            if should_skip_candidate(candidate.len(), query.len()) {
                continue;
            }

            // Calculate Jaro-Winkler similarity (already normalized 0.0-1.0)
            let similarity = jaro_winkler(query, candidate);

            if similarity >= similarity_threshold {
                let score_candidate = ScoreCandidate {
                    string_ref: string_ref.clone(),
                    algorithm: AlgorithmType::JaroWinkler,
                    raw_score: similarity,
                    normalized_score: similarity,
                    alternative_scores: Vec::new(),
                    final_score: 0.0,
                };
                results.push(score_candidate);

                // Early termination: stop if we have enough high-quality candidates
                if results.len() >= target_count * 2 {
                    break;
                }
            }
        }

        // sort results by similarity descending
        results.sort_by(|a, b| b.normalized_score.partial_cmp(&a.normalized_score).unwrap());
        results
    }

    // Helper function for fuzzy subsequence scoring
    fn score_fuzzy_subsequence(&self, string_ref: &StringRef, query: &str) -> Option<f64> {
        let candidate = string_ref.string.as_str();

        // Apply smart filtering to skip unpromising candidates
        // Use fuzzy-specific filtering for abbreviation matching
        if should_skip_candidate_fuzzy(candidate.len(), query.len()) {
            // println!("Skipping candidate {} due to length filtering", candidate);
            return None;
        }

        // Note: We don't use character filtering for fuzzy subsequence search
        // because it's designed to handle missing characters in the candidate
        // The subsequence matching itself will determine if the query can be found

        // Use existing fuzzy subsequence logic from the codebase
        // This adapts the existing fuzzy_subsequence_search but searches entire database
        let query_chars: Vec<char> = query.chars().collect();
        let candidate_chars: Vec<char> = candidate.chars().collect();

        if is_subsequence_chars(&query_chars, &candidate_chars).is_none() {
            // println!("Skipping candidate {} due to subsequence mismatch", candidate);
            return None;
        }

        // Calculate match span score
        let score = score_match_span_chars(&query_chars, &candidate_chars);
        // println!("Candidate {} scored: {}", candidate, score);
        Some(score)
    }

    // Progressive algorithm execution with early termination and error recovery
    fn progressive_algorithm_execution(
        &self,
        query: &str,
        limit: usize
    ) -> Vec<ScoreCandidate> {
        let mut all_candidates = Vec::new();
        let mut seen_strings = std::collections::HashSet::new();

        // Helper function to add unique candidates
        fn add_unique_candidates(
            candidates: Vec<ScoreCandidate>,
            all_candidates: &mut Vec<ScoreCandidate>,
            seen_strings: &mut std::collections::HashSet<String>
        ) {
            for candidate in candidates {
                if seen_strings.insert(candidate.string_ref.string.clone()) {
                    all_candidates.push(candidate);
                }
            }
        }

        // 1. Fast prefix search first (O(log n))
        // Only get 'limit' candidates
        let prefix_candidates = self.scored_prefix_search(query).into_iter()
            .take(limit)
            .collect::<Vec<_>>();

        add_unique_candidates(prefix_candidates, &mut all_candidates, &mut seen_strings);

        // 2. Fuzzy subsequence with early termination (O(n) with early exit)
        // Use fallback threshold for fuzzy search to ensure we get some results
        let fuzzy_candidates = self.fuzzy_subsequence_full_database(
            query,
            limit,
            0.0 // score threshold - include all matches for progressive execution
        );
        add_unique_candidates(fuzzy_candidates, &mut all_candidates, &mut seen_strings);

        // 3. Jaro-Winkler only if still needed (O(n) with early exit)
        // Use adaptive threshold for Jaro-Winkler based on query length
        let jaro_threshold = if query.len() <= 2 { 0.6 } else { 0.7 };
        let jaro_candidates = self.jaro_winkler_full_database(
            query,
            limit,
            jaro_threshold // adaptive similarity threshold
        );
        add_unique_candidates(jaro_candidates, &mut all_candidates, &mut seen_strings);


        all_candidates
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

fn print_debug_score_candidates(candidates: &[ScoreCandidate]) {
    println!("Debugging Score Candidates:");
    for candidate in candidates {
        println!(
            "String: {}, Final Score: {:.6}, Algorithm: {:?}, Raw Score: {:.6}, Normalized Score: {:.6}, Alt Scores: {:?}",
            candidate.string_ref.string,
            candidate.final_score,
            candidate.algorithm,
            candidate.raw_score,
            candidate.normalized_score,
            candidate.alternative_scores.iter()
                .map(|alt| format!("{:?}: {:.6}", alt.algorithm, alt.normalized_score))
                .collect::<Vec<_>>()
        );
    }
}

fn validate_query(query: &str) -> Result<(), &'static str> {
    if query.is_empty() {
        return Err("Query cannot be empty");
    }

    // Check for control characters and other problematic inputs
    if query.chars().any(|c| c.is_control()) {
        return Err("Query contains control characters");
    }

    // Check for minimum length requirements based on algorithm types
    // For very short queries, we need to ensure they're meaningful
    if query.len() == 1 {
        // Single character queries are allowed but limited to alphanumeric
        if !query.chars().all(|c| c.is_alphanumeric()) {
            return Err("Single character queries must be alphanumeric");
        }
    }

    // Check for maximum length (same as string storage limit)
    if query.len() > MAX_CHARS {
        return Err("Query too long");
    }

    // Check for Unicode normalization issues
    // Ensure the query is valid UTF-8 and doesn't contain problematic sequences
    if query.chars().any(|c| c == '\u{FFFD}') {
        return Err("Query contains invalid Unicode replacement characters");
    }

    Ok(())
}

// Metadata integration functions

/// Apply metadata adjustments to weighted score
fn apply_metadata_adjustments(
    weighted_score: f64,
    frequency: TFreq,
    age_days: TAgeDays,
    candidate_len: usize,
    query_len: usize,
    max_len: usize
) -> f64 {
    // 1. Frequency factor with logarithmic scaling to prevent dominance
    let frequency_factor = 1.0 + ((frequency as f64 + 1.0).ln() * 0.1);

    // 2. Age factor with bounded influence - newer items (bigger age) get slight preference
    // Set the max_age to the current time since epoch divided seconds in a day
    let max_age = days_since_epoch();
    let age_factor = 1.0 + (age_days as f64 / max_age as f64) * 0.05;

    // 3. Length penalty applied only for significant length mismatches
    let length_penalty = if candidate_len > query_len * 3 {
        // Only penalize when candidate is 3x longer than query
        1.0 - ((candidate_len - query_len) as f64 / max_len as f64) * 0.1
    } else {
        1.0 // No penalty for reasonable length differences
    };

    // 4. Apply multiplicative combination with bounds checking
    let final_score = weighted_score * frequency_factor * age_factor * length_penalty;

    // Ensure score doesn't exceed reasonable bounds
    final_score.clamp(0.0, 2.0) // Cap at 2.0 to prevent extreme values
}

/// Get dynamic weights based on query length
fn get_dynamic_weights(query: &str) -> AlgorithmWeights {
    let category = QueryLengthCategory::from_query(query);
    AlgorithmWeights::for_category(category)
}

/// Calculate weighted score combining all algorithm contributions
fn calculate_weighted_score(
    prefix_score: f64,
    fuzzy_score: f64,
    jaro_score: f64,
    substring_score: f64,
    query: &str
) -> f64 {
    let weights = get_dynamic_weights(query);

    weights.prefix * prefix_score +
    weights.fuzzy_subseq * fuzzy_score +
    weights.jaro_winkler * jaro_score +
    weights.substring * substring_score
}

/// Calculate final score for a candidate with metadata adjustments
fn calculate_final_score(
    candidate: &mut ScoreCandidate,
    query: &str,
    string_space: &StringSpaceInner
) -> f64 {
    // Get all algorithm scores for this candidate
    let mut prefix_score = 0.0;
    let mut fuzzy_score = 0.0;
    let mut jaro_score = 0.0;
    let mut substring_score = 0.0;

    // Extract scores from primary and alternative algorithms
    match candidate.algorithm {
        AlgorithmType::Prefix => prefix_score = candidate.normalized_score,
        AlgorithmType::FuzzySubseq => fuzzy_score = candidate.normalized_score,
        AlgorithmType::JaroWinkler => jaro_score = candidate.normalized_score,
        AlgorithmType::Substring => substring_score = candidate.normalized_score,
    }

    // Add alternative scores
    for alt in &candidate.alternative_scores {
        match alt.algorithm {
            AlgorithmType::Prefix => prefix_score = prefix_score.max(alt.normalized_score),
            AlgorithmType::FuzzySubseq => fuzzy_score = fuzzy_score.max(alt.normalized_score),
            AlgorithmType::JaroWinkler => jaro_score = jaro_score.max(alt.normalized_score),
            AlgorithmType::Substring => substring_score = substring_score.max(alt.normalized_score),
        }
    }

    // Calculate weighted algorithm score
    let weighted_score = calculate_weighted_score(
        prefix_score, fuzzy_score, jaro_score, substring_score, query
    );

    // Apply metadata adjustments
    let (frequency, age_days, candidate_len) = get_string_metadata(&candidate.string_ref);
    let query_len = query.len();
    let max_len = string_space.get_max_string_length();

    let final_score = apply_metadata_adjustments(
        weighted_score,
        frequency,
        age_days,
        candidate_len,
        query_len,
        max_len
    );

    candidate.final_score = final_score;
    final_score
}

/// Merge candidates from different algorithms and calculate final scores
fn merge_and_score_candidates(
    candidates: Vec<ScoreCandidate>,
    query: &str,
    string_space: &StringSpaceInner
) -> Vec<ScoreCandidate> {
    let mut merged: HashMap<String, ScoreCandidate> = HashMap::new();

    // Merge candidates by string reference
    for candidate in candidates {
        let string_key = candidate.string_ref.string.clone();
        if let Some(existing) = merged.get_mut(&string_key) {
            // Add as alternative score if this algorithm provides a better score
            if candidate.normalized_score > existing.normalized_score {
                existing.add_alternative_score(existing.algorithm, existing.normalized_score);
                existing.algorithm = candidate.algorithm;
                existing.raw_score = candidate.raw_score;
                existing.normalized_score = candidate.normalized_score;
            } else {
                existing.add_alternative_score(candidate.algorithm, candidate.normalized_score);
            }
        } else {
            merged.insert(string_key, candidate);
        }
    }

    // Calculate final scores for all merged candidates
    let mut scored_candidates: Vec<ScoreCandidate> = merged.into_values().collect();
    for candidate in &mut scored_candidates {
        calculate_final_score(candidate, query, string_space);
    }

    scored_candidates
}

/// Sort candidates by final score in descending order
fn rank_candidates_by_score(candidates: &mut [ScoreCandidate]) {
    candidates.sort_by(|a, b| {
        b.final_score.partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Apply result limiting and convert to StringRef output
fn limit_and_convert_results(candidates: Vec<ScoreCandidate>, limit: usize) -> Vec<StringRef> {
    candidates
        .into_iter()
        .take(limit)
        .map(|candidate| candidate.string_ref)
        .collect()
}


// Score normalization functions

/// For substring search (earlier matches are better)
fn normalize_substring_score(position: usize, max_position: usize) -> f64 {
    1.0 - (position as f64 / max_position as f64)
}

/// Get metadata for a string reference
fn get_string_metadata(string_ref: &StringRef) -> (TFreq, TAgeDays, usize) {
    // Use the metadata directly from StringRef
    (string_ref.meta.frequency, string_ref.meta.age_days, string_ref.string.len())
}

fn days_since_epoch() -> TAgeDays {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() / (60 * 60 * 24)).try_into().unwrap()
}

use libc::CLOCK_UPTIME_RAW;
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
            println!("word: {}, possibility: {}, score: {}", word, possibility, score);
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

// Character-based version for use with char vectors
fn is_subsequence_chars(query_chars: &[char], candidate_chars: &[char]) -> Option<Vec<usize>> {
    let mut query_iter = query_chars.iter();
    let mut current_char = query_iter.next();
    let mut match_indices = Vec::new();

    for (i, c) in candidate_chars.iter().enumerate() {
        if current_char == Some(c) {
            match_indices.push(i);
            current_char = query_iter.next();
            if current_char.is_none() {
                return Some(match_indices);
            }
        }
    }

    None
}

// Character-based version for scoring
fn score_match_span_chars(query_chars: &[char], candidate_chars: &[char]) -> f64 {
    if let Some(match_indices) = is_subsequence_chars(query_chars, candidate_chars) {
        if match_indices.is_empty() {
            return 0.0;
        }

        // get the average distance between matched characters
        let mut cumulative_distance = 0usize;
        for i in 1..match_indices.len() {
            cumulative_distance += match_indices[i] - match_indices[i-1];
        }
        let mut avg_distance = cumulative_distance as f64 / (match_indices.len() - 1) as f64; //avg_distance /= (match_indices.len() - 1) as f64;

        // normalize the average distance based on the length of the candidate string
        avg_distance /= candidate_chars.len() as f64;
        avg_distance


        // let span_length = (match_indices.last().unwrap() - match_indices.first().unwrap() + 1) as f64;
        // let candidate_length = candidate_chars.len() as f64;
        // candidate_length - span_length
    } else {
        0.0
    }
}

// Smart filtering to skip unpromising candidates
fn should_skip_candidate(candidate_len: usize, query_len: usize) -> bool {
    // Skip strings that are too short to contain the query
    if candidate_len < query_len {
        return true;
    }

    // Optimized: More aggressive filtering for better performance
    // Skip strings that are excessively long for short queries
    // For very short queries (1-2 chars), allow longer candidates but be more restrictive
    // For longer queries, be even more restrictive to reduce computation
    if query_len <= 2 && candidate_len > query_len * 8 {
        return true;
    } else if query_len <= 3 && candidate_len > query_len * 5 {
        return true;
    } else if query_len > 3 && candidate_len > query_len * 4 {
        return true;
    }

    false
}

// More lenient filtering for fuzzy subsequence search to allow abbreviation matching
fn should_skip_candidate_fuzzy(candidate_len: usize, query_len: usize) -> bool {
    // Skip strings that are too short to contain the query
    if candidate_len < query_len {
        return true;
    }

    // Optimized: Slightly more restrictive filtering for better performance
    // while still allowing abbreviation matching
    if query_len <= 2 && candidate_len > query_len * 15 {
        return true;
    } else if query_len <= 3 && candidate_len > query_len * 12 {
        return true;
    } else if query_len > 3 && candidate_len > query_len * 8 {
        return true;
    }

    false
}


// For fuzzy subsequence (lower raw scores are better)
fn normalize_fuzzy_score(raw_score: f64, min_score: f64, max_score: f64) -> f64 {
    // Optimized: Avoid division by zero and use efficient calculation
    let range = max_score - min_score;
    if range <= f64::EPSILON {
        // All scores are the same, return middle value
        return 0.5;
    }

    // Invert and normalize: lower raw scores → higher normalized scores
    let normalized = 1.0 - ((raw_score - min_score) / range);
    normalized.clamp(0.0, 1.0)
}
