use std::fs;
use std::io::{self};
use crate::modules::utils::generate_random_words;
use crate::modules::utils::time_execution;
use crate::modules::string_space::StringSpace;
use crate::modules::string_space::StringRef;

#[allow(unused)]
pub fn benchmark(args: Vec<String>) {
    if args.len() != 2 {
        eprintln!("Usage: {} -b <num>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[0];
    let num_words: usize = args[1].parse().expect("Invalid number of words");

    // TRUNCATE FILE IF IT EXISTS
    if let Err(e) = fs::remove_file(file_path) {
        if e.kind() != io::ErrorKind::NotFound {
            panic!("Error deleting file: {}", e);
        }
    }
    let _ = fs::File::create(file_path);

    let mut space = StringSpace::new();

    // Insert strings
    space.insert_string("hello", 1).unwrap();
    space.insert_string("helicopter", 1).unwrap();
    space.insert_string("helicopter", 1).unwrap();
    space.insert_string("helicopter", 1).unwrap();
    space.insert_string("helicopter", 1).unwrap();
    space.insert_string("help", 1).unwrap();
    space.insert_string("harmony", 1).unwrap();
    space.insert_string("hero", 1).unwrap();
    space.insert_string("rust", 1).unwrap();

    let mut random_words = generate_random_words(num_words, 15, 26);
    random_words.sort();
    random_words.reverse();

    let insert_time = time_execution(|| {
        // Insert strings
        for word in random_words.iter() {
            space.insert_string(word, 1).unwrap();
        }
    });
    println!("Inserting {} words took {:?}", num_words, insert_time);

    let insert_time = time_execution(|| {
        // Insert strings
        for word in random_words.iter() {
            space.insert_string(word, 1).unwrap();
        }
    });
    println!("Inserting {} words again took {:?}", num_words, insert_time);

    // Write strings to file
    let write_time = time_execution(|| {
        space.write_to_file(file_path).unwrap();
    });
    println!("Writing strings to file took {:?}", write_time);

    // Read strings from file
    space.clear_space();
    let read_time = time_execution(|| {
        space.read_from_file(file_path).unwrap();
    });
    println!("Reading strings from file took {:?}", read_time);

    // space.print_strings();

    let substring = "he";
    // Search by prefix
    let mut found_strings: Vec<StringRef> = Vec::new();
    let find_time = time_execution(|| {
        found_strings = space.find_by_prefix(substring);
        println!("Found {} strings with prefix '{}':", found_strings.len(), substring);
        let max_len = std::cmp::min(found_strings.len(), 5);
        for string_ref in found_strings[0..max_len].iter() {
            println!("  {} {}", string_ref.string, string_ref.meta.frequency);
        }
    });
    found_strings.sort_by(|a, b| a.meta.frequency.cmp(&b.meta.frequency));
    println!("Finding strings with prefix '{}' took {:?}", substring, find_time);

    // Search by substring
    let mut found_strings: Vec<StringRef> = Vec::new();

    let find_time = time_execution(|| {
        found_strings = space.find_with_substring(substring);
        println!("Found {} strings with substring '{}':", found_strings.len(), substring);
        let max_len = std::cmp::min(found_strings.len(), 5);
        for string_ref in found_strings[0..max_len].iter() {
            println!("  {} {}", string_ref.string, string_ref.meta.frequency);
        }
    });
    found_strings.sort_by(|a, b| a.meta.frequency.cmp(&b.meta.frequency));
    println!("Finding strings with substring '{}' took {:?}", substring, find_time);

    // Search by fuzzy-subsequence
    let mut found_strings: Vec<StringRef> = Vec::new();
    let find_time = time_execution(|| {
        found_strings = space.fuzzy_subsequence_search(substring);
        println!("Found {} strings with fuzzy-subsequence '{}':", found_strings.len(), substring);
        let max_len = std::cmp::min(found_strings.len(), 5);
        for string_ref in found_strings[0..max_len].iter() {
            println!("  {} {}", string_ref.string, string_ref.meta.frequency);
        }
    });
    println!("Finding strings with fuzzy-subsequence '{}' took {:?}", substring, find_time);

    // Additional test queries for comprehensive benchmarking
    let test_queries = vec!["he", "lo", "wor", "hl", "elp", "rld"];
    for query in test_queries {
        let mut found_strings: Vec<StringRef> = Vec::new();
        let find_time = time_execution(|| {
            found_strings = space.fuzzy_subsequence_search(query);
        });
        println!("Fuzzy-subsequence search for '{}' found {} strings in {:?}", query, found_strings.len(), find_time);
    }

    let insert_time = time_execution(|| {
        // Insert strings
        space.insert_string("aaaaaaaaaaaaaaaa", 1).unwrap();
        space.insert_string("aaaaaaaaaaaaaaa", 1).unwrap();
        space.insert_string("aaaaaaaaaaaaaa", 1).unwrap();
        space.insert_string("aaaaaaaaaaaaa", 1).unwrap();
        space.insert_string("aaaaaaaaaaaa", 1).unwrap();
    });
    println!("Inserting first word 5 times for {} word list took {:?}", num_words, insert_time);

    space.write_to_file(file_path).unwrap();

}

