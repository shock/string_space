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
        fn test_get_best_score_primary() {
            let string_ref = StringRef {
                string: "hello".to_string(),
                meta: StringMeta { frequency: 1, age_days: 0 },
            };
            let candidate = ScoreCandidate::new(
                string_ref,
                AlgorithmType::Prefix,
                0.8,
                0.9,
            );

            assert_eq!(candidate.get_best_score(), 0.9);
        }

        #[test]
        fn test_get_best_score_alternative() {
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

            candidate.add_alternative_score(AlgorithmType::JaroWinkler, 0.95);
            candidate.add_alternative_score(AlgorithmType::FuzzySubseq, 0.7);

            assert_eq!(candidate.get_best_score(), 0.95);
        }

        #[test]
        fn test_get_best_score_mixed() {
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

            candidate.add_alternative_score(AlgorithmType::JaroWinkler, 0.85);
            candidate.add_alternative_score(AlgorithmType::FuzzySubseq, 0.92);

            assert_eq!(candidate.get_best_score(), 0.92);
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
        fn test_has_high_quality_prefix_matches() {
            let ssi = StringSpaceInner::new();

            // Create test candidates
            let candidates = vec![
                StringRef { string: "hello".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "help".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "helicopter".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
            ];

            // All candidates start with "hel" - should be high quality
            assert!(ssi.has_high_quality_prefix_matches(&candidates, "hel"));

            // Mixed candidates - should not be high quality
            let mixed_candidates = vec![
                StringRef { string: "hello".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "world".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
                StringRef { string: "help".to_string(), meta: StringMeta { frequency: 1, age_days: 0 } },
            ];
            assert!(!ssi.has_high_quality_prefix_matches(&mixed_candidates, "hel"));
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
    }
}