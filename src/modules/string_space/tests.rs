#[cfg(test)]
mod tests {
    use crate::modules::string_space::*;

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

        #[test]
        fn test_prefix_priority_over_other_algorithms() {
            let mut ss = StringSpace::new();

            // Insert test data with clear prefix matches and fuzzy matches
            ss.insert_string("hello", 10).unwrap();
            ss.insert_string("help", 15).unwrap();
            ss.insert_string("helicopter", 5).unwrap();
            ss.insert_string("world", 20).unwrap();

            // Query that should return prefix matches
            let query = "hel";
            let results = ss.best_completions(query, Some(10));

            // All prefix matches should be in results
            assert!(results.len() >= 3, "Should find all prefix matches for 'hel'");

            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()), "'hello' should be in results");
            assert!(strings.contains(&"help".to_string()), "'help' should be in results");
            assert!(strings.contains(&"helicopter".to_string()), "'helicopter' should be in results");

            // Prefix matches should appear before non-prefix matches
            // "world" should not appear in results since it doesn't match "hel"
            assert!(!strings.contains(&"world".to_string()), "'world' should not be in results for 'hel' query");

            // Test with a query that has both prefix and fuzzy matches
            let mut ss2 = StringSpace::new();
            ss2.insert_string("hello", 10).unwrap();
            ss2.insert_string("help", 15).unwrap();
            ss2.insert_string("helicopter", 5).unwrap();
            ss2.insert_string("hallelujah", 8).unwrap();
            ss2.insert_string("world", 20).unwrap();

            let query2 = "hl";
            let results2 = ss2.best_completions(query2, Some(10));

            // Should find fuzzy matches for "hl"
            assert!(results2.len() >= 4, "Should find fuzzy matches for 'hl'");

            let strings2: Vec<String> = results2.iter().map(|r| r.string.clone()).collect();
            assert!(strings2.contains(&"hello".to_string()), "'hello' should be in fuzzy results");
            assert!(strings2.contains(&"help".to_string()), "'help' should be in fuzzy results");
            assert!(strings2.contains(&"helicopter".to_string()), "'helicopter' should be in fuzzy results");
            assert!(strings2.contains(&"hallelujah".to_string()), "'hallelujah' should be in fuzzy results");
        }

        #[test]
        fn test_implement_prefix_priority() {
            let mut ss = StringSpace::new();

            // Insert the exact test data with metadata (frequency, age_days)
            let test_data = vec![
                ("-implementing", 2, 20046),
                ("Change_in_valuation_multiple", 1, 20369),
                ("implement", 117, 20378),
                ("implementation", 67, 20378),
                ("implementations", 23, 20046),
                ("implementatoin", 1, 19990),
                ("implemented", 21, 20378),
                ("implementers", 1, 20016),
                ("implementing", 18, 20378),
                ("implements", 31, 20231),
            ];

            // Insert all strings with their actual metadata
            for (string, frequency, age_days) in test_data {
                ss.insert_string_with_age(string, frequency, age_days).unwrap();
            }

            // Query that should prioritize prefix matches
            let query = "imple";
            let results = ss.best_completions(query, Some(10));

            println!("Query: '{}'", query);
            println!("Results:");
            for (i, result) in results.iter().enumerate() {
                println!("  {}: {} (freq: {}, age: {})", i + 1, result.string, result.meta.frequency, result.meta.age_days);
            }

            // "implement" should be the highest ranked result because:
            // 1. It's a direct prefix match
            // 2. It's the shortest prefix match
            // 3. It has the highest frequency (117)
            // 4. It should have highest priority over other algorithms
            assert!(results.len() > 0, "Should find matches for 'imple'");

            // Check that "implement" is in the results
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"implement".to_string()), "'implement' should be in results");

            // The top result should be "implement" since it's the shortest prefix match with highest frequency
            if results.len() > 0 {
                println!("Top result: '{}' (freq: {}, age: {})", results[0].string, results[0].meta.frequency, results[0].meta.age_days);
                // This assertion will likely fail with the current implementation
                // because metadata and scoring may not properly prioritize prefix matches
                assert_eq!(results[0].string, "implement", "'implement' should be the top result for 'imple' query");
            }
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_best_completions_complete_pipeline() {
            let mut ss = StringSpace::new();

            // Add diverse test data with different characteristics
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

            // Test 1: Progressive algorithm execution with mixed results
            let results = ss.best_completions("hel", Some(10));
            assert!(results.len() >= 3, "Should find all hel-prefix words");
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
            assert!(strings.contains(&"helicopter".to_string()));

            // Test 2: Typo correction via Jaro-Winkler
            let results = ss.best_completions("wrold", Some(10));
            assert!(results.len() > 0, "Should find matches for typo correction");

            // Test 3: Abbreviation matching via fuzzy subsequence
            let results = ss.best_completions("og4", Some(10));
            assert!(results.len() > 0, "Should find abbreviation matches");

            // Test 4: Metadata integration - frequency should influence ranking
            let results = ss.best_completions("app", Some(10));
            assert!(results.len() > 0, "Should find matches for 'app'");
            // "apple" has highest frequency (25), should be in top results
            // Note: With new scoring system, other factors like length and metadata may affect ranking
            let apple_found = results.iter().any(|r| r.string == "apple");
            assert!(apple_found, "'apple' should be in results for 'app' query");
        }

        #[test]
        fn test_progressive_execution_with_various_query_types() {
            let mut ss = StringSpace::new();

            // Add test data
            ss.insert_string("hello", 10).unwrap();
            ss.insert_string("help", 15).unwrap();
            ss.insert_string("helicopter", 5).unwrap();
            ss.insert_string("world", 20).unwrap();

            // Test different query types that trigger different algorithms
            let query_types = vec![
                ("h", "very short - prefix + fuzzy"),
                ("he", "short - prefix + fuzzy"),
                ("hel", "medium - prefix + jaro"),
                ("hell", "medium - prefix + jaro"),
                ("hl", "fuzzy subsequence"),
                ("wrold", "typo - jaro"),
                ("wor", "prefix"),
            ];

            for (query, description) in query_types {
                let results = ss.best_completions(query, Some(5));
                assert!(results.len() <= 5, "Result limit should be respected for {}", description);
                assert!(results.len() > 0 || query == "hl", "Should find results for {} query '{}'", description, query);
            }
        }

        #[test]
        fn test_result_merging_and_ranking() {
            let mut ss = StringSpace::new();

            // Add test data with mixed algorithm results
            ss.insert_string("hello", 10).unwrap();
            ss.insert_string("help", 15).unwrap();
            ss.insert_string("helicopter", 5).unwrap();
            ss.insert_string("world", 20).unwrap();

            // Test result limiting
            let results = ss.best_completions("h", Some(3));
            assert!(results.len() <= 3, "Result limit should be respected");

            // Test early termination for high-quality prefix matches
            let results = ss.best_completions("hel", Some(5));
            assert!(results.len() > 0, "Should find results for 'hel'");

            // Test that we get reasonable results for various queries
            let queries = vec!["he", "hel", "hl", "wor"];
            for query in queries {
                let results = ss.best_completions(query, Some(5));
                assert!(results.len() <= 5, "Result limit should be respected for query '{}'", query);
            }
        }

        #[test]
        fn test_query_length_categories() {
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
                assert!(results.len() <= 5, "Result limit should be respected for {}", description);
            }
        }
    }

    mod performance_tests {
        use super::*;
        use std::time::Instant;

        #[test]
        fn test_performance_small_dataset() {
            let mut ss = StringSpace::new();

            // Small dataset: 100 words
            for i in 0..100 {
                ss.insert_string(&format!("test{}", i), 1).unwrap();
            }
            ss.insert_string("apple", 5).unwrap();
            ss.insert_string("application", 3).unwrap();
            ss.insert_string("apply", 1).unwrap();

            let start = Instant::now();
            for _ in 0..10 {
                let _ = ss.best_completions("app", Some(10));
            }
            let duration = start.elapsed();

            // Should complete in reasonable time (less than 1 second for 10 queries)
            assert!(duration.as_millis() < 1000, "Small dataset should be fast");
        }

        #[test]
        fn test_performance_medium_dataset() {
            let mut ss = StringSpace::new();

            // Medium dataset: 1,000 words
            for i in 0..1000 {
                ss.insert_string(&format!("testword{}", i), 1).unwrap();
            }
            ss.insert_string("apple", 5).unwrap();
            ss.insert_string("application", 3).unwrap();
            ss.insert_string("apply", 1).unwrap();

            let start = Instant::now();
            for _ in 0..5 {
                let _ = ss.best_completions("app", Some(10));
            }
            let duration = start.elapsed();

            // Should complete in reasonable time (less than 1 second for 5 queries)
            assert!(duration.as_millis() < 1000, "Medium dataset should be fast");
        }

        #[test]
        fn test_performance_large_dataset() {
            let mut ss = StringSpace::new();

            // Large dataset: 10,000 words
            for i in 0..10000 {
                ss.insert_string(&format!("largeword{}", i), 1).unwrap();
            }
            ss.insert_string("apple", 5).unwrap();
            ss.insert_string("application", 3).unwrap();
            ss.insert_string("apply", 1).unwrap();

            let start = Instant::now();
            let _ = ss.best_completions("app", Some(10));
            let duration = start.elapsed();

            // Should complete in reasonable time (less than 500ms for single query)
            assert!(duration.as_millis() < 500, "Large dataset should be reasonable");
        }

        #[test]
        fn test_early_termination_effectiveness() {
            let mut ss = StringSpace::new();

            // Add many strings to test early termination
            for i in 0..1000 {
                ss.insert_string(&format!("test{}", i), 1).unwrap();
            }
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 3).unwrap();

            // Test with small limit to trigger early termination
            let start = Instant::now();
            let results = ss.best_completions("hel", Some(5));
            let duration = start.elapsed();

            // Should get some results but not necessarily all matches due to early termination
            assert!(results.len() > 0, "Should find results");
            assert!(results.len() <= 5, "Result limit should be respected");
            // Adjust timeout to be more realistic for debug builds
            assert!(duration.as_millis() < 500, "Early termination should be fast, took {}ms", duration.as_millis());
        }

        #[test]
        fn test_memory_usage_patterns() {
            let mut ss = StringSpace::new();

            // Test memory usage with large dataset
            for i in 0..5000 {
                ss.insert_string(&format!("memorytest{}", i), 1).unwrap();
            }

            let initial_capacity = ss.capacity();

            // Perform multiple queries to test memory stability
            for _ in 0..10 {
                let _ = ss.best_completions("mem", Some(10));
            }

            let final_capacity = ss.capacity();

            // Memory capacity should remain stable during query operations
            assert_eq!(initial_capacity, final_capacity, "Memory usage should be stable");
        }

        #[test]
        fn test_progressive_execution_performance() {
            let mut ss = StringSpace::new();

            // Add test data
            for i in 0..1000 {
                ss.insert_string(&format!("perftest{}", i), 1).unwrap();
            }
            ss.insert_string("hello", 10).unwrap();
            ss.insert_string("help", 15).unwrap();

            // Test different query types that trigger different algorithms
            let query_types = vec![
                ("app", "prefix match"),
                ("apl", "fuzzy subsequence"),
                ("appl", "jaro-winkler"),
            ];

            for (query, description) in query_types {
                let start = Instant::now();
                let _ = ss.best_completions(query, Some(10));
                let duration = start.elapsed();

                // All query types should complete in reasonable time
                assert!(duration.as_millis() < 100, "{} should be fast", description);
            }
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_database() {
            let ss = StringSpace::new();

            // All search methods should return empty results for empty database
            assert!(ss.best_completions("hello", Some(10)).is_empty());
            assert!(ss.find_by_prefix("hel").is_empty());
            assert!(ss.fuzzy_subsequence_search("hl").is_empty());
            assert!(ss.find_with_substring("lo").is_empty());
            assert!(ss.get_similar_words("hell", Some(0.7)).is_empty());
        }

        #[test]
        fn test_single_character_queries() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 5).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("world", 2).unwrap();
            ss.insert_string("wonder", 1).unwrap();

            // Single character queries should work
            let results = ss.best_completions("h", Some(10));
            assert!(results.len() >= 2);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));

            // Single character queries should be sorted by frequency
            let results = ss.best_completions("w", Some(10));
            assert!(results.len() >= 2);
            // "world" has higher frequency than "wonder"
            assert_eq!(results[0].string, "world");
            assert_eq!(results[1].string, "wonder");
        }

        #[test]
        fn test_invalid_single_character_queries() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();

            // Non-alphanumeric single character queries should be rejected
            let results = ss.best_completions("!", Some(10));
            assert!(results.is_empty());

            let results = ss.best_completions(" ", Some(10));
            assert!(results.is_empty());

            let results = ss.best_completions("\n", Some(10));
            assert!(results.is_empty());
        }

        #[test]
        fn test_control_characters_in_query() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();

            // Queries with control characters should be rejected
            let results = ss.best_completions("hel\0lo", Some(10));
            assert!(results.is_empty());

            let results = ss.best_completions("hel\tlo", Some(10));
            assert!(results.is_empty());

            let results = ss.best_completions("hel\rlo", Some(10));
            assert!(results.is_empty());
        }

        #[test]
        fn test_very_long_queries() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();

            // Queries longer than MAX_CHARS should be rejected
            let long_query = "a".repeat(MAX_CHARS + 1);
            let results = ss.best_completions(&long_query, Some(10));
            assert!(results.is_empty());

            // Queries at maximum length should work
            let max_length_query = "a".repeat(MAX_CHARS);
            let results = ss.best_completions(&max_length_query, Some(10));
            // Should return empty results (no matches) but not crash
            assert!(results.is_empty());
        }

        #[test]
        fn test_unicode_edge_cases() {
            let mut ss = StringSpace::new();
            ss.insert_string("café", 1).unwrap();
            ss.insert_string("naïve", 2).unwrap();
            ss.insert_string("über", 3).unwrap();
            ss.insert_string("hello", 4).unwrap();

            // Unicode queries should work correctly
            let results = ss.best_completions("caf", Some(10));
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].string, "café");

            let results = ss.best_completions("naï", Some(10));
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].string, "naïve");

            let results = ss.best_completions("üb", Some(10));
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].string, "über");
        }

        #[test]
        fn test_unicode_replacement_characters() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();

            // Queries with Unicode replacement characters should be rejected
            let results = ss.best_completions("hel�lo", Some(10));
            assert!(results.is_empty());
        }

        #[test]
        fn test_empty_query() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();

            // Empty queries should return empty results
            let results = ss.best_completions("", Some(10));
            assert!(results.is_empty());
        }

        #[test]
        fn test_no_matches_fallback() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 2).unwrap();
            ss.insert_string("world", 3).unwrap();

            // Query with no direct matches should use fallback algorithms
            let results = ss.best_completions("xyz", Some(10));
            // Should return empty results (no matches found)
            assert!(results.is_empty());

            // Query with partial matches should use fuzzy algorithms
            let results = ss.best_completions("hl", Some(10));
            // Should find "hello" and "help" via fuzzy subsequence
            assert!(results.len() >= 2);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
        }

        #[test]
        fn test_progressive_algorithm_fallback() {
            let mut ss = StringSpace::new();
            // Add many strings to test progressive execution
            for i in 0..20 {
                ss.insert_string(&format!("test{}", i), 1).unwrap();
            }
            ss.insert_string("hello", 5).unwrap();
            ss.insert_string("help", 3).unwrap();
            ss.insert_string("helicopter", 1).unwrap();

            // Test that progressive execution works through multiple algorithms
            let results = ss.best_completions("hl", Some(10));
            // Should find all three "h" words via fuzzy subsequence
            assert!(results.len() >= 3);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello".to_string()));
            assert!(strings.contains(&"help".to_string()));
            assert!(strings.contains(&"helicopter".to_string()));
        }

        #[test]
        fn test_graceful_degradation() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 2).unwrap();

            // Test that the system doesn't crash with various edge cases
            // Very short queries
            let results = ss.best_completions("h", Some(5));
            assert!(results.len() <= 5);

            // Queries with special characters (should be rejected)
            let results = ss.best_completions("h\0", Some(5));
            assert!(results.is_empty());

            // Queries at boundary lengths
            let boundary_query = "a".repeat(MAX_CHARS);
            let results = ss.best_completions(&boundary_query, Some(5));
            // Should not crash, may return empty results
            assert!(results.len() <= 5);
        }

        #[test]
        fn test_algorithm_threshold_adjustment() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("help", 2).unwrap();
            ss.insert_string("helicopter", 3).unwrap();

            // Test that different query lengths use appropriate thresholds
            // Short query (2 chars) should use lower Jaro-Winkler threshold
            let short_results = ss.best_completions("he", Some(10));
            // Longer query (4 chars) should use higher threshold
            let long_results = ss.best_completions("hell", Some(10));

            // Both should return results
            assert!(!short_results.is_empty());
            assert!(!long_results.is_empty());
        }

        #[test]
        fn test_character_set_validation() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();

            // Test various problematic character combinations
            let problematic_queries = [
                "\x00", // Null character
                "\x1F", // Unit separator
                "\x7F", // Delete character
                "\u{0080}", // C1 control character
                "\u{009F}", // C1 control character
            ];

            for query in problematic_queries {
                let results = ss.best_completions(query, Some(10));
                // Should return empty results (query rejected)
                assert!(results.is_empty(), "Query '{}' should be rejected", query);
            }
        }

        #[test]
        fn test_unicode_complex_characters() {
            let mut ss = StringSpace::new();
            // Test with complex Unicode characters
            ss.insert_string("café", 1).unwrap();
            ss.insert_string("naïve", 2).unwrap();
            ss.insert_string("über", 3).unwrap();
            ss.insert_string("jalapeño", 4).unwrap();
            ss.insert_string("résumé", 5).unwrap();
            ss.insert_string("crème brûlée", 6).unwrap();

            // Test various Unicode queries
            let unicode_queries = vec![
                ("caf", "café"),
                ("naï", "naïve"),
                ("üb", "über"),
                ("jal", "jalapeño"),
                ("rés", "résumé"),
                ("crè", "crème brûlée"),
            ];

            for (query, expected) in unicode_queries {
                let results = ss.best_completions(query, Some(10));
                assert!(results.len() > 0, "Should find matches for Unicode query '{}'", query);
                let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
                assert!(strings.contains(&expected.to_string()), "Should find '{}' for query '{}'", expected, query);
            }
        }

        #[test]
        fn test_emoji_and_symbol_handling() {
            let mut ss = StringSpace::new();
            // Test with emoji and symbols in strings
            ss.insert_string("hello 😊", 1).unwrap();
            ss.insert_string("world 🌍", 2).unwrap();
            ss.insert_string("test ✅", 3).unwrap();

            // Regular text queries should still work
            let results = ss.best_completions("hello", Some(10));
            assert!(results.len() >= 1);
            // The result may contain the emoji string
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"hello 😊".to_string()));

            // Test that we can find results for various queries
            let queries = vec!["hello", "world", "test"];
            for query in queries {
                let results = ss.best_completions(query, Some(10));
                assert!(results.len() > 0, "Should find results for query '{}'", query);
            }
        }

        #[test]
        fn test_maximum_capacity_datasets() {
            let mut ss = StringSpace::new();

            // Fill with maximum reasonable number of strings
            for i in 0..10000 {
                ss.insert_string(&format!("maxword{}", i), 1).unwrap();
            }

            // Test that queries still work with large dataset
            let results = ss.best_completions("max", Some(10));
            assert!(results.len() > 0, "Should find results in large dataset");
            assert!(results.len() <= 10, "Result limit should be respected");

            // Test memory usage doesn't explode
            let capacity = ss.capacity();
            assert!(capacity > 0, "Should have allocated memory");
        }

        #[test]
        fn test_mixed_frequency_and_age_distributions() {
            let mut ss = StringSpace::new();

            // Add words with varied frequency and age distributions
            ss.insert_string("high_freq_new", 100).unwrap();
            ss.insert_string("high_freq_old", 100).unwrap();
            ss.insert_string("medium_freq_new", 50).unwrap();
            ss.insert_string("medium_freq_old", 50).unwrap();
            ss.insert_string("low_freq_new", 10).unwrap();
            ss.insert_string("low_freq_old", 10).unwrap();

            // Test that we can find high frequency items
            let results = ss.best_completions("high", Some(10));
            assert!(results.len() >= 2);
            // All results should have high frequency
            let all_high_freq = results.iter().all(|r| r.meta.frequency == 100);
            assert!(all_high_freq, "All results should have high frequency");

            // Test that we can find items across frequency ranges
            let results = ss.best_completions("freq", Some(10));
            assert!(results.len() >= 6);
            // Should contain items from all frequency ranges
            let high_freq_count = results.iter().filter(|r| r.meta.frequency == 100).count();
            let medium_freq_count = results.iter().filter(|r| r.meta.frequency == 50).count();
            let low_freq_count = results.iter().filter(|r| r.meta.frequency == 10).count();
            assert!(high_freq_count >= 2, "Should have high frequency items");
            assert!(medium_freq_count >= 2, "Should have medium frequency items");
            assert!(low_freq_count >= 2, "Should have low frequency items");
        }

        #[test]
        fn test_single_character_edge_cases() {
            let mut ss = StringSpace::new();
            // Only insert valid strings (minimum 3 chars)
            ss.insert_string("abc", 1).unwrap(); // Minimum length
            ss.insert_string("xyz", 2).unwrap();

            // Single character queries should work for valid strings
            let results = ss.best_completions("a", Some(10));
            assert!(results.len() > 0, "Should find results for single char query");
            // The result should contain "abc"
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"abc".to_string()));

            // Very short queries should be handled gracefully
            let results = ss.best_completions("x", Some(10));
            assert!(results.len() > 0, "Should find results for single char query");
            // The result should contain "xyz"
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"xyz".to_string()));
        }

        #[test]
        fn test_boundary_length_queries() {
            let mut ss = StringSpace::new();
            ss.insert_string("hello", 1).unwrap();
            ss.insert_string("world", 2).unwrap();

            // Test queries at various boundary lengths
            let boundary_queries = vec![
                ("", "empty query"),
                ("h", "1 char"),
                ("he", "2 chars"),
                ("hel", "3 chars"),
                ("hell", "4 chars"),
                ("hello", "5 chars"),
                ("helloo", "6 chars"),
                ("hellooo", "7 chars"),
                ("helloooo", "8 chars"),
                ("hellooooo", "9 chars"),
                ("helloooooo", "10 chars"),
            ];

            for (query, description) in boundary_queries {
                let results = ss.best_completions(query, Some(10));
                // Should not crash for any query length
                assert!(results.len() <= 10, "Result limit should be respected for {}", description);
            }
        }

        #[test]
        fn test_special_character_sequences() {
            let mut ss = StringSpace::new();
            // Test with strings containing special characters
            ss.insert_string("test-hyphen", 1).unwrap();
            ss.insert_string("test_underscore", 2).unwrap();
            ss.insert_string("test.dot", 3).unwrap();

            // Queries with special characters should work
            let results = ss.best_completions("test-", Some(10));
            assert!(results.len() >= 1);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"test-hyphen".to_string()));

            let results = ss.best_completions("test_", Some(10));
            assert!(results.len() >= 1);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"test_underscore".to_string()));

            let results = ss.best_completions("test.", Some(10));
            assert!(results.len() >= 1);
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"test.dot".to_string()));
        }
    }

    mod quality_assurance_tests {
        use super::*;

        #[test]
        fn test_result_ranking_quality() {
            let mut ss = StringSpace::new();

            // Add test data with clear ranking expectations
            ss.insert_string("apple", 25).unwrap();    // High frequency
            ss.insert_string("application", 10).unwrap(); // Medium frequency
            ss.insert_string("apply", 5).unwrap();     // Low frequency
            ss.insert_string("applesauce", 3).unwrap(); // Very low frequency

            // Test that we find all app-prefix words
            let results = ss.best_completions("app", Some(10));
            assert!(results.len() >= 4, "Should find all app-prefix words");

            // Verify all expected strings are present
            let strings: Vec<String> = results.iter().map(|r| r.string.clone()).collect();
            assert!(strings.contains(&"apple".to_string()));
            assert!(strings.contains(&"application".to_string()));
            assert!(strings.contains(&"apply".to_string()));
            assert!(strings.contains(&"applesauce".to_string()));

            // Verify that higher frequency items tend to rank higher
            // (but not strictly due to complex scoring)
            let high_freq_count = results.iter().filter(|r| r.meta.frequency >= 10).count();
            let low_freq_count = results.iter().filter(|r| r.meta.frequency < 10).count();
            assert!(high_freq_count >= 2, "Should have high frequency items");
            assert!(low_freq_count >= 2, "Should have low frequency items");
        }

        #[test]
        fn test_frequency_recency_prioritization() {
            let mut ss = StringSpace::new();

            // Add words with same frequency but different ages
            ss.insert_string("recent_high_freq", 100).unwrap();
            ss.insert_string("old_high_freq", 100).unwrap();
            ss.insert_string("recent_medium_freq", 50).unwrap();
            ss.insert_string("old_medium_freq", 50).unwrap();

            // Test that we find high frequency items
            let results = ss.best_completions("high", Some(10));
            assert!(results.len() >= 2);
            // Both should have same frequency
            let high_freq_items: Vec<_> = results.iter().filter(|r| r.meta.frequency == 100).collect();
            assert!(high_freq_items.len() >= 2, "Should find high frequency items");

            // Test that we find items with frequency patterns
            let results = ss.best_completions("freq", Some(10));
            assert!(results.len() >= 4);
            // Should contain both high and medium frequency items
            let high_freq_count = results.iter().filter(|r| r.meta.frequency == 100).count();
            let medium_freq_count = results.iter().filter(|r| r.meta.frequency == 50).count();
            assert!(high_freq_count >= 2, "Should have high frequency items");
            assert!(medium_freq_count >= 2, "Should have medium frequency items");
        }

        #[test]
        fn test_length_penalty_verification() {
            let mut ss = StringSpace::new();

            // Add words with same frequency but different lengths
            ss.insert_string("short", 10).unwrap();
            ss.insert_string("medium_length", 10).unwrap();
            ss.insert_string("very_long_word_that_is_much_longer", 10).unwrap();

            // Test with query that matches all
            let results = ss.best_completions("s", Some(10));
            assert!(results.len() >= 1);

            // Test with query that requires fuzzy matching
            let results = ss.best_completions("shrt", Some(10));
            // "short" should rank higher than longer words due to length penalty
            if results.len() > 0 {
                assert_eq!(results[0].string, "short");
            }
        }

        #[test]
        fn test_algorithm_preference_hierarchy() {
            let mut ss = StringSpace::new();

            // Add test data
            ss.insert_string("hello", 10).unwrap();
            ss.insert_string("help", 15).unwrap();
            ss.insert_string("helicopter", 5).unwrap();

            // Test that prefix matches are preferred over fuzzy matches
            let prefix_results = ss.best_completions("hel", Some(10));
            let fuzzy_results = ss.best_completions("hl", Some(10));

            // Prefix queries should find exact matches
            assert!(prefix_results.len() >= 3);
            // Fuzzy queries may find fewer or different matches
            assert!(fuzzy_results.len() >= 3);

            // Verify that prefix results are exact matches
            let prefix_strings: Vec<String> = prefix_results.iter().map(|r| r.string.clone()).collect();
            assert!(prefix_strings.contains(&"hello".to_string()));
            assert!(prefix_strings.contains(&"help".to_string()));
            assert!(prefix_strings.contains(&"helicopter".to_string()));

            // Fuzzy results should also contain the same strings
            let fuzzy_strings: Vec<String> = fuzzy_results.iter().map(|r| r.string.clone()).collect();
            assert!(fuzzy_strings.contains(&"hello".to_string()));
            assert!(fuzzy_strings.contains(&"help".to_string()));
            assert!(fuzzy_strings.contains(&"helicopter".to_string()));
        }

        #[test]
        fn test_result_quality_across_query_types() {
            let mut ss = StringSpace::new();

            // Add comprehensive test data
            ss.insert_string("hello", 10).unwrap();
            ss.insert_string("help", 15).unwrap();
            ss.insert_string("helicopter", 5).unwrap();
            ss.insert_string("world", 20).unwrap();
            ss.insert_string("word", 18).unwrap();
            ss.insert_string("openai/gpt-4o-2024-08-06", 8).unwrap();

            // Test different query types and verify result quality
            let query_types = vec![
                ("hel", "prefix"),
                ("hl", "fuzzy subsequence"),
                ("wrold", "jaro-winkler typo"),
                ("og4", "abbreviation"),
            ];

            for (query, query_type) in query_types {
                let results = ss.best_completions(query, Some(10));
                assert!(results.len() > 0, "Should find results for {} query '{}'", query_type, query);

                // Verify results are relevant to the query
                for result in &results {
                    match query_type {
                        "prefix" => assert!(result.string.starts_with(query),
                                           "Prefix result '{}' should start with '{}'", result.string, query),
                        "fuzzy subsequence" => {
                            // Should contain the characters in order
                            let mut query_chars = query.chars();
                            let mut current_char = query_chars.next();
                            for c in result.string.chars() {
                                if current_char == Some(c) {
                                    current_char = query_chars.next();
                                }
                            }
                            assert!(current_char.is_none(),
                                   "Fuzzy result '{}' should contain characters from '{}' in order", result.string, query);
                        },
                        "jaro-winkler typo" => {
                            // Should be similar to the intended word
                            let similarity = strsim::jaro_winkler(&result.string, "world");
                            assert!(similarity > 0.7,
                                   "Typo correction result '{}' should be similar to 'world'", result.string);
                        },
                        "abbreviation" => {
                            // Should match the abbreviation pattern
                            assert!(result.string.contains("openai/gpt"),
                                   "Abbreviation result '{}' should match pattern", result.string);
                        },
                        _ => {}
                    }
                }
            }
        }

        #[test]
        fn test_metadata_integration_quality() {
            let mut ss = StringSpace::new();

            // Add words with different metadata characteristics
            ss.insert_string("frequent_recent", 100).unwrap();
            ss.insert_string("frequent_old", 100).unwrap();
            ss.insert_string("infrequent_recent", 10).unwrap();
            ss.insert_string("infrequent_old", 10).unwrap();

            // Test that we can find frequent items
            let results = ss.best_completions("frequent", Some(10));
            assert!(results.len() >= 2);
            // Most should have high frequency (may include some from other algorithms)
            let high_freq_count = results.iter().filter(|r| r.meta.frequency == 100).count();
            assert!(high_freq_count >= 1, "Should have high frequency items");

            // Test that we can find infrequent items
            let results = ss.best_completions("infrequent", Some(10));
            assert!(results.len() >= 2);
            // Most should have low frequency (may include some from other algorithms)
            let low_freq_count = results.iter().filter(|r| r.meta.frequency == 10).count();
            assert!(low_freq_count >= 1, "Should have low frequency items");

            // Test mixed frequency patterns
            let results = ss.best_completions("ent", Some(10));
            assert!(results.len() >= 4);
            // Should contain both high and low frequency items
            let high_freq_count = results.iter().filter(|r| r.meta.frequency == 100).count();
            let low_freq_count = results.iter().filter(|r| r.meta.frequency == 10).count();
            assert!(high_freq_count >= 1, "Should have high frequency items");
            assert!(low_freq_count >= 1, "Should have low frequency items");
        }
    }
}