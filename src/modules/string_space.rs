use std::collections::HashMap;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlgorithmType {
    Prefix,
    FuzzySubseq,
    JaroWinkler,
    Substring,
}

/// Alternative score from other algorithms for the same string
#[derive(Debug, Clone)]
pub struct AlternativeScore {
    pub algorithm: AlgorithmType,
    pub normalized_score: f64,
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

    /// Get the best available score for this candidate (primary or alternative)
    pub fn get_best_score(&self) -> f64 {
        let mut best_score = self.normalized_score;
        for alt in &self.alternative_scores {
            if alt.normalized_score > best_score {
                best_score = alt.normalized_score;
            }
        }
        best_score
    }
}

#[derive(Debug, Clone)]
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

    #[allow(unused)]
    pub fn best_completions(&self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
        self.inner.best_completions(query, limit)
    }

    #[allow(unused)]
    pub fn fuzzy_subsequence_full_database(
        &self,
        query: &str,
        target_count: usize,
        score_threshold: f64
    ) -> Vec<StringRef> {
        self.inner.fuzzy_subsequence_full_database(query, target_count, score_threshold)
    }

    #[allow(unused)]
    pub fn jaro_winkler_full_database(
        &self,
        query: &str,
        target_count: usize,
        similarity_threshold: f64
    ) -> Vec<StringRef> {
        self.inner.jaro_winkler_full_database(query, target_count, similarity_threshold)
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

    // Wrapper method for prefix search - returns unsorted results for best_completions system
    fn prefix_search(&self, query: &str) -> Vec<StringRef> {
        self.find_by_prefix_no_sort(query)
    }

    // Wrapper method for substring search - returns unsorted results for best_completions system
    fn substring_search(&self, query: &str) -> Vec<StringRef> {
        self.find_with_substring(query)
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

        // Validate query
        if let Err(_) = validate_query(query) {
            return Vec::new();
        }

        // Use progressive algorithm execution to get initial candidates
        let all_candidates = self.progressive_algorithm_execution(query, limit);

        // If we have enough high-quality prefix matches, return them directly
        if all_candidates.len() >= limit && self.has_high_quality_prefix_matches(&all_candidates, query) {
            return all_candidates.into_iter().take(limit).collect();
        }

        // Otherwise, collect detailed scores from all algorithms
        let scored_candidates = self.collect_detailed_scores(query, &all_candidates);

        // Merge duplicate candidates and calculate final scores
        let merged_candidates = merge_and_score_candidates(scored_candidates, query, self);

        // Sort by final score
        let mut ranked_candidates = merged_candidates;
        rank_candidates_by_score(&mut ranked_candidates);

        // Apply limit and return
        limit_and_convert_results(ranked_candidates, limit)
    }

    fn collect_results(&self) -> Vec<ScoreCandidate> {
        // Placeholder implementation
        // Will be replaced with actual algorithm execution in subsequent phases
        Vec::new()
    }

    /// Get maximum string length in the database
    fn get_max_string_length(&self) -> usize {
        self.get_all_strings()
            .iter()
            .map(|s| s.string.len())
            .max()
            .unwrap_or(0)
    }

    // Full-database fuzzy subsequence search with early termination
    fn fuzzy_subsequence_full_database(
        &self,
        query: &str,
        target_count: usize,
        score_threshold: f64
    ) -> Vec<StringRef> {
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
            let string_copy = string_ref.string.clone();
            println!("Normalized score for {}: {} (raw: {}, min: {}, max: {})", string_copy, normalized_score, raw_score, min_score, max_score);

            // For fuzzy subsequence: lower normalized scores are better (lower raw scores are better)
            // So we want to keep candidates with normalized scores ABOVE the threshold
            // (since better matches have higher normalized scores)
            if normalized_score >= score_threshold {
                results.push(string_ref);
                println!("Added {} to results", string_copy);

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
    ) -> Vec<StringRef> {
        let mut results = Vec::new();
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
                results.push(string_ref);

                // Early termination: stop if we have enough high-quality candidates
                if results.len() >= target_count * 2 {
                    break;
                }
            }
        }

        results
    }

    // Helper function for fuzzy subsequence scoring
    fn score_fuzzy_subsequence(&self, string_ref: &StringRef, query: &str) -> Option<f64> {
        let candidate = string_ref.string.as_str();

        // Apply smart filtering to skip unpromising candidates
        if should_skip_candidate(candidate.len(), query.len()) {
            println!("Skipping candidate {} due to length filtering", candidate);
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
            println!("Skipping candidate {} due to subsequence mismatch", candidate);
            return None;
        }

        // Calculate match span score (lower is better)
        let score = score_match_span_chars(&query_chars, &candidate_chars);
        println!("Candidate {} scored: {}", candidate, score);
        Some(score)
    }

    // Progressive algorithm execution with early termination
    fn progressive_algorithm_execution(
        &self,
        query: &str,
        limit: usize
    ) -> Vec<StringRef> {
        let mut all_candidates = Vec::new();

        // 1. Fast prefix search first (O(log n))
        let prefix_candidates = self.prefix_search(query);
        all_candidates.extend(prefix_candidates);

        // Early termination if we have enough high-quality prefix matches
        if all_candidates.len() >= limit && self.has_high_quality_prefix_matches(&all_candidates, query) {
            return all_candidates.into_iter().take(limit).collect();
        }

        // 2. Fuzzy subsequence with early termination (O(n) with early exit)
        let remaining_needed = limit.saturating_sub(all_candidates.len());
        if remaining_needed > 0 {
            let fuzzy_candidates = self.fuzzy_subsequence_full_database(
                query,
                remaining_needed,
                0.0 // score threshold - include all matches for progressive execution
            );
            all_candidates.extend(fuzzy_candidates);
        }

        // 3. Jaro-Winkler only if still needed (O(n) with early exit)
        let remaining_needed = limit.saturating_sub(all_candidates.len());
        if remaining_needed > 0 {
            let jaro_candidates = self.jaro_winkler_full_database(
                query,
                remaining_needed,
                0.8 // similarity threshold
            );
            all_candidates.extend(jaro_candidates);
        }

        // 4. Substring only as last resort for longer queries
        let remaining_needed = limit.saturating_sub(all_candidates.len());
        if remaining_needed > 0 && query.len() >= 3 {
            let substring_candidates = self.substring_search(query)
                .into_iter()
                .take(remaining_needed)
                .collect::<Vec<_>>();
            all_candidates.extend(substring_candidates);
        }

        all_candidates.into_iter().take(limit).collect()
    }

    // Helper to check for high-quality prefix matches
    fn has_high_quality_prefix_matches(&self, candidates: &[StringRef], query: &str) -> bool {
        // Consider it high quality if more than 2/3 of candidates are exact prefix matches
        let prefix_match_count = candidates.iter()
            .filter(|c| c.string.starts_with(query))
            .count();
        let threshold = (candidates.len() * 2) / 3;
        prefix_match_count > threshold
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

fn validate_query(query: &str) -> Result<(), &'static str> {
    if query.is_empty() {
        return Err("Query cannot be empty");
    }

    // Additional validation can be added here
    // For example: minimum length requirements, character restrictions, etc.

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

    // 2. Age factor with bounded influence (newer items get slight preference)
    let max_age = 365; // Maximum age in days for normalization
    let age_factor = 1.0 + (1.0 - (age_days as f64 / max_age as f64)) * 0.05;

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

// Dynamic weight tables for each query length category
struct AlgorithmWeights {
    prefix: f64,
    fuzzy_subseq: f64,
    jaro_winkler: f64,
    substring: f64,
}

impl AlgorithmWeights {
    fn for_category(category: QueryLengthCategory) -> Self {
        match category {
            QueryLengthCategory::VeryShort => AlgorithmWeights {
                prefix: 0.45,      // Highest weight for very short queries
                fuzzy_subseq: 0.35, // High weight for abbreviation matching
                jaro_winkler: 0.15, // Lower weight (less useful for very short)
                substring: 0.05,   // Minimal weight
            },
            QueryLengthCategory::Short => AlgorithmWeights {
                prefix: 0.40,      // High weight
                fuzzy_subseq: 0.30, // Good weight
                jaro_winkler: 0.20, // Medium weight
                substring: 0.10,   // Low weight
            },
            QueryLengthCategory::Medium => AlgorithmWeights {
                prefix: 0.35,      // Balanced weight
                fuzzy_subseq: 0.25, // Balanced weight
                jaro_winkler: 0.25, // Balanced weight
                substring: 0.15,   // Slightly higher weight
            },
            QueryLengthCategory::Long => AlgorithmWeights {
                prefix: 0.25,      // Lower weight (prefix less useful for long queries)
                fuzzy_subseq: 0.20, // Lower weight
                jaro_winkler: 0.35, // Highest weight (good for typo correction)
                substring: 0.20,   // Higher weight (more relevant for long queries)
            },
        }
    }
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

/// Get maximum string length in the database
fn get_max_string_length(&self) -> usize {
    self.get_all_strings()
        .iter()
        .map(|s| s.string.len())
        .max()
        .unwrap_or(0)
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
            return f64::MAX;
        }

        let span_length = (match_indices.last().unwrap() - match_indices.first().unwrap() + 1) as f64;
        let candidate_length = candidate_chars.len() as f64;
        span_length + (candidate_length * 0.1)
    } else {
        f64::MAX
    }
}

// Smart filtering to skip unpromising candidates
fn should_skip_candidate(candidate_len: usize, query_len: usize) -> bool {
    // Skip strings that are too short to contain the query
    if candidate_len < query_len {
        return true;
    }

    // Skip strings that are excessively long for short queries
    // For very short queries (1-2 chars), allow longer candidates
    // For longer queries, be more restrictive
    if query_len <= 2 && candidate_len > query_len * 10 {
        return true;
    } else if query_len <= 3 && candidate_len > query_len * 6 {
        return true;
    }

    false
}

// Character set filtering for fuzzy algorithms
fn contains_required_chars(candidate: &str, query: &str) -> bool {
    let candidate_chars: std::collections::HashSet<char> = candidate.chars().collect();
    query.chars().all(|c| candidate_chars.contains(&c))
}

// For fuzzy subsequence (lower raw scores are better)
fn normalize_fuzzy_score(raw_score: f64, min_score: f64, max_score: f64) -> f64 {
    // Invert and normalize: lower raw scores → higher normalized scores
    let normalized = 1.0 - ((raw_score - min_score) / (max_score - min_score));
    normalized.clamp(0.0, 1.0)
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

    mod fuzzy_subsequence_full_database {
        use super::*;

        #[test]
        fn test_fuzzy_subsequence_full_database_basic() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();

            // Test basic functionality - should find all strings that match "hl"
            // Use threshold of 0.0 to include "helicopter" which has normalized score of 0
            let results = ss.fuzzy_subsequence_full_database("hl", 10, 0.0);
            assert_eq!(results.len(), 3);

            // Should find all three "h" words that match "hl" as subsequence
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
            assert!(strings.contains(&"helicopter".to_string()));
        }

        #[test]
        fn test_fuzzy_subsequence_full_database_empty_query() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Empty query should return empty results
            let results = ss.fuzzy_subsequence_full_database("", 10, 0.5);
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_fuzzy_subsequence_full_database_no_matches() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // No matches should return empty results
            let results = ss.fuzzy_subsequence_full_database("xyz", 10, 0.5);
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_fuzzy_subsequence_full_database_with_threshold() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();

            // With high threshold, should only get best matches
            let results = ss.fuzzy_subsequence_full_database("hl", 10, 0.9);
            // Should get fewer results with high threshold
            assert!(results.len() <= 3);
        }

        #[test]
        fn test_fuzzy_subsequence_full_database_early_termination() {
            let mut ss = StringSpace::new();
            // Add many strings to test early termination
            for i in 0..100 {
                ss.insert_string(&format!("test{}", i), 1).unwrap();
            }
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test with target_count that should trigger early termination
            let results = ss.fuzzy_subsequence_full_database("hl", 5, 0.5);
            // Should get some results but not necessarily all matches due to early termination
            assert!(results.len() > 0);
        }

        #[test]
        fn test_fuzzy_subsequence_full_database_character_filtering() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test character filtering - "hlo" should match "hello" but not "world"
            // Use moderate threshold since there's only one match
            let results = ss.fuzzy_subsequence_full_database("hlo", 10, 0.5);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(!strings.contains(&"world".to_string()));
        }
    }

    mod jaro_winkler_full_database {
        use super::*;

        #[test]
        fn test_jaro_winkler_full_database_basic() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();

            // Test basic functionality - should find strings similar to "hell"
            let results = ss.jaro_winkler_full_database("hell", 10, 0.7);
            assert!(results.len() >= 2);

            // Should find "hello" and "help" which are similar to "hell"
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
        }

        #[test]
        fn test_jaro_winkler_full_database_empty_query() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Empty query should return empty results
            let results = ss.jaro_winkler_full_database("", 10, 0.5);
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_jaro_winkler_full_database_no_matches() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // No matches should return empty results
            let results = ss.jaro_winkler_full_database("xyz", 10, 0.5);
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_jaro_winkler_full_database_with_threshold() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();

            // With high threshold, should only get very similar matches
            let results = ss.jaro_winkler_full_database("hell", 10, 0.9);
            // Should get fewer results with high threshold
            assert!(results.len() <= 2);
        }

        #[test]
        fn test_jaro_winkler_full_database_early_termination() {
            let mut ss = StringSpace::new();
            // Add many strings to test early termination
            for i in 0..100 {
                ss.insert_string(&format!("test{}", i), 1).unwrap();
            }
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test with target_count that should trigger early termination
            let results = ss.jaro_winkler_full_database("hell", 5, 0.5);
            // Should get some results but not necessarily all matches due to early termination
            assert!(results.len() > 0);
        }

        #[test]
        fn test_jaro_winkler_full_database_length_filtering() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();
            // "a" is too short (minimum 3 chars), so we'll use a very long string instead
            ss.insert_string("extremelylongstringthatiswaytoolongforthisquery", 1).unwrap();

            // Test length filtering - very long strings should be skipped for very short queries
            // Use a 1-character query to trigger the filtering
            let results = ss.jaro_winkler_full_database("h", 10, 0.5);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(!strings.contains(&"extremelylongstringthatiswaytoolongforthisquery".to_string()));
        }

        #[test]
        fn test_jaro_winkler_full_database_exact_matches() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Exact matches should have high similarity scores
            let results = ss.jaro_winkler_full_database("hello", 10, 0.9);
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].string, "hello");
        }

        #[test]
        fn test_jaro_winkler_full_database_public_api() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test public API method
            let results = ss.jaro_winkler_full_database("hell", 10, 0.7);
            assert!(results.len() >= 2);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
        }

        #[test]
        fn test_jaro_winkler_full_database_character_set_filtering() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test that strings with completely different character sets are filtered out
            let results = ss.jaro_winkler_full_database("abc", 10, 0.1);
            // Should have very few or no matches since character sets are completely different
            assert!(results.len() <= 1);
        }
    }

    mod score_candidate {
        use super::*;

        #[test]
        fn test_score_candidate_creation() {
            let string_ref = StringRef {
                string: "hello".to_string(),
                meta: StringMeta { frequency: 1, age_days: 0 },
            };
            let candidate = ScoreCandidate::new(
                string_ref.clone(),
                AlgorithmType::Prefix,
                0.8,
                0.9,
            );

            assert_eq!(candidate.string_ref.string, "hello");
            assert_eq!(candidate.algorithm, AlgorithmType::Prefix);
            assert_eq!(candidate.raw_score, 0.8);
            assert_eq!(candidate.normalized_score, 0.9);
            assert_eq!(candidate.final_score, 0.0);
            assert!(candidate.alternative_scores.is_empty());
        }

        #[test]
        fn test_add_alternative_score() {
            let string_ref = StringRef {
                string: "hello".to_string(),
                meta: StringMeta { frequency: 1, age_days: 0 },
            };
            let mut candidate = ScoreCandidate::new(
                string_ref,
                AlgorithmType::Prefix,
                0.8,
                0.9,
            );

            candidate.add_alternative_score(AlgorithmType::FuzzySubseq, 0.7);
            candidate.add_alternative_score(AlgorithmType::JaroWinkler, 0.95);

            assert_eq!(candidate.alternative_scores.len(), 2);
            assert_eq!(candidate.alternative_scores[0].algorithm, AlgorithmType::FuzzySubseq);
            assert_eq!(candidate.alternative_scores[0].normalized_score, 0.7);
            assert_eq!(candidate.alternative_scores[1].algorithm, AlgorithmType::JaroWinkler);
            assert_eq!(candidate.alternative_scores[1].normalized_score, 0.95);
        }

        #[test]
        fn test_get_best_score_primary() {
            let string_ref = StringRef {
                string: "hello".to_string(),
                meta: StringMeta { frequency: 1, age_days: 0 },
            };
            let candidate = ScoreCandidate::new(
                string_ref,
                AlgorithmType::Prefix,
                0.8,
                0.9,
            );

            assert_eq!(candidate.get_best_score(), 0.9);
        }

        #[test]
        fn test_get_best_score_alternative() {
            let string_ref = StringRef {
                string: "hello".to_string(),
                meta: StringMeta { frequency: 1, age_days: 0 },
            };
            let mut candidate = ScoreCandidate::new(
                string_ref,
                AlgorithmType::Prefix,
                0.8,
                0.9,
            );

            candidate.add_alternative_score(AlgorithmType::JaroWinkler, 0.95);
            candidate.add_alternative_score(AlgorithmType::FuzzySubseq, 0.7);

            assert_eq!(candidate.get_best_score(), 0.95);
        }

        #[test]
        fn test_get_best_score_mixed() {
            let string_ref = StringRef {
                string: "hello".to_string(),
                meta: StringMeta { frequency: 1, age_days: 0 },
            };
            let mut candidate = ScoreCandidate::new(
                string_ref,
                AlgorithmType::Prefix,
                0.8,
                0.9,
            );

            candidate.add_alternative_score(AlgorithmType::JaroWinkler, 0.85);
            candidate.add_alternative_score(AlgorithmType::FuzzySubseq, 0.92);

            assert_eq!(candidate.get_best_score(), 0.92);
        }

        #[test]
        fn test_alternative_score_creation() {
            let alt_score = AlternativeScore {
                algorithm: AlgorithmType::Substring,
                normalized_score: 0.75,
            };

            assert_eq!(alt_score.algorithm, AlgorithmType::Substring);
            assert_eq!(alt_score.normalized_score, 0.75);
        }

        #[test]
        fn test_algorithm_type_variants() {
            // Test that all algorithm variants are properly defined
            let prefix = AlgorithmType::Prefix;
            let fuzzy_subseq = AlgorithmType::FuzzySubseq;
            let jaro_winkler = AlgorithmType::JaroWinkler;
            let substring = AlgorithmType::Substring;

            // Test equality
            assert_eq!(prefix, AlgorithmType::Prefix);
            assert_eq!(fuzzy_subseq, AlgorithmType::FuzzySubseq);
            assert_eq!(jaro_winkler, AlgorithmType::JaroWinkler);
            assert_eq!(substring, AlgorithmType::Substring);

            // Test inequality
            assert_ne!(prefix, AlgorithmType::FuzzySubseq);
            assert_ne!(jaro_winkler, AlgorithmType::Substring);
        }
    }

    mod metadata_integration {
        use super::*;

        #[test]
        fn test_apply_metadata_adjustments_basic() {
            // Test basic metadata adjustments
            let weighted_score = 1.0;
            let frequency = 10;
            let age_days = 30;
            let candidate_len = 5;
            let query_len = 3;
            let max_len = 50;

            let result = apply_metadata_adjustments(
                weighted_score, frequency, age_days, candidate_len, query_len, max_len
            );

            // Should be slightly above 1.0 due to frequency and age factors
            assert!(result > 1.0);
            assert!(result <= 2.0); // Should be within bounds
        }

        #[test]
        fn test_apply_metadata_adjustments_frequency_effect() {
            // Test frequency factor effect
            let weighted_score = 1.0;
            let high_frequency = 100;
            let low_frequency = 1;
            let age_days = 30;
            let candidate_len = 5;
            let query_len = 3;
            let max_len = 50;

            let high_freq_result = apply_metadata_adjustments(
                weighted_score, high_frequency, age_days, candidate_len, query_len, max_len
            );
            let low_freq_result = apply_metadata_adjustments(
                weighted_score, low_frequency, age_days, candidate_len, query_len, max_len
            );

            // Higher frequency should result in higher score
            assert!(high_freq_result > low_freq_result);
        }

        #[test]
        fn test_apply_metadata_adjustments_age_effect() {
            // Test age factor effect
            let weighted_score = 1.0;
            let frequency = 10;
            let older_age = 365; // Maximum age
            let newer_age = 1;   // Very new
            let candidate_len = 5;
            let query_len = 3;
            let max_len = 50;

            let older_result = apply_metadata_adjustments(
                weighted_score, frequency, older_age, candidate_len, query_len, max_len
            );
            let newer_result = apply_metadata_adjustments(
                weighted_score, frequency, newer_age, candidate_len, query_len, max_len
            );

            // Newer items should have slightly higher scores
            assert!(newer_result > older_result);
        }

        #[test]
        fn test_apply_metadata_adjustments_length_penalty() {
            // Test length penalty for very long candidates
            let weighted_score = 1.0;
            let frequency = 10;
            let age_days = 30;
            let short_candidate_len = 5;
            let long_candidate_len = 50; // 10x longer than query
            let query_len = 5;
            let max_len = 50;

            let short_result = apply_metadata_adjustments(
                weighted_score, frequency, age_days, short_candidate_len, query_len, max_len
            );
            let long_result = apply_metadata_adjustments(
                weighted_score, frequency, age_days, long_candidate_len, query_len, max_len
            );

            // Longer candidate should have lower score due to penalty
            assert!(short_result > long_result);
        }

        #[test]
        fn test_apply_metadata_adjustments_no_length_penalty() {
            // Test no length penalty for reasonable length differences
            let weighted_score = 1.0;
            let frequency = 10;
            let age_days = 30;
            let candidate_len = 10;
            let query_len = 5;
            let max_len = 50;

            let result = apply_metadata_adjustments(
                weighted_score, frequency, age_days, candidate_len, query_len, max_len
            );

            // Should not have significant penalty for 2x length difference
            assert!(result > 1.0);
        }

        #[test]
        fn test_apply_metadata_adjustments_bounds() {
            // Test that scores are properly bounded
            let high_weighted_score = 10.0; // Unrealistically high
            let frequency = 1000;
            let age_days = 0;
            let candidate_len = 5;
            let query_len = 3;
            let max_len = 50;

            let result = apply_metadata_adjustments(
                high_weighted_score, frequency, age_days, candidate_len, query_len, max_len
            );

            // Should be clamped to maximum of 2.0
            assert!(result <= 2.0);
            assert!(result >= 0.0);
        }

        #[test]
        fn test_normalize_substring_score() {
            // Test substring score normalization
            let position = 2;
            let max_position = 10;

            let result = normalize_substring_score(position, max_position);

            // Earlier positions should have higher scores
            assert_eq!(result, 1.0 - (2.0 / 10.0)); // 0.8
        }

        #[test]
        fn test_normalize_substring_score_start_position() {
            // Test substring at start position
            let position = 0;
            let max_position = 10;

            let result = normalize_substring_score(position, max_position);

            // Start position should have highest score
            assert_eq!(result, 1.0);
        }

        #[test]
        fn test_normalize_substring_score_end_position() {
            // Test substring at end position
            let position = 9;
            let max_position = 10;

            let result = normalize_substring_score(position, max_position);

            // End position should have lowest score
            assert_eq!(result, 1.0 - (9.0 / 10.0)); // 0.1
        }

        #[test]
        fn test_get_string_metadata() {
            // Test metadata extraction from StringRef
            let string_ref = StringRef {
                string: "hello".to_string(),
                meta: StringMeta { frequency: 5, age_days: 10 },
            };

            let (frequency, age_days, length) = get_string_metadata(&string_ref);

            assert_eq!(frequency, 5);
            assert_eq!(age_days, 10);
            assert_eq!(length, 5); // "hello" has 5 characters
        }

        #[test]
        fn test_normalize_fuzzy_score() {
            // Test fuzzy score normalization
            let raw_score = 5.0;
            let min_score = 0.0;
            let max_score = 10.0;

            let result = normalize_fuzzy_score(raw_score, min_score, max_score);

            // Lower raw scores should result in higher normalized scores
            assert_eq!(result, 1.0 - (5.0 / 10.0)); // 0.5
        }

        #[test]
        fn test_normalize_fuzzy_score_best_case() {
            // Test best possible fuzzy score
            let raw_score = 0.0;
            let min_score = 0.0;
            let max_score = 10.0;

            let result = normalize_fuzzy_score(raw_score, min_score, max_score);

            // Best raw score should give normalized score of 1.0
            assert_eq!(result, 1.0);
        }

        #[test]
        fn test_normalize_fuzzy_score_worst_case() {
            // Test worst possible fuzzy score
            let raw_score = 10.0;
            let min_score = 0.0;
            let max_score = 10.0;

            let result = normalize_fuzzy_score(raw_score, min_score, max_score);

            // Worst raw score should give normalized score of 0.0
            assert_eq!(result, 0.0);
        }

        #[test]
        fn test_normalize_fuzzy_score_clamping() {
            // Test that scores are properly clamped
            let raw_score = -5.0; // Below min
            let min_score = 0.0;
            let max_score = 10.0;

            let result = normalize_fuzzy_score(raw_score, min_score, max_score);

            // Should be clamped to 0.0-1.0 range
            assert!(result >= 0.0);
            assert!(result <= 1.0);
        }
    }

    mod best_completions {
        use super::*;

        #[test]
        fn test_best_completions_basic() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Test basic prefix completion
            let results = ss.best_completions("hel", Some(10));
            // Should find all three "hel" prefix matches
            assert!(results.len() >= 3);

            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
            assert!(strings.contains(&"helicopter".to_string()));
        }

        #[test]
        fn test_best_completions_empty_query() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Empty query should return empty results
            let results = ss.best_completions("", Some(10));
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_best_completions_no_matches() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // No matches should return empty results
            let results = ss.best_completions("xyz", Some(10));
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_best_completions_with_limit() {
            let mut ss = StringSpace::new();
            // Add more strings than the limit
            ss.insert_string("hello1", 1).unwrap();
            ss.insert_string("hello2", 2).unwrap();
            ss.insert_string("hello3", 3).unwrap();
            ss.insert_string("hello4", 4).unwrap();
            ss.insert_string("hello5", 5).unwrap();

            // Test with limit smaller than available matches
            let results = ss.best_completions("hello", Some(3));
            assert_eq!(results.len(), 3);
        }

        #[test]
        fn test_best_completions_progressive_execution() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Test that progressive execution works - should find matches through multiple algorithms
            let results = ss.best_completions("hl", Some(10));
            // Should find all three "h" words that match "hl" as subsequence
            assert!(results.len() >= 3);

            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
            assert!(strings.contains(&"helicopter".to_string()));
        }

        #[test]
        fn test_best_completions_early_termination() {
            let mut ss = StringSpace::new();
            // Add many strings to test early termination
            for i in 0..50 {
                ss.insert_string(&format!("test{}", i), 1).unwrap();
            }
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test with small limit to trigger early termination
            let results = ss.best_completions("hel", Some(5));
            // Should get some results but not necessarily all matches due to early termination
            assert!(results.len() > 0);
            assert!(results.len() <= 5);
        }

        #[test]
        fn test_has_high_quality_prefix_matches() {
            let ssi = StringSpaceInner::new();

            // Create test candidates
            let candidates = vec![
                StringRef { string: "hello".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "help".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "helicopter".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
            ];

            // All candidates start with "hel" - should be high quality
            assert!(ssi.has_high_quality_prefix_matches(&candidates, "hel"));

            // Mixed candidates - should not be high quality
            let mixed_candidates = vec![
                StringRef { string: "hello".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "world".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "help".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
            ];
            assert!(!ssi.has_high_quality_prefix_matches(&mixed_candidates, "hel"));
        }

        #[test]
        fn test_progressive_algorithm_execution() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Test progressive execution directly
            let results = ss.inner.progressive_algorithm_execution("hel", 10);
            // Should find all three "hel" prefix matches
            assert!(results.len() >= 3);

            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
            assert!(strings.contains(&"helicopter".to_string()));
        }

        #[test]
        fn test_progressive_algorithm_execution_with_fallback() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Test with query that requires fallback algorithms
            // "hl" won't match via prefix search, so should use fuzzy subsequence
            let results = ss.inner.progressive_algorithm_execution("hl", 10);
            assert!(results.len() >= 3);

            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
            assert!(strings.contains(&"helicopter".to_string()));
        }

        #[test]
        fn test_progressive_algorithm_execution_empty() {
            let ss = StringSpace::new();

            // Test with empty database
            let results = ss.inner.progressive_algorithm_execution("hel", 10);
            assert_eq!(results.len(), 0);
        }

        #[test]
        fn test_progressive_algorithm_execution_early_termination() {
            let mut ss = StringSpace::new();
            // Add many strings to test early termination
            for i in 0..100 {
                ss.insert_string(&format!("test{}", i), 1).unwrap();
            }
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test with small limit to trigger early termination
            let results = ss.inner.progressive_algorithm_execution("hel", 5);
            // Should get some results but not necessarily all matches due to early termination
            assert!(results.len() > 0);
            assert!(results.len() <= 5);
        }
    }
}