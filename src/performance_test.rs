use std::time::Instant;
use crate::modules::string_space::StringSpace;

pub fn run_performance_tests() {
    println!("Running performance tests...\n");

    // Test 1: Small dataset - use simple prefix queries to avoid debug output
    let mut ss_small = StringSpace::new();
    for i in 0..100 {
        ss_small.insert_string(&format!("test{}", i), 1).unwrap();
    }
    ss_small.insert_string("apple", 5).unwrap();
    ss_small.insert_string("application", 3).unwrap();
    ss_small.insert_string("apply", 1).unwrap();

    let start = Instant::now();
    for _ in 0..100 {
        let _ = ss_small.best_completions("app", Some(10));
    }
    let duration = start.elapsed();
    println!("Small dataset (100 words): {:.2?} per query", duration / 100);

    // Test 2: Medium dataset
    let mut ss_medium = StringSpace::new();
    for i in 0..1000 {
        ss_medium.insert_string(&format!("testword{}", i), 1).unwrap();
    }
    ss_medium.insert_string("apple", 5).unwrap();
    ss_medium.insert_string("application", 3).unwrap();
    ss_medium.insert_string("apply", 1).unwrap();

    let start = Instant::now();
    for _ in 0..50 {
        let _ = ss_medium.best_completions("app", Some(10));
    }
    let duration = start.elapsed();
    println!("Medium dataset (1,000 words): {:.2?} per query", duration / 50);

    // Test 3: Large dataset
    let mut ss_large = StringSpace::new();
    for i in 0..10000 {
        ss_large.insert_string(&format!("largeword{}", i), 1).unwrap();
    }
    ss_large.insert_string("apple", 5).unwrap();
    ss_large.insert_string("application", 3).unwrap();
    ss_large.insert_string("apply", 1).unwrap();

    let start = Instant::now();
    for _ in 0..10 {
        let _ = ss_large.best_completions("app", Some(10));
    }
    let duration = start.elapsed();
    println!("Large dataset (10,000 words): {:.2?} per query", duration / 10);

    // Test 4: Progressive execution with different query types
    println!("\nProgressive execution performance:");

    let start = Instant::now();
    for _ in 0..50 {
        let _ = ss_medium.best_completions("app", Some(10));  // Prefix match
    }
    let duration = start.elapsed();
    println!("  Prefix query (app): {:.2?} per query", duration / 50);

    let start = Instant::now();
    for _ in 0..50 {
        let _ = ss_medium.best_completions("apl", Some(10));   // Fuzzy subsequence
    }
    let duration = start.elapsed();
    println!("  Fuzzy query (apl): {:.2?} per query", duration / 50);

    let start = Instant::now();
    for _ in 0..50 {
        let _ = ss_medium.best_completions("appl", Some(10)); // Jaro-Winkler
    }
    let duration = start.elapsed();
    println!("  Jaro query (appl): {:.2?} per query", duration / 50);

    // Test 5: Early termination effectiveness
    println!("\nEarly termination analysis:");
    let start = Instant::now();
    let results = ss_medium.best_completions("app", Some(5));
    let duration = start.elapsed();
    println!("  Early termination (limit 5): {:.2?}, found {} results", duration, results.len());

    let start = Instant::now();
    let results = ss_medium.best_completions("app", Some(20));
    let duration = start.elapsed();
    println!("  Full execution (limit 20): {:.2?}, found {} results", duration, results.len());

    println!("\nPerformance tests completed!");
}