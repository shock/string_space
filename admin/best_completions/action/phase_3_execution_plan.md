# Phase 3 Execution Plan: Unified Scoring System

## Introduction

Phase 3 implements the comprehensive scoring system for the `best_completions` method, creating a unified framework that combines scores from multiple algorithms with dynamic weighting, metadata integration, and result ranking. This phase builds upon the progressive algorithm execution system implemented in Phase 2.

**Critical**: If any steps cannot be completed due to technical blockers, implementation complexity, or unexpected dependencies, execution should be aborted, status document updated with specific issues identified, and the user notified immediately.

## Pre-Implementation Steps

### Step 1: Master Plan Review
- Read the entire master plan document at `admin/best_completions/master_plan.md` to understand complete scope and context
- Focus on Phase 3 sections: Unified Scoring System implementation
- Review Phase 1 and Phase 2 dependencies to ensure proper integration

### Step 2: Status Assessment
- Scan `admin/best_completions/status/phase_2_execution_status.md` for current execution status
- Verify Phase 2 completion: 68/68 tests passing, progressive algorithm execution working
- **Risk Check**: If Phase 2 implementation is incomplete or tests are failing, stop and notify user

### Step 3: Test Suite Validation
- Run full test suite: `make test`
- **If tests fail and status shows they should pass**: Stop and notify user
- **If tests fail and status shows they were failing**: Make note and continue
- Expected baseline: 68/68 tests passing

### Step 4: Codebase Review
- Review existing `best_completions` method in `src/modules/string_space.rs`
- Understand current progressive algorithm execution implementation
- Review existing search algorithms and their integration points
- Examine `StringRef` and word struct metadata access patterns

## Implementation Steps

**Critical**: Use sub-agents for each implementation step sequentially, giving all the necessary information to complete each step.

### Step 1: Create `ScoreCandidate` Struct and Related Types

**File**: `src/modules/string_space.rs`

```rust
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

**Testing Requirements**:
- Unit tests for `ScoreCandidate` construction and methods
- Test alternative score addition and best score calculation
- Verify enum variants and hash/equality implementations

### Step 2: Implement Frequency, Age, and Length Normalization

**File**: `src/modules/string_space.rs`

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

**Testing Requirements**:
- Unit tests for metadata adjustment calculations
- Test edge cases (zero frequency, maximum age, extreme length mismatches)
- Verify score clamping and normalization boundaries
- Integration tests with actual word struct data

### Step 3: Create Dynamic Weighting System with Query Length Categorization

**File**: `src/modules/string_space.rs`

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

**Testing Requirements**:
- Unit tests for query length categorization
- Test weight assignments for each category
- Verify weight sums are reasonable (approximately 1.0)
- Integration tests with actual query scenarios

### Step 4: Implement Score Calculation Logic with Dynamic Weights

**File**: `src/modules/string_space.rs`

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

**Testing Requirements**:
- Unit tests for weighted score calculation
- Test final score calculation with various algorithm combinations
- Verify metadata integration works correctly
- Test edge cases (missing metadata, zero scores)

### Step 5: Implement Result Merging with Deduplication

**File**: `src/modules/string_space.rs`

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

**Testing Requirements**:
- Unit tests for candidate deduplication
- Test alternative score merging logic
- Verify final score calculation for merged candidates
- Integration tests with duplicate candidates from different algorithms

### Step 6: Create Final Ranking Logic

**File**: `src/modules/string_space.rs`

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

**Testing Requirements**:
- Unit tests for candidate ranking
- Test result limiting functionality
- Verify score ordering (descending)
- Test edge cases (empty candidates, single candidate)

### Step 7: Integrate with Progressive Algorithm Execution

**File**: `src/modules/string_space.rs`

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
```

**Testing Requirements**:
- Integration tests for complete `best_completions` flow
- Test early termination scenarios
- Verify backward compatibility with existing tests
- Performance tests with large datasets

### Step 8: Implement Detailed Score Collection

**File**: `src/modules/string_space.rs`

```rust
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

**Testing Requirements**:
- Unit tests for individual algorithm score calculations
- Test best algorithm selection logic
- Verify alternative score collection
- Integration tests with actual search scenarios

## Testing Strategy

### Unit Tests
- Test all new structs and enums (`ScoreCandidate`, `AlgorithmType`, `QueryLengthCategory`)
- Test individual scoring functions and normalization
- Test metadata integration and adjustment calculations
- Test dynamic weighting system
- Test result merging and deduplication

### Integration Tests
- Test complete `best_completions` flow with various query types
- Verify scoring system works with progressive algorithm execution
- Test edge cases (empty results, single candidate, exact matches)
- Test metadata integration with actual word struct data

### Performance Tests
- Verify scoring system doesn't significantly impact performance
- Test with large datasets to ensure scalability
- Monitor memory usage during candidate scoring

### Backward Compatibility
- All existing tests must continue to pass (68/68)
- Progressive algorithm execution from Phase 2 must remain functional
- Early termination scenarios must work correctly

## Error Handling
- Preserve existing error handling patterns from Phase 1 and Phase 2
- Handle missing metadata gracefully with default values
- Ensure score calculations handle edge cases (division by zero, NaN values)
- Maintain robust candidate deduplication and merging

## Sub-Agent Usage Recommendations

- Use sub-agents for atomic file modifications
- Use sub-agents for test execution and debugging
- Use sub-agents for performance profiling
- Use sub-agents for code review of complex implementations

## Next Steps

### Step 1: Final Test Run
- Run full test suite: `make test`
- Verify all 68+ new tests pass
- Ensure backward compatibility maintained

### Step 2: Status Documentation
- Create/update `admin/best_completions/status/phase_3_execution_status.md`
- Document current phase execution steps with status
- Document final status of the test suite
- Include summary of phase execution and accomplishments
- Note any risks or blocking issues
- End with single overarching next step for Phase 4

## Success Criteria
- All new scoring system components implemented according to master plan
- Unified scoring system integrates with progressive algorithm execution
- Dynamic weighting works correctly for different query lengths
- Metadata integration (frequency, age, length) functions properly
- All existing tests continue to pass (68/68)
- New unit and integration tests for Phase 3 functionality pass
- Performance characteristics meet expectations
- Code follows existing StringSpace patterns and conventions