use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter, BufRead};
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};

mod tests;

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

    /// Collect detailed scores for candidates from all algorithms
    fn collect_detailed_scores(&self, query: &str, candidates: &[StringRef]) -> Vec<ScoreCandidate> {
        let mut scored_candidates = Vec::new();

        for string_ref in candidates {
            // Calculate scores from all algorithms for this candidate
            let prefix_score = self.calculate_prefix_score(string_ref, query);
            let fuzzy_score = self.calculate_fuzzy_subsequence_score(string_ref, query);
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

    /// Calculate prefix match score with case-insensitive support
    fn calculate_prefix_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
        let candidate = string_ref.string.as_str();

        // Case-insensitive prefix matching
        if candidate.to_lowercase().starts_with(&query.to_lowercase()) {
            // Perfect prefix match gets maximum score
            Some(AlgorithmScore::new(
                AlgorithmType::Prefix,
                1.0,  // raw score
                1.0   // normalized score
            ))
        } else {
            None
        }
    }

    /// Calculate fuzzy subsequence score with normalization
    fn calculate_fuzzy_subsequence_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
        let candidate = string_ref.string.as_str();

        // Apply smart filtering to skip unpromising candidates
        // For fuzzy subsequence, be more lenient with length filtering to allow abbreviation matching
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

        // For normalization, we need min/max scores across all candidates
        // Since we don't have all candidates here, use a reasonable range
        let min_score = 0.0;
        let max_score = candidate.len() as f64 * 2.0; // Reasonable upper bound

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

        // Apply threshold - only include reasonable matches
        if similarity < 0.6 {
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
        // Use fuzzy-specific filtering for abbreviation matching
        if should_skip_candidate_fuzzy(candidate.len(), query.len()) {
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

// More lenient filtering for fuzzy subsequence search to allow abbreviation matching
fn should_skip_candidate_fuzzy(candidate_len: usize, query_len: usize) -> bool {
    // Skip strings that are too short to contain the query
    if candidate_len < query_len {
        return true;
    }

    // For fuzzy subsequence, be much more lenient with length filtering
    // since it's designed for abbreviation matching
    if query_len <= 2 && candidate_len > query_len * 20 {
        return true;
    } else if query_len <= 3 && candidate_len > query_len * 15 {
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
