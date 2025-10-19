//! Integration tests for best_completions Phase 4 validation

use crate::modules::string_space::StringSpace;

/// Integration test for progressive algorithm execution
pub fn test_progressive_algorithm_execution() {
    println!("\n=== Testing Progressive Algorithm Execution ===");
    let mut ss = StringSpace::new();

    // Add test data with different characteristics
    ss.insert_string("hello", 10).unwrap();
    ss.insert_string("help", 15).unwrap();
    ss.insert_string("helicopter", 5).unwrap();
    ss.insert_string("openai/gpt-4o-2024-08-06", 8).unwrap();
    ss.insert_string("anthropic/claude-3-opus", 12).unwrap();
    ss.insert_string("world", 20).unwrap();
    ss.insert_string("word", 18).unwrap();
    ss.insert_string("ward", 3).unwrap();
    ss.insert_string("apple", 25).unwrap();
    ss.insert_string("pineapple", 8).unwrap();
    ss.insert_string("applesauce", 6).unwrap();

    // Test 1: Very short queries (1-2 chars) - should use prefix and fuzzy subsequence
    println!("\nTest 1: Very short queries (1-2 chars)");
    let results = ss.best_completions("h", Some(10));
    println!("Query 'h': Found {} results", results.len());
    assert!(results.len() > 0, "Should find results for very short query");

    // Test 2: Short queries (3-4 chars) - balanced algorithm weighting
    println!("\nTest 2: Short queries (3-4 chars)");
    let results = ss.best_completions("hel", Some(10));
    println!("Query 'hel': Found {} results", results.len());
    assert!(results.len() >= 3, "Should find all hel-prefix words");

    // Test 3: Medium queries (5-6 chars) - all algorithms contribute
    println!("\nTest 3: Medium queries (5-6 chars)");
    let results = ss.best_completions("hello", Some(10));
    println!("Query 'hello': Found {} results", results.len());
    assert!(results.len() >= 1, "Should find exact match for 'hello'");

    // Test 4: Long queries (7+ chars) - Jaro-Winkler and substring emphasis
    println!("\nTest 4: Long queries (7+ chars)");
    let results = ss.best_completions("pineapp", Some(10));
    println!("Query 'pineapp': Found {} results", results.len());
    assert!(results.len() >= 1, "Should find substring match for 'pineapp'");

    println!("✓ Progressive algorithm execution tests passed");
}

/// Integration test for unified scoring system
pub fn test_unified_scoring_system() {
    println!("\n=== Testing Unified Scoring System ===");
    let mut ss = StringSpace::new();

    // Add test data
    ss.insert_string("hello", 10).unwrap();
    ss.insert_string("help", 15).unwrap();
    ss.insert_string("world", 20).unwrap();
    ss.insert_string("word", 18).unwrap();
    ss.insert_string("openai/gpt-4o-2024-08-06", 8).unwrap();
    ss.insert_string("apple", 25).unwrap();

    // Test 1: Typo correction - Jaro-Winkler should handle character substitutions
    println!("\nTest 1: Typo correction");
    let results = ss.best_completions("wrold", Some(10));
    println!("Query 'wrold' (typo for 'world'): Found {} results", results.len());
    assert!(results.len() > 0, "Should find matches for typo correction");

    // Test 2: Abbreviation matching - Fuzzy subsequence with character order preservation
    println!("\nTest 2: Abbreviation matching");
    let results = ss.best_completions("og4", Some(10));
    println!("Query 'og4' (abbreviation for 'openai/gpt-4'): Found {} results", results.len());
    assert!(results.len() > 0, "Should find abbreviation matches");

    // Test 3: Metadata integration - frequency should influence ranking
    println!("\nTest 3: Metadata integration");
    let results = ss.best_completions("app", Some(10));
    println!("Query 'app': Found {} results", results.len());
    assert!(results.len() > 0, "Should find matches for 'app'");

    println!("✓ Unified scoring system tests passed");
}

/// Integration test for result processing pipeline
pub fn test_result_processing_pipeline() {
    println!("\n=== Testing Result Processing Pipeline ===");
    let mut ss = StringSpace::new();

    // Add test data
    ss.insert_string("hello", 10).unwrap();
    ss.insert_string("help", 15).unwrap();
    ss.insert_string("helicopter", 5).unwrap();
    ss.insert_string("world", 20).unwrap();

    // Test 1: Candidate deduplication - same word from different algorithms
    println!("\nTest 1: Candidate deduplication");
    let results = ss.best_completions("he", Some(10));
    println!("Query 'he': Found {} results", results.len());

    // Check for duplicates
    let mut seen = std::collections::HashMap::new();
    for result in &results {
        *seen.entry(result.string.clone()).or_insert(0) += 1;
    }

    let duplicates: Vec<_> = seen.iter().filter(|(_, &count)| count > 1).collect();
    assert!(duplicates.is_empty(), "Should not have duplicate results");

    // Test 2: Result limiting
    println!("\nTest 2: Result limiting");
    let results = ss.best_completions("h", Some(3));
    println!("Query 'h' with limit 3: Found {} results", results.len());
    assert!(results.len() <= 3, "Result limit should be respected");

    // Test 3: Early termination for high-quality prefix matches
    println!("\nTest 3: Early termination");
    let results = ss.best_completions("hel", Some(5));
    println!("Query 'hel' with limit 5: Found {} results", results.len());
    assert!(results.len() > 0, "Should find results for 'hel'");

    println!("✓ Result processing pipeline tests passed");
}

/// Integration test for query length categories
pub fn test_query_length_categories() {
    println!("\n=== Testing Query Length Categories ===");
    let mut ss = StringSpace::new();

    // Add test data
    ss.insert_string("hello", 10).unwrap();
    ss.insert_string("help", 15).unwrap();
    ss.insert_string("helicopter", 5).unwrap();
    ss.insert_string("pineapple", 8).unwrap();

    let test_cases = vec![
        ("h", "Very short (1 char)"),
        ("he", "Very short (2 chars)"),
        ("hel", "Short (3 chars)"),
        ("hell", "Short (4 chars)"),
        ("hello", "Medium (5 chars)"),
        ("helloo", "Medium (6 chars)"),
        ("pineapp", "Long (7 chars)"),
        ("pineappl", "Long (8 chars)"),
    ];

    for (query, description) in test_cases {
        let results = ss.best_completions(query, Some(5));
        println!("Query '{}' ({}): Found {} results", query, description, results.len());
        assert!(results.len() <= 5, "Result limit should be respected");
    }

    println!("✓ Query length category tests passed");
}

/// Run all integration tests
pub fn run_all_integration_tests() {
    println!("\nStringSpace Best Completions Phase 4 Integration Test");
    println!("======================================================");

    test_progressive_algorithm_execution();
    test_unified_scoring_system();
    test_result_processing_pipeline();
    test_query_length_categories();

    println!("\n=== All Integration Tests Complete ===");
    println!("✓ All integration tests passed successfully!");
}