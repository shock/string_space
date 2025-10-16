# Phase 1 Execution Plan: Core Algorithm Implementation

## Introduction

**Phase 1** implements the core fuzzy-subsequence search algorithm including subsequence detection, span-based scoring, and the main search method with prefix filtering. This phase establishes the foundational algorithm that will be integrated into the StringSpace API in subsequent phases.

**CRITICAL**: If any step cannot be completed due to compilation errors, test failures, or unexpected codebase changes, execution should be aborted, the status document updated with the specific issue, and the user notified immediately.

## Pre-Implementation Steps

**IMPORTANT**: These steps should NOT use sub-agents and should be executed by the main agent.

### Step 1: Master Plan Review
- Read the entire master plan document at `admin/fuzzy-subsequence-search/master_plan.md` to understand complete scope and context
- Focus on Phase 1 sections: Core Algorithm Implementation and Implementation Details

### Step 2: Status Assessment
- Read `admin/fuzzy-subsequence-search/status/phase_0_execution_status.md` to understand current state
- Verify Phase 0 was completed successfully with all tests passing
- **Risk Check**: Confirm no blocking issues from Phase 0

### Step 3: Test Suite Validation
- Run full test suite using `make test`
- **If tests fail and status shows they should pass**: Stop and notify user
- **If tests fail and status shows they were failing**: Make note and continue
- **Current Baseline**: All 14 tests should pass as confirmed in Phase 0

### Step 4: Codebase Review
- Review `src/modules/string_space.rs` to understand existing search method patterns
- Review existing test infrastructure in `string_space.rs` (lines 584-791)
- Verify understanding of `StringRef` structure and string access patterns

## Implementation Steps

### Step 1: Implement Subsequence Detection Helper

**Objective**: Add `is_subsequence()` as a private standalone function following existing patterns

**Implementation Details**:
```rust
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
```

**Location**: Add as private standalone function in `src/modules/string_space.rs` after existing helper functions like `similar()` and `get_close_matches()`

**Key Requirements**:
- UTF-8 character handling using `chars()` iterator
- Case-sensitive matching (consistent with existing search behavior)
- Early termination when no match possible
- Returns `Some(Vec<usize>)` with match indices or `None` if no match

**Testing Requirements**:
- Unit tests for basic subsequence matching
- Unit tests for non-matching sequences
- UTF-8 character handling tests
- Edge cases (empty strings, single characters)

### Step 2: Implement Scoring Function

**Objective**: Add `score_match_span()` as a private standalone function for span-based scoring

**Implementation Details**:
```rust
fn score_match_span(match_indices: &[usize], candidate: &str) -> f64 {
    if match_indices.is_empty() {
        return f64::MAX;
    }
    let span_length = (match_indices.last().unwrap() - match_indices.first().unwrap() + 1) as f64;
    let candidate_length = candidate.len() as f64;
    span_length + (candidate_length * 0.1)
}
```

**Location**: Add as private standalone function in `src/modules/string_space.rs` after `is_subsequence()`

**Key Requirements**:
- Lower scores indicate better matches (more compact subsequences)
- Span length calculation: last_match_index - first_match_index + 1
- Candidate length penalty: candidate_length * 0.1
- Returns `f64::MAX` for empty match indices

**Testing Requirements**:
- Unit tests for scoring calculations
- Tests for empty match indices
- Verification that lower scores represent better matches

### Step 3: Implement Main Search Method on StringSpaceInner

**Objective**: Add `fuzzy_subsequence_search()` method to `StringSpaceInner` struct

**Implementation Details**:
```rust
fn fuzzy_subsequence_search(&self, query: &str) -> Vec<StringRef> {
    // Empty query handling: return empty vector for empty queries
    // This is consistent with existing search method behavior where empty queries yield no matches
    if query.is_empty() {
        return Vec::new();
    }

    // Use prefix filtering like get_similar_words for performance
    // Identical implementation to get_similar_words()
    let possibilities = self.find_by_prefix(query[0..1].to_string().as_str());

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
            .then(b.0.meta.frequency.cmp(&a.0.meta.frequency))
            .then(b.0.meta.age_days.cmp(&a.0.meta.age_days))
    });

    // Limit to top 10 results AFTER all sorting is complete
    // This ensures the best 10 matches are selected based on the full sorting criteria
    matches.truncate(10);

    matches.into_iter().map(|(string_ref, _)| string_ref).collect()
}
```

**Location**: Add as method to `StringSpaceInner` implementation in `src/modules/string_space.rs`

**Key Requirements**:
- **Empty query handling**: Returns empty vector for empty queries, consistent with existing search method behavior
- **Prefix filtering**: Uses identical implementation to `get_similar_words()` with `query[0..1].to_string().as_str()`
- **String access**: Directly accesses `candidate.string` field, consistent with all existing search methods
- **Sorting strategy**: Score ascending (lower scores are better), frequency descending, age descending
- **Result limiting**: Limits to top 10 results after all sorting is complete using `matches.truncate(10)`
- **Performance optimization**: Prefix filtering significantly reduces search space

**Testing Requirements**:
- Unit tests for basic subsequence matching
- Unit tests for empty query handling
- Unit tests for result ranking verification
- Unit tests for UTF-8 character handling
- Unit tests for abbreviation matching scenarios

### Step 4: Add Comprehensive Unit Tests

**Objective**: Extend existing test infrastructure with comprehensive fuzzy-subsequence search tests

**Implementation Details**:
```rust
// Add to the existing tests module in string_space.rs (around line 790, after mod get_similar_words)
mod fuzzy_subsequence_search {
    use super::*;

    #[test]
    fn test_basic_subsequence_matching() {
        let mut ss = StringSpace::new();
        ss.insert_string("hello", 1).unwrap();
        ss.insert_string("world", 2).unwrap();
        ss.insert_string("help", 3).unwrap();
        ss.insert_string("helicopter", 1).unwrap();

        let results = ss.fuzzy_subsequence_search("hl");
        assert_eq!(results.len(), 2);
        assert!(results[0].string == "hello");
        assert!(results[1].string == "help");
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
    fn test_empty_query_handling() {
        let mut ss = StringSpace::new();
        ss.insert_string("hello", 1).unwrap();
        ss.insert_string("world", 2).unwrap();

        let results = ss.fuzzy_subsequence_search("");
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

        let results = ss.fuzzy_subsequence_search("g4");
        assert_eq!(results.len(), 1);
        assert!(results[0].string == "openai/gpt-4o-2024-08-06");

        let results = ss.fuzzy_subsequence_search("ogp5");
        assert_eq!(results.len(), 1);
        assert!(results[0].string == "openai/gpt-5");
    }
}
```

**Location**: Add within the existing `#[cfg(test)] mod tests` section in `src/modules/string_space.rs` after the `mod get_similar_words` module

**Key Requirements**:
- Follow existing test patterns and organization
- Use same test naming conventions (`test_` prefix)
- Follow same assertion patterns and setup approaches
- Create new `mod fuzzy_subsequence_search` module within existing test module

**Test Scenarios**:
- Basic subsequence matching
- Non-matching sequences
- Empty query handling
- Exact matches
- UTF-8 character handling
- Result ranking verification
- Abbreviation matching

### Step 5: Verify Compilation and Run Tests

**Objective**: Ensure all new code compiles correctly and tests pass

**Implementation Details**:
- Run `cargo build` to verify compilation
- Run `cargo test` to run all unit tests including new fuzzy-subsequence tests
- **If compilation fails**: Debug and fix compilation errors
- **If tests fail**: Debug and fix failing tests
- **Expected Outcome**: All existing tests continue to pass, new fuzzy-subsequence tests pass

## Next Steps

### Step 1: Final Test Run
- Run full test suite using `make test`
- Verify all tests pass including integration tests
- Confirm no performance regressions

### Step 2: Status Documentation
- Create `admin/fuzzy-subsequence-search/status/phase_1_execution_status.md`
- Document current phase execution steps with status ("COMPLETED", "IN PROGRESS", "NOT STARTED", "NEEDS CLARIFICATION")
- Document final status of the test suite
- Include a summary of the phase execution and what was accomplished
- Note any risks or blocking issues
- End the status document with a single overarching next step

## Critical Requirements Checklist

### TESTING REQUIREMENTS
- [ ] Unit tests for `is_subsequence()` function
- [ ] Unit tests for `score_match_span()` function
- [ ] Unit tests for `fuzzy_subsequence_search()` method
- [ ] UTF-8 character handling tests
- [ ] Empty query handling tests
- [ ] Result ranking verification tests
- [ ] All existing tests continue to pass

### BACKWARD COMPATIBILITY
- [ ] No changes to existing search methods
- [ ] No changes to existing protocol commands
- [ ] No changes to existing API
- [ ] All existing functionality preserved

### ERROR HANDLING
- [ ] Empty queries return empty results (consistent with existing behavior)
- [ ] UTF-8 character handling preserves existing error patterns
- [ ] No new error conditions introduced

### STATUS TRACKING
- [ ] Status document creation/update instructions included
- [ ] Progress tracking for each implementation step
- [ ] Risk assessment and issue documentation

## Implementation Notes

- **Sub-Agent Usage**: Consider using sub-agents for atomic operations like file creation/modification, test execution, and debugging
- **Code Location**: All implementation should be in `src/modules/string_space.rs`
- **Pattern Consistency**: Follow existing code patterns for helper functions, method signatures, and test organization
- **Performance**: Prefix filtering should maintain good performance characteristics
- **String Access**: Use direct `candidate.string` access consistent with all existing search methods