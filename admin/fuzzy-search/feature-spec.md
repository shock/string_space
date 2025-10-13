# Fuzzy Subsequence Search Feature Specification

## Overview
This document specifies the implementation of a fuzzy subsequence search feature for the StringSpace Rust project. The feature will provide a new type of matching search similar to existing Python implementations, with full TCP protocol integration.

## Feature Description

Fuzzy subsequence search allows users to find strings where the query characters appear in order within the candidate string, but not necessarily consecutively. This is particularly useful for:
- Abbreviation matching (e.g., "g4" matches "openai/gpt-4o-2024-08-06")
- Partial word matching (e.g., "ogp5" matches "openai/gpt-5")
- Flexible pattern matching with character order preservation

## Implementation Requirements

### 1. Core Algorithm Implementation

#### 1.1 Subsequence Detection
```rust
fn is_subsequence(query: &str, candidate: &str) -> Option<Vec<usize>>
```
- Returns `Some(Vec<usize>)` containing indices of matched characters in candidate
- Returns `None` if query is not a subsequence of candidate
- Must handle UTF-8 strings correctly
- Should be case-sensitive (matching existing search behavior)

#### 1.2 Scoring Methods

**Option A: Span-Based Scoring** (Default)
```rust
fn score_match_span(match_indices: &[usize], candidate: &str) -> f64
```
- Score = span_length + (candidate_length * 0.1)
- Span length = last_match_index - first_match_index + 1
- Lower scores are better (closer matches)

**Option B: Normalized Scoring**
```rust
fn score_match_normalized(match_indices: &[usize], candidate: &str, max_len: usize) -> f64
```
- Normalized span = span_length / candidate_length
- Normalized length = candidate_length / max_candidate_length
- Score = 0.7 * normalized_span + 0.3 * normalized_length
- Lower scores are better (closer matches)

#### 1.3 Main Search Function
```rust
fn fuzzy_subsequence_search(&self, query: &str, scoring_method: ScoringMethod) -> Vec<StringRef>
```
- `ScoringMethod` enum with variants: `SpanBased`, `Normalized`
- Returns results sorted by score (ascending), then by frequency (descending), then by age (descending)
- Should handle empty query by returning empty vector
- Must respect existing string length constraints (3-50 characters)

### 2. StringSpace API Extension

Add to `StringSpace` struct in `src/modules/string_space.rs`:
```rust
pub fn fuzzy_subsequence_search(&self, query: &str, scoring_method: ScoringMethod) -> Vec<StringRef>

// Internal helper functions
fn is_subsequence(query: &str, candidate: &str) -> Option<Vec<usize>>
fn score_match_span(match_indices: &[usize], candidate: &str) -> f64
fn score_match_normalized(match_indices: &[usize], candidate: &str, max_len: usize) -> f64
```

### 3. Protocol Integration

#### 3.1 New Command: "fuzzy"

**Request Format:**
```
fuzzy<RS>query<RS>[scoring_method]
```

**Parameters:**
- `query`: The subsequence to search for (required)
- `scoring_method`: Optional parameter ("span" or "normalized"), defaults to "span"

**Response Format:**
Same as existing search commands - newline-separated list of matching strings, optionally with metadata if `SEND_METADATA` is true.

**Error Cases:**
- Invalid parameter count: "ERROR - invalid parameters (length = X)"
- Invalid scoring method: "ERROR - invalid scoring method 'X', expected 'span' or 'normalized'"

#### 3.2 Protocol Implementation

Add to `StringSpaceProtocol::create_response()` in `src/modules/protocol.rs`:
```rust
else if "fuzzy" == operation {
    // Parameter validation and fuzzy search implementation
}
```

### 4. Python Client Integration

Add to `StringSpaceClient` in `python/string_space_client/`:
```python
def fuzzy_search(self, query: str, scoring_method: str = "span") -> List[str]
```

## Test Plan

### 4.1 Unit Tests

Add to `string_space.rs` test module:

#### Subsequence Detection Tests
```rust
#[test]
fn test_is_subsequence_basic() {
    // Test basic subsequence matching
    // Test non-matching sequences
    // Test empty strings
    // Test exact matches
}

#[test]
fn test_is_subsequence_unicode() {
    // Test UTF-8 character handling
}
```

#### Scoring Tests
```rust
#[test]
fn test_score_match_span() {
    // Test span-based scoring calculations
}

#[test]
fn test_score_match_normalized() {
    // Test normalized scoring calculations
}
```

#### Integration Tests
```rust
#[test]
fn test_fuzzy_subsequence_search_basic() {
    // Test basic fuzzy search functionality
}

#[test]
fn test_fuzzy_subsequence_search_ranking() {
    // Test result ranking (score, frequency, age)
}

#[test]
fn test_fuzzy_subsequence_search_empty() {
    // Test empty query handling
}
```

### 4.2 Integration Tests

Extend `tests/client.py`:
```python
def fuzzy_test(client):
    try:
        # Test basic fuzzy search
        results = client.fuzzy_search("g4")
        print(f"Fuzzy search 'g4':")
        for result in results:
            print(f"  {result}")

        # Test normalized scoring
        results = client.fuzzy_search("ogp5", "normalized")
        print(f"Fuzzy search 'ogp5' (normalized):")
        for result in results:
            print(f"  {result}")

    except ProtocolError as e:
        print(f"ProtocolError: {e}")
```

### 4.3 Performance Tests

Add benchmark tests to compare with existing search methods:
- Compare performance against `get_similar_words`
- Test with various query lengths and dataset sizes
- Measure memory usage impact

## Implementation Strategy

### Phase 1: Core Algorithm (Week 1)
1. Implement `is_subsequence` helper function
2. Implement scoring functions (span-based and normalized)
3. Implement main `fuzzy_subsequence_search` method
4. Add comprehensive unit tests

### Phase 2: Protocol Integration (Week 1)
1. Add "fuzzy" command to protocol handler
2. Implement parameter validation and error handling
3. Add protocol-level unit tests

### Phase 3: Testing and Polish (Week 2)
1. Extend integration tests
2. Add Python client support
3. Performance benchmarking
4. Documentation updates

## Open Questions / TBD

1. **Case Sensitivity**: Should fuzzy search be case-sensitive like other searches?
   - Current analysis suggests yes for consistency

2. **Scoring Default**: Which scoring method should be the default?
   - Span-based seems more intuitive for most use cases

3. **Performance Optimization**: Should we implement any optimizations for large datasets?
   - Consider prefix filtering like existing `get_similar_words`

4. **Result Limit**: Should there be a default limit on results returned?
   - Current implementation returns all matches, consider adding limit parameter

5. **Metadata Integration**: How should frequency and age affect ranking?
   - Current plan: score primary, then frequency, then age

## Success Criteria

- [ ] All unit tests pass
- [ ] Integration tests demonstrate working TCP protocol
- [ ] Performance is acceptable for typical use cases
- [ ] Backward compatibility maintained
- [ ] Documentation updated
- [ ] Python client integration complete

## Dependencies

- No external dependencies required (uses existing string comparison)
- Must maintain compatibility with existing Rust version
- Python client must work with existing uv/pip setup

## Risk Assessment

- **Low Risk**: Algorithm implementation is straightforward
- **Medium Risk**: Protocol integration could affect existing commands
- **Low Risk**: Performance impact should be minimal
- **Medium Risk**: Unicode handling in subsequence detection

## Future Enhancements

1. **Configurable Scoring Weights**: Allow users to customize scoring formula
2. **Case Insensitive Option**: Add case-insensitive matching
3. **Result Limit Parameter**: Add optional limit to protocol
4. **Performance Optimizations**: Add indexing for common patterns

---

*This document will be updated as implementation progresses and new questions arise.*