# Master Plan: `best_completions` Method Implementation

## Overview

This document outlines the implementation strategy for a new `best_completions` method on the `StringSpaceInner` struct that intelligently combines multiple search algorithms to provide high-quality word completion suggestions.

## Core Design Philosophy

- **Multi-algorithm fusion**: Combine strengths of prefix, fuzzy subsequence, Jaro-Winkler, and substring search
- **Full database search**: Remove first-character prefix filtering for fuzzy algorithms to catch matches that start with different characters
- **Unified scoring**: Normalize all algorithm scores to a common 0.0-1.0 range with metadata integration
- **Performance vs accuracy trade-off**: Accept slower search for significantly better completion quality

## Algorithm Integration Strategy

### 1. Prefix Search
- **Method**: Use existing `find_by_prefix_no_sort`
- **Scoring**: Exact prefix matches get highest score (1.0), case-insensitive matches get 0.8
- **Advantage**: Fast and precise for prefix-based completion

### 2. Fuzzy Subsequence Search (Full Database)
- **Logic**: Adapt from `fuzzy_subsequence_search` but search entire database
- **Implementation**:
  ```rust
  // Instead of: let possibilities = self.find_by_prefix_no_sort(query[0..1].to_string().as_str());
  let possibilities = self.get_all_strings(); // Search entire database
  // Keep existing is_subsequence and score_match_span logic
  ```
- **Scoring**: Apply inverted normalization to convert span-based scores to 0.0-1.0 range
- **Advantage**: Finds matches where query characters appear in order but not necessarily consecutively

### 3. Jaro-Winkler Similarity (Full Database)
- **Logic**: Adapt from `get_similar_words` but search entire database
- **Implementation**:
  ```rust
  // Instead of: let possibilities = self.find_by_prefix_no_sort(word[0..1].to_string().as_str());
  let possibilities = self.get_all_strings(); // Search entire database
  // Keep existing jaro_winkler scoring and cutoff logic
  ```
- **Scoring**: Use existing 0.0-1.0 Jaro-Winkler similarity score (already normalized)
- **Advantage**: Handles typos, transpositions, and character substitutions

### 4. Substring Search
- **Method**: Use existing `find_with_substring`
- **Scoring**: Apply position-based normalization to convert match positions to 0.0-1.0 range
- **Advantage**: Finds matches where query appears anywhere in the string

## Unified Scoring System

### Important Note: Age Scoring Direction
- **Current Implementation**: `age_days` stores days since epoch (higher = more recent)
- **Existing Behavior**: Younger items (higher `age_days`) are preferred in current search methods
- **Consistency**: The `best_completions` method should maintain this same preference pattern

### Algorithm Scoring Analysis and Normalization

#### Current Scoring Characteristics

**Prefix Search**
- **Range**: Already normalized (1.0 exact, 0.8 case-insensitive)
- **Direction**: Higher = better ✓
- **Normalization**: None needed

**Fuzzy Subsequence Search**
- **Range**: Unbounded positive values (span-based scoring)
- **Direction**: Lower = better (ascending sort in current implementation) ✗
- **Normalization Required**: Inverted normalization to 0.0-1.0 scale

**Jaro-Winkler Similarity**
- **Range**: 0.0-1.0 (already normalized)
- **Direction**: Higher = better ✓
- **Normalization**: None needed

**Substring Search**
- **Range**: Position-based (earlier matches better)
- **Direction**: Lower position = better ✗
- **Normalization Required**: Position-based normalization to 0.0-1.0 scale

### Algorithm Weighting System

#### Static Weights (Baseline)
- **Prefix Search**: `0.35` (highest weight)
  - Most reliable for completion scenarios
  - Users expect prefix matches to appear first

- **Fuzzy Subsequence**: `0.30` (high weight)
  - Excellent for abbreviation-style input
  - Very useful for interactive completion

- **Jaro-Winkler**: `0.25` (medium weight)
  - Good for typo correction
  - Less critical than prefix/fuzzy for completion

- **Substring Search**: `0.10` (lowest weight)
  - Least relevant for completion scenarios
  - Useful as fallback

#### Dynamic Weighting System

**Query Length Categories and Weight Tables**

*Note: Implementation details moved to Phase 3: Unified Scoring System*

**Dynamic Weight Selection Implementation**

*Note: Implementation details moved to Phase 3: Unified Scoring System*

**Weight Validation and Effectiveness Testing**

*Note: Test implementation details moved to Phase 5: Testing and Optimization*

**Dynamic Weighting Strategy Rationale**

- **Very Short Queries (1-2 chars)**: Prioritize prefix and fuzzy subsequence since users are likely typing the beginning of words
- **Short Queries (3-4 chars)**: Balanced approach with emphasis on prefix and fuzzy subsequence
- **Medium Queries (5-6 chars)**: More balanced distribution as query provides more context
- **Long Queries (7+ chars)**: Emphasize Jaro-Winkler for typo correction and substring for partial matches

### Metadata Integration

#### Frequency Weighting with Conflict Resolution
- Use logarithmic scaling to prevent high-frequency words from dominating
- Formula: `frequency_factor = 1.0 + (ln(frequency + 1) * 0.1)`
- **Conflict Resolution**: Logarithmic scaling prevents extreme frequency values from overriding age and length preferences

#### Age-Based Recency Bonus with Bounded Influence
- Newer items get slight preference (higher `age_days` values are more recent)
- Formula: `age_factor = 1.0 + (current_age / max_age) * 0.05`
- **Conflict Resolution**: Small bounded influence (5% max) prevents age from overriding relevance

#### Length Normalization with Threshold
- Penalize only very long matches for short queries (when candidate_len > query_len * 3)
- Formula: `length_penalty = 1.0 - (candidate_len - query_len) / max_len * 0.1`
- **Conflict Resolution**: Length penalty only applied for significant length mismatches to avoid over-penalizing good matches

#### Metadata Factor Interaction Matrix

| Scenario | Frequency | Age | Length | Conflict Type | Resolution Strategy |
|----------|-----------|-----|--------|---------------|---------------------|
| High-freq old word | High | Old | Normal | Frequency vs Age | Log scaling limits frequency dominance |
| Low-freq new word | Low | New | Normal | Age preference | Age bonus provides slight advantage |
| Long high-freq word | High | Any | Long | Length vs Frequency | Length penalty capped, frequency log-scaled |
| Short low-freq word | Low | Any | Short | No conflict | All factors aligned |
| Medium-freq medium-age | Medium | Medium | Medium | Balanced | Multiplicative approach works well |

#### Enhanced Metadata Integration Implementation

*Note: Implementation details moved to Phase 3: Unified Scoring System*

## Implementation Phases

### Phase 1: Core Method Structure

#### Implementation Steps

**1. Add `best_completions` method signature to `StringSpaceInner` impl**

```rust
// In src/modules/string_space.rs, within the StringSpaceInner impl block
fn best_completions(&self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
    let limit = limit.unwrap_or(15);

    // Basic query validation
    if query.is_empty() {
        return Vec::new();
    }

    // TODO: Implement multi-algorithm fusion in subsequent phases
    // For now, return empty vector as placeholder
    Vec::new()
}
```

**2. Add public `best_completions` method to `StringSpace` struct**

```rust
// In src/modules/string_space.rs, within the StringSpace impl block
#[allow(unused)]
pub fn best_completions(&self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
    self.inner.best_completions(query, limit)
}
```

**3. Implement basic query validation and empty query handling**

```rust
// Query validation helper function
fn validate_query(query: &str) -> Result<(), &'static str> {
    if query.is_empty() {
        return Err("Query cannot be empty");
    }

    // Additional validation can be added here
    // For example: minimum length requirements, character restrictions, etc.

    Ok(())
}

// Enhanced best_completions method with validation
fn best_completions(&self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
    let limit = limit.unwrap_or(15);

    // Validate query
    if let Err(_) = validate_query(query) {
        return Vec::new();
    }

    // TODO: Implement multi-algorithm fusion in subsequent phases
    // For now, return empty vector as placeholder
    Vec::new()
}
```

**4. Create result collection infrastructure**

```rust
// Basic result collection structure for Phase 1
// This will be expanded in Phase 3 with the full ScoreCandidate struct

struct BasicCandidate {
    string_ref: StringRef,
    algorithm: AlgorithmType,
    score: f64,
}

impl BasicCandidate {
    fn new(string_ref: StringRef, algorithm: AlgorithmType, score: f64) -> Self {
        Self {
            string_ref,
            algorithm,
            score,
        }
    }
}


// Result collection helper
fn collect_results(&self) -> Vec<BasicCandidate> {
    // Placeholder implementation
    // Will be replaced with actual algorithm execution in subsequent phases
    Vec::new()
}
```

### Phase 2: Individual Algorithm Integration

#### Implementation Steps

**1. Implement full-database fuzzy subsequence search with early termination**

```rust
// Full-database fuzzy subsequence search with early termination
fn fuzzy_subsequence_full_database(
    &self,
    query: &str,
    target_count: usize,
    score_threshold: f64
) -> Vec<StringRef> {
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

    // Handle edge case where all scores are the same
    if (max_score - min_score).abs() < f64::EPSILON {
        min_score = 0.0;
        max_score = 1.0;
    }

    // Second pass: apply normalization and threshold filtering
    for (string_ref, raw_score) in scores {
        let normalized_score = normalize_fuzzy_score(raw_score, min_score, max_score);

        if normalized_score >= score_threshold {
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
    let candidate = string_ref.as_str();

    // Apply smart filtering to skip unpromising candidates
    if should_skip_candidate(candidate.len(), query.len()) {
        return None;
    }

    if !contains_required_chars(candidate, query) {
        return None;
    }

    // Use existing fuzzy subsequence logic from the codebase
    // This adapts the existing fuzzy_subsequence_search but searches entire database
    let query_chars: Vec<char> = query.chars().collect();
    let candidate_chars: Vec<char> = candidate.chars().collect();

    if !is_subsequence(&query_chars, &candidate_chars) {
        return None;
    }

    // Calculate match span score (lower is better)
    let score = score_match_span(&query_chars, &candidate_chars);
    Some(score)
}

```

**2. Implement full-database Jaro-Winkler similarity search with early termination**

```rust
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
        let candidate = string_ref.as_str();

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
```

**3. Integrate existing prefix and substring search methods**

```rust
// Use existing prefix search method (already efficient)
fn prefix_search(&self, query: &str) -> Vec<StringRef> {
    self.find_by_prefix_no_sort(query)
}

// Use existing substring search method
fn substring_search(&self, query: &str) -> Vec<StringRef> {
    self.find_with_substring(query)
}

```

**4. Performance optimization strategies**

```rust
// Smart filtering to skip unpromising candidates
fn should_skip_candidate(candidate_len: usize, query_len: usize) -> bool {
    // Skip strings that are too short to contain the query
    if candidate_len < query_len {
        return true;
    }

    // Skip strings that are excessively long for short queries
    if query_len <= 3 && candidate_len > query_len * 4 {
        return true;
    }

    false
}

// Character set filtering for fuzzy algorithms
fn contains_required_chars(candidate: &str, query: &str) -> bool {
    let candidate_chars: HashSet<char> = candidate.chars().collect();
    query.chars().all(|c| candidate_chars.contains(&c))
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
    if all_candidates.len() >= limit && has_high_quality_prefix_matches(&all_candidates, query) {
        return all_candidates.into_iter().take(limit).collect();
    }

    // 2. Fuzzy subsequence with early termination (O(n) with early exit)
    let remaining_needed = limit.saturating_sub(all_candidates.len());
    if remaining_needed > 0 {
        let fuzzy_candidates = self.fuzzy_subsequence_full_database(
            query,
            remaining_needed,
            0.7 // score threshold
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
fn has_high_quality_prefix_matches(candidates: &[StringRef], query: &str) -> bool {
    candidates.iter()
        .filter(|c| c.as_str().starts_with(query))
        .count() >= candidates.len() / 2
}
```

### Phase 3: Unified Scoring System

#### Implementation Steps

**1. Create `ScoreCandidate` struct and related types**

```rust
// In src/modules/string_space.rs

/// Represents a candidate string with scoring information from multiple algorithms
#[derive(Debug, Clone)]
struct ScoreCandidate {
    string_ref: StringRef,
    algorithm: AlgorithmType,
    raw_score: f64,
    normalized_score: f64,
    final_score: f64,
    alternative_scores: Vec<AlternativeScore>,
}

/// Alternative score from other algorithms for the same string
#[derive(Debug, Clone)]
struct AlternativeScore {
    algorithm: AlgorithmType,
    normalized_score: f64,
}

/// Algorithm type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AlgorithmType {
    PREFIX,
    FUZZY_SUBSEQ,
    JARO_WINKLER,
    SUBSTRING,
}

impl ScoreCandidate {
    fn new(string_ref: StringRef, algorithm: AlgorithmType, raw_score: f64, normalized_score: f64) -> Self {
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
    fn add_alternative_score(&mut self, algorithm: AlgorithmType, normalized_score: f64) {
        self.alternative_scores.push(AlternativeScore {
            algorithm,
            normalized_score,
        });
    }

    /// Get the best available score for this candidate (primary or alternative)
    fn get_best_score(&self) -> f64 {
        let mut best_score = self.normalized_score;
        for alt in &self.alternative_scores {
            if alt.normalized_score > best_score {
                best_score = alt.normalized_score;
            }
        }
        best_score
    }
}
```

**2. Implement frequency, age, and length normalization**

```rust
// Metadata integration functions
fn apply_metadata_adjustments(
    weighted_score: f64,
    frequency: u32,
    age_days: u32,
    candidate_len: usize,
    query_len: usize,
    max_len: usize
) -> f64 {
    // 1. Frequency factor with logarithmic scaling to prevent dominance
    let frequency_factor = 1.0 + (ln(frequency as f64 + 1.0) * 0.1);

    // 2. Age factor with bounded influence (newer items get slight preference)
    let max_age = 365; // Maximum age in days for normalization
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

// Score normalization functions

/// For fuzzy subsequence (lower raw scores are better)
fn normalize_fuzzy_score(raw_score: f64, min_score: f64, max_score: f64) -> f64 {
    // Invert and normalize: lower raw scores → higher normalized scores
    let normalized = 1.0 - ((raw_score - min_score) / (max_score - min_score));
    normalized.clamp(0.0, 1.0)
}

/// For substring search (earlier matches are better)
fn normalize_substring_score(position: usize, max_position: usize) -> f64 {
    1.0 - (position as f64 / max_position as f64)
}

/// Get metadata for a string reference
fn get_string_metadata(&self, string_ref: &StringRef) -> (u32, u32, usize) {
    // Get the word struct to access frequency and age
    if let Some(word_struct) = self.get_word_struct(string_ref) {
        (word_struct.frequency, word_struct.age_days, string_ref.as_str().len())
    } else {
        // Default values if word struct not found
        (1, 0, string_ref.as_str().len())
    }
}
```

**3. Create dynamic weighting system with query length categorization**

```rust
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
```

**4. Implement score calculation logic with dynamic weights**

```rust
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
        AlgorithmType::PREFIX => prefix_score = candidate.normalized_score,
        AlgorithmType::FUZZY_SUBSEQ => fuzzy_score = candidate.normalized_score,
        AlgorithmType::JARO_WINKLER => jaro_score = candidate.normalized_score,
        AlgorithmType::SUBSTRING => substring_score = candidate.normalized_score,
    }

    // Add alternative scores
    for alt in &candidate.alternative_scores {
        match alt.algorithm {
            AlgorithmType::PREFIX => prefix_score = prefix_score.max(alt.normalized_score),
            AlgorithmType::FUZZY_SUBSEQ => fuzzy_score = fuzzy_score.max(alt.normalized_score),
            AlgorithmType::JARO_WINKLER => jaro_score = jaro_score.max(alt.normalized_score),
            AlgorithmType::SUBSTRING => substring_score = substring_score.max(alt.normalized_score),
        }
    }

    // Calculate weighted algorithm score
    let weighted_score = calculate_weighted_score(
        prefix_score, fuzzy_score, jaro_score, substring_score, query
    );

    // Apply metadata adjustments
    let (frequency, age_days, candidate_len) = string_space.get_string_metadata(&candidate.string_ref);
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

/// Get maximum string length in the database
fn get_max_string_length(&self) -> usize {
    self.get_all_strings()
        .iter()
        .map(|s| s.as_str().len())
        .max()
        .unwrap_or(0)
}
```

### Phase 4: Result Processing

#### Implementation Steps

**1. Implement result merging with deduplication**

```rust
/// Merge candidates from different algorithms and calculate final scores
fn merge_and_score_candidates(
    candidates: Vec<ScoreCandidate>,
    query: &str,
    string_space: &StringSpaceInner
) -> Vec<ScoreCandidate> {
    let mut merged: HashMap<StringRef, ScoreCandidate> = HashMap::new();

    // Merge candidates by string reference
    for candidate in candidates {
        if let Some(existing) = merged.get_mut(&candidate.string_ref) {
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
            merged.insert(candidate.string_ref.clone(), candidate);
        }
    }

    // Calculate final scores for all merged candidates
    let mut scored_candidates: Vec<ScoreCandidate> = merged.into_values().collect();
    for candidate in &mut scored_candidates {
        calculate_final_score(candidate, query, string_space);
    }

    scored_candidates
}
```

**2. Create final ranking logic**

```rust
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
```

**3. Add result limiting and integrate with progressive algorithm execution**

```rust
/// Complete best_completions method that integrates all phases
fn best_completions(&self, query: &str, limit: Option<usize>) -> Vec<StringRef> {
    let limit = limit.unwrap_or(15);

    // Basic query validation
    if query.is_empty() {
        return Vec::new();
    }

    // Use progressive algorithm execution from Phase 2
    let all_candidates = self.progressive_algorithm_execution(query, limit);

    // If we have enough high-quality prefix matches, return them directly
    if all_candidates.len() >= limit && self.has_high_quality_prefix_matches(&all_candidates, query) {
        return all_candidates.into_iter().take(limit).collect();
    }

    // Otherwise, collect detailed scores from all algorithms
    let mut scored_candidates = self.collect_detailed_scores(query, &all_candidates);

    // Merge duplicate candidates and calculate final scores
    let merged_candidates = self.merge_and_score_candidates(scored_candidates, query);

    // Sort by final score
    let mut ranked_candidates = merged_candidates;
    self.rank_candidates_by_score(&mut ranked_candidates);

    // Apply limit and return
    self.limit_and_convert_results(ranked_candidates, limit)
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
            prefix_score, fuzzy_score, jaro_score, substring_score
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

/// Helper struct for algorithm scores
struct AlgorithmScore {
    algorithm: AlgorithmType,
    raw_score: f64,
    normalized_score: f64,
}

/// Calculate prefix score for a candidate
fn calculate_prefix_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
    let candidate = string_ref.as_str();

    if candidate.starts_with(query) {
        Some(AlgorithmScore {
            algorithm: AlgorithmType::PREFIX,
            raw_score: 1.0,
            normalized_score: 1.0,
        })
    } else if candidate.to_lowercase().starts_with(&query.to_lowercase()) {
        Some(AlgorithmScore {
            algorithm: AlgorithmType::PREFIX,
            raw_score: 0.8,
            normalized_score: 0.8,
        })
    } else {
        None
    }
}

/// Calculate fuzzy subsequence score for a candidate
fn calculate_fuzzy_subsequence_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
    if let Some(raw_score) = self.score_fuzzy_subsequence(string_ref, query) {
        // For fuzzy subsequence, we need min/max for normalization
        // This would be calculated during the initial search phase
        let normalized_score = self.normalize_fuzzy_score(raw_score, 0.0, 100.0); // Placeholder values
        Some(AlgorithmScore {
            algorithm: AlgorithmType::FUZZY_SUBSEQ,
            raw_score,
            normalized_score,
        })
    } else {
        None
    }
}

/// Calculate Jaro-Winkler score for a candidate
fn calculate_jaro_winkler_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
    let candidate = string_ref.as_str();
    let similarity = jaro_winkler(query, candidate);

    if similarity >= 0.7 { // Threshold for meaningful matches
        Some(AlgorithmScore {
            algorithm: AlgorithmType::JARO_WINKLER,
            raw_score: similarity,
            normalized_score: similarity, // Already normalized
        })
    } else {
        None
    }
}

/// Calculate substring score for a candidate
fn calculate_substring_score(&self, string_ref: &StringRef, query: &str) -> Option<AlgorithmScore> {
    let candidate = string_ref.as_str();

    if let Some(position) = candidate.find(query) {
        let normalized_score = self.normalize_substring_score(position, candidate.len());
        Some(AlgorithmScore {
            algorithm: AlgorithmType::SUBSTRING,
            raw_score: position as f64,
            normalized_score,
        })
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
    let mut best_algorithm = AlgorithmType::PREFIX;

    // Compare all available scores and select the best one
    if let Some(score) = prefix_score {
        best_score = Some(score);
        best_algorithm = AlgorithmType::PREFIX;
    }

    if let Some(score) = fuzzy_score {
        if best_score.as_ref().map_or(true, |best| score.normalized_score > best.normalized_score) {
            best_score = Some(score);
            best_algorithm = AlgorithmType::FUZZY_SUBSEQ;
        }
    }

    if let Some(score) = jaro_score {
        if best_score.as_ref().map_or(true, |best| score.normalized_score > best.normalized_score) {
            best_score = Some(score);
            best_algorithm = AlgorithmType::JARO_WINKLER;
        }
    }

    if let Some(score) = substring_score {
        if best_score.as_ref().map_or(true, |best| score.normalized_score > best.normalized_score) {
            best_score = Some(score);
            best_algorithm = AlgorithmType::SUBSTRING;
        }
    }

    // If no algorithm found a match, use a fallback
    let fallback_score = AlgorithmScore {
        algorithm: AlgorithmType::SUBSTRING,
        raw_score: 0.0,
        normalized_score: 0.0,
    };

    (best_algorithm, best_score.unwrap_or(fallback_score))
}
```
