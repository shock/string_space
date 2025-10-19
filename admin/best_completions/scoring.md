# Best Completions Scoring Algorithm

## Overview

The `best_completions` method in StringSpaceInner implements a sophisticated multi-algorithm scoring system that intelligently combines four different search algorithms with dynamic weighting based on query length. This document details the complete scoring and ranking logic.

## Key Implementation Details (Updated)

This documentation has been updated to accurately reflect the actual code implementation. Key corrections include:

- **Progressive Execution**: Added 5th step (first character fallback)
- **Early Termination**: Fixed threshold from ≥2/3 to ≥ limit
- **Jaro-Winkler**: Added adaptive threshold (0.6 for 1-2 chars, 0.7 for 3+ chars)
- **Substring**: Added usage restriction (only for queries ≥2 chars)
- **Fuzzy Filtering**: Added detailed length-based filtering rules
- **Metadata**: Added maximum age (365 days) and max_length parameters
- **Special Cases**: Added first character fallback details
- **UTF-8**: Added comprehensive Unicode handling section

## Algorithm Flow

```
best_completions(query, limit)
├── Query Validation
├── Progressive Algorithm Execution
├── Candidate Scoring (Two-Pass Approach)
│   ├── First Pass: Collect Fuzzy Raw Scores
│   ├── Calculate Global Min/Max for Fuzzy Normalization
│   └── Second Pass: Score All Algorithms with Consistent Ranges
├── Dynamic Weighting by Query Length
├── Metadata Integration
├── Final Score Calculation
└── Ranking & Limiting
```

## 1. Progressive Algorithm Execution

The system uses a progressive execution strategy to efficiently gather candidates:

### Execution Order
1. **Prefix Search** (O(log n)) - Fast exact prefix matching
2. **Fuzzy Subsequence** (O(n) with early exit) - Character order-preserving search
3. **Jaro-Winkler** (O(n) with early exit) - Typo correction and similarity
4. **Substring Search** (O(n)) - Fallback for queries ≥2 characters
5. **First Character Fallback** - Last resort using first character prefix search

### Early Termination
- If enough high-quality prefix matches are found (≥ limit), return them directly
- Each algorithm stops early if target count is reached
- Progressive execution fills remaining slots after each algorithm

## 2. Individual Algorithm Scoring

### 2.1 Prefix Matching
**Algorithm Type**: `AlgorithmType::Prefix`

**Scoring Logic**:
- **Raw Score**:
  - 1.0 for exact case-sensitive prefix match
  - 0.9999 for case-insensitive prefix match
  - None for non-matches
- **Normalized Score**:
  - 1.0 for case-sensitive matches
  - 0.9999 for case-insensitive matches
  - 0.0 for non-matches
- **Formula**:
  ```
  if candidate.starts_with(query):
      score = 1.0
  elif candidate.to_lowercase().starts_with(query.to_lowercase()):
      score = 0.9999
  else:
      score = None
  ```

### 2.2 Fuzzy Subsequence
**Algorithm Type**: `AlgorithmType::FuzzySubseq`

**Scoring Logic**:
- **Raw Score**: Match span length + 10% of candidate length
  - `raw_score = span_length + (candidate_length * 0.1)`
  - Where `span_length = last_match_index - first_match_index + 1`
- **Normalized Score**: Inverted and normalized using **global min/max** across all candidates
  - `normalized_score = 1.0 - ((raw_score - min_score) / (max_score - min_score))`
  - **Key Improvement**: Uses consistent normalization range across all candidates

**Normalization Process**:
1. **First Pass**: Collect raw scores from all fuzzy candidates
2. **Calculate Global Range**: Compute actual min/max across all candidates
3. **Second Pass**: Normalize all scores using the same global range
4. **Edge Case Handling**:
   - Identical scores → Use range [0.0, 1.0]
   - Very close scores → Expand range for better differentiation

**Filtering**: Smart length-based filtering to skip unpromising candidates:
- Query ≤2 chars: Skip if candidate > query × 8
- Query ≤3 chars: Skip if candidate > query × 5
- Query >3 chars: Skip if candidate > query × 4
- Always skip if candidate < query

### 2.3 Jaro-Winkler Similarity
**Algorithm Type**: `AlgorithmType::JaroWinkler`

**Scoring Logic**:
- **Raw Score**: Jaro-Winkler similarity (0.0-1.0)
- **Normalized Score**: Same as raw score
- **Adaptive Threshold**:
  - 0.6 for very short queries (1-2 characters)
  - 0.7 for longer queries (3+ characters)

### 2.4 Substring Matching
**Algorithm Type**: `AlgorithmType::Substring`

**Scoring Logic**:
- **Raw Score**: Position index of substring match
- **Normalized Score**: Position-based normalization (earlier matches are better)
  - `normalized_score = 1.0 - (position / max_position)`
  - Where `max_position = candidate_length - query_length`
- **Usage**: Only used for queries ≥2 characters

## 3. Dynamic Weighting System

### Query Length Categories

| Category | Query Length | Description |
|----------|--------------|-------------|
| VeryShort | 1-2 chars | Single characters or very short queries |
| Short | 3-4 chars | Short word fragments |
| Medium | 5-6 chars | Medium-length queries |
| Long | 7+ chars | Full words or longer queries |

### Algorithm Weights by Category

#### Very Short Queries (1-2 chars)
```
Prefix:      0.45 (45%)
FuzzySubseq: 0.35 (35%)
JaroWinkler: 0.15 (15%)
Substring:   0.05 (5%)
```

#### Short Queries (3-4 chars)
```
Prefix:      0.40 (40%)
FuzzySubseq: 0.30 (30%)
JaroWinkler: 0.20 (20%)
Substring:   0.10 (10%)
```

#### Medium Queries (5-6 chars)
```
Prefix:      0.35 (35%)
FuzzySubseq: 0.25 (25%)
JaroWinkler: 0.25 (25%)
Substring:   0.15 (15%)
```

#### Long Queries (7+ chars)
```
Prefix:      0.25 (25%)
FuzzySubseq: 0.20 (20%)
JaroWinkler: 0.35 (35%)
Substring:   0.20 (20%)
```

### Weighted Score Calculation

```rust
weighted_score =
    weights.prefix * prefix_score +
    weights.fuzzy_subseq * fuzzy_score +
    weights.jaro_winkler * jaro_score +
    weights.substring * substring_score
```

## 4. Metadata Integration

### Frequency Factor
- **Formula**: `frequency_factor = 1.0 + ln(frequency + 1) * 0.1`
- **Purpose**: Logarithmic scaling to prevent high-frequency items from dominating
- **Range**: 1.0 to ~1.7 for typical frequencies

### Age Factor
- **Formula**: `age_factor = 1.0 + (1.0 - (age_days / 365)) * 0.05`
- **Purpose**: Slight preference for newer items
- **Range**: 1.0 to 1.05
- **Maximum Age**: 365 days for normalization

### Length Penalty
- **Applied Only When**: `candidate_length > query_length * 3`
- **Formula**: `length_penalty = 1.0 - ((candidate_length - query_length) / max_length) * 0.1`
- **Purpose**: Penalize excessively long candidates for short queries
- **max_length**: Maximum string length in the database

## 5. Final Score Calculation

### Combined Formula
```
final_score = weighted_score * frequency_factor * age_factor * length_penalty
```

### Score Bounds
- **Minimum**: 0.0
- **Maximum**: 2.0 (capped to prevent extreme values)

## 6. Candidate Ranking

### Ranking Process
1. **Merge Duplicates**: Combine scores from different algorithms for the same string
2. **Calculate Final Scores**: Apply all weighting and metadata adjustments
3. **Sort Descending**: Higher scores appear first
4. **Apply Limit**: Return top N results

### Ranking Priority
1. **Final Score** (descending) - Primary ranking criterion
2. **Algorithm Scores** - Best available score from any algorithm
3. **Metadata** - Frequency, age, and length considerations

## 7. Special Cases

### Single Character Queries
- Use prefix search only
- Sort by frequency (descending)
- Skip expensive fuzzy matching
- Handled separately in `handle_single_character_query()`

### High-Quality Prefix Matches
- If ≥ limit of candidates are exact prefix matches, return them directly
- Avoids unnecessary scoring computation
- Uses `has_high_quality_prefix_matches()` check

### Empty Database
- Return empty vector immediately

### First Character Fallback
- Used as last resort when other algorithms don't produce enough results
- Searches by first character prefix only
- Ensures minimum result count even for difficult queries

## 8. Normalization Improvements

### Key Enhancement: Consistent Fuzzy Subsequence Normalization

**Problem Solved**: Previously, fuzzy subsequence scores were normalized using **inconsistent ranges**:
- Each candidate used its own length-based range: `[0.0, candidate_length × 2.0]`
- This made scores incomparable across candidates with different lengths
- Fuzzy matches were unfairly penalized in the dynamic weighting system

**Solution Implemented**: **Global Two-Pass Normalization**

```rust
// First pass: collect raw scores
for candidate in candidates {
    if let Some(raw_score) = calculate_fuzzy_subsequence_raw_score(candidate, query) {
        fuzzy_raw_scores.push(raw_score);
    }
}

// Calculate global min/max
let min_score = fuzzy_raw_scores.iter().fold(f64::MAX, |a, &b| a.min(b));
let max_score = fuzzy_raw_scores.iter().fold(f64::MIN, |a, &b| a.max(b));

// Second pass: normalize with consistent range
for candidate in candidates {
    let normalized_score = normalize_fuzzy_score(raw_score, min_score, max_score);
}
```

**Benefits**:
- **Comparable Scores**: All fuzzy scores use the same normalization range
- **Fair Weighting**: Fuzzy matches now compete fairly in dynamic weighting
- **Consistent Results**: Same query produces consistent rankings regardless of candidate mix

**Edge Case Handling**:
- **Identical Scores**: Use range `[0.0, 1.0]` for perfect differentiation
- **Very Close Scores**: Expand range to `[mid - 0.5, mid + 0.5]` for better separation
- **Single Candidate**: Use reasonable range around the single score

## 9. Performance Optimizations

### Smart Filtering
- Skip candidates with length mismatches
- Apply algorithm-specific thresholds
- Early termination when target count reached

### Two-Pass Optimization
- **Minimal Overhead**: Two passes over same candidate set
- **Memory Efficient**: Only store raw scores temporarily
- **Balanced Performance**: Better accuracy with minimal computation increase

### Memory Efficiency
- Progressive execution reduces memory usage
- Hash-based duplicate detection
- Minimal temporary storage

## 10. Mathematical Formulas Summary

### Algorithm Scores
- **Prefix**: `score = 1.0 if prefix_match else 0.0`
- **Fuzzy**: `raw_score = span_length + 0.1 * candidate_length`
- **Jaro-Winkler**: `score = jaro_winkler_similarity`
- **Substring**: `normalized_score = 1.0 - position/max_position`

### Normalization
- **Fuzzy**: `normalized = 1.0 - (raw - min)/(max - min)`
- **Substring**: `normalized = 1.0 - position/max_position`

### Weighted Combination
```
weighted_score = Σ(weight_i * score_i)
```

### Metadata Adjustments
```
final_score = weighted_score *
              (1.0 + ln(freq + 1) * 0.1) *
              (1.0 + (1.0 - age/365) * 0.05) *
              length_penalty
```

## 11. Example Scenarios

### Short Query: "hel"
- **Weights**: Prefix-heavy (40%)
- **Results**: "hello", "help", "helicopter"
- **Scoring**: Prefix matches dominate

### Abbreviation: "hl"
- **Weights**: Fuzzy-heavy (35%)
- **Results**: "hello", "help", "helicopter"
- **Scoring**: Fuzzy subsequence finds matches

### Typo: "wrold"
- **Weights**: Jaro-heavy (35%)
- **Results**: "world"
- **Scoring**: Jaro-Winkler corrects typo

This scoring system provides intelligent, context-aware search results by dynamically balancing multiple search algorithms based on query characteristics.

## 12. UTF-8 Character Handling

### Unicode Support
- Full Unicode character handling using `chars()` iterator
- Proper multi-byte UTF-8 sequence support (emoji, accented characters, etc.)
- Character-by-character matching for all algorithms
- Separate character-based implementations for fuzzy subsequence algorithms

### First Character Fallback
- Uses `query.chars().next()` for proper Unicode character extraction
- Converts first character to string for prefix search
- Ensures correct handling of multi-byte characters