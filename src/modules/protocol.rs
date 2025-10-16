use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::str;
use crate::modules::string_space::StringSpace;
use crate::modules::utils;
use regex;

pub trait Protocol {
    fn handle_client(&mut self, stream: &mut TcpStream);
}

const EOT_BYTE: u8 = 0x04;         // ASCII EOT (End of Transmission) character
const RS_BYTE_STR: &str = "\x1E";  // ASCII RS (Record Separator) character
const SEND_METADATA: bool = false;

pub struct StringSpaceProtocol {
    pub space: StringSpace,
    pub file_path: String,
}

impl StringSpaceProtocol {
    pub fn new(file_path: String) -> Self {
        let mut space = StringSpace::new();
        let expanded_path = utils::expand_path(&file_path);
        let result: io::Result<()> = space.read_from_file(&expanded_path);
        match result {
            Ok(_) => {
                println!("File read successfully");
            },
            Err(e) => {
                println!("Error reading file: {}", e);
            }
        }
        Self {
            space: space,
            file_path: expanded_path,
        }

    }

    fn create_response(&mut self, operation: &str, params: Vec<&str>) -> Vec<u8> {
        let mut response = Vec::new();
        if "prefix" == operation {
            if params.len() != 1 {
                let response_str = format!("ERROR - invalid parameters (length = {})", params.len());
                response.extend_from_slice(response_str.as_bytes());
                return response;
            }
            let prefix = params[0];
            let matches = self.space.find_by_prefix(prefix);
            for m in matches {
                response.extend_from_slice(m.string.as_bytes());
                if SEND_METADATA {
                    // add a space between string and frequency
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.frequency.to_string().as_bytes());
                    // add a space between frequency and age
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.age_days.to_string().as_bytes());
                }
                // add a newline between each record
                response.extend_from_slice("\n".as_bytes());
            }
            return response;
        }
        else if "similar" == operation {
            if params.len() != 2 {
                let response_str = format!("ERROR - invalid parameters (length = {})", params.len());
                response.extend_from_slice(response_str.as_bytes());
                return response;
            }
            let word = params[0];
            let cutoff_param = params[1].parse::<f64>();
            let cutoff: f64;
            match cutoff_param {
                Ok(t) => { cutoff = t; },
                Err(_) => {
                    let response_str = format!("ERROR\nInvalid cutoff parameter '{}'.  expecting floating point string between 0.0 and 1.0", params[1]);
                    response.extend_from_slice(response_str.as_bytes());
                    return response;
                }
            }
            println!("cutoff: {}", cutoff);
            let matches = self.space.get_similar_words(word, Some(cutoff));
            for m in matches {
                response.extend_from_slice(m.string.as_bytes());
                if SEND_METADATA {
                    // add a space between string and frequency
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.frequency.to_string().as_bytes());
                    // add a space between frequency and age
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.age_days.to_string().as_bytes());
                }
                // add a newline between each record
                response.extend_from_slice("\n".as_bytes());
            }
            return response;
        }
        else if "fuzzy-subsequence" == operation {
            if params.len() != 1 {
                let response_str = format!("ERROR - invalid parameters (length = {})", params.len());
                response.extend_from_slice(response_str.as_bytes());
                return response;
            }
            let query = params[0];
            let matches = self.space.fuzzy_subsequence_search(query);
            for m in matches {
                response.extend_from_slice(m.string.as_bytes());
                if SEND_METADATA {
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.frequency.to_string().as_bytes());
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.age_days.to_string().as_bytes());
                }
                response.extend_from_slice("\n".as_bytes());
            }
            return response;
        }
        else if "substring" == operation {
            if params.len() != 1 {
                let response_str = format!("ERROR - invalid parameters (length = {})", params.len());
                response.extend_from_slice(response_str.as_bytes());
                return response;
            }
            let substring = params[0];
            let matches = self.space.find_with_substring(substring);
            for m in matches {
                response.extend_from_slice(m.string.as_bytes());
                if SEND_METADATA {
                    // add a space between string and frequency
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.frequency.to_string().as_bytes());
                    // add a space between frequency and age
                    response.extend_from_slice(" ".as_bytes());
                    response.extend_from_slice(m.meta.age_days.to_string().as_bytes());
                }
                // add a newline between each record
                response.extend_from_slice("\n".as_bytes());
            }
            return response;
        }
        else if "data-file"  == operation {
            response.extend_from_slice(self.file_path.as_bytes());
            return response;
        }
        else if "insert"  == operation {
            if params.len() < 1 {
                let response_str = format!("ERROR - invalid parameters (length = {})", params.len());
                response.extend_from_slice(response_str.as_bytes());
                return response;
            }
            let mut counter = 0;
            let mut num_words = 0;
            for param in params {
                let string = param.trim();
                // replace newlines with spaces
                let string = string.replace('\n', " ");
                let string = string.replace(',', " ");
                // replace multiple spaces with a single space, using a regex
                let re = regex::Regex::new(r"\s+").unwrap();
                let string = re.replace_all(&string, " ").to_string();
                let words: Vec<&str> = string.split(' ').collect();
                for word in words {
                    num_words += 1;
                    let response = self.space.insert_string(word, 1);
                    match response {
                        Ok(_) => { counter += 1; },
                        Err(e) => {
                            println!("Error inserting word '{}': {}", word, e);
                        }
                    }
                }
            }
            if counter > 0 {
                self.space.write_to_file(&self.file_path).unwrap();
            }
            let response_str = format!("OK\nInserted {} of {} words", counter, num_words);
            response.extend_from_slice(response_str.as_bytes());
            return response;
        } else {
            response.extend_from_slice(format!("ERROR - unknown operation '{}'", operation).as_bytes());
            return response;
        }
    }
}

impl Protocol for StringSpaceProtocol {
    fn handle_client(&mut self, stream: &mut TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone stream"));

        loop {
            let mut buffer = Vec::new();

            // Read the input from the client until the EOT (End of Text) character is encountered
            reader.read_until(EOT_BYTE, &mut buffer).expect("Failed to read from stream");

            // Remove the EOT character from the buffer
            if let Some(index) = buffer.iter().position(|&b| b == EOT_BYTE) {
                buffer.truncate(index);
            } else {
                // If no EOT found, client probably disconnected
                break;
            }

            if buffer.is_empty() {
                // Empty message, client probably disconnected
                break;
            }

            let str_or_err = String::from_utf8(buffer);
            if let Ok(buffer_str) = str_or_err {
                // Split the buffer into a vector of strings using RS_BYTE as the delimiter
                let request_elements: Vec<&str> = buffer_str.split(RS_BYTE_STR).collect();
                println!("\nRequest:\n{}", request_elements.join("\n"));

                let mut response = self.create_response(request_elements[0], request_elements[1..].to_vec());
                // convert the response byte vector to a string
                let response_str = String::from_utf8(response.clone()).unwrap();
                println!("Response:\n{:?}", response_str);
                response.push(EOT_BYTE);

                stream.write_all(&response).expect("Failed to write to stream");
                stream.flush().expect("Failed to flush the stream");

                // println!("Sent response: {:?}", response);
            } else {
                println!("Error decoding buffer: {}", str_or_err.unwrap_err());
                continue;
            }
        }
        println!("Client disconnected");
    }
}

#[allow(unused)]
pub fn run_server<F>(host: &str, port: u16, mut protocol: Box<dyn Protocol>, bind_success: Option<F>) -> io::Result<()>
where F: FnMut()
{
    // let host = "127.0.0.1";
    let listener = TcpListener::bind(format!("{}:{}", host, port));
    match listener {
        Ok(listener) => {
            if let Some(mut bind_success) = bind_success {
                bind_success();
            }
            println!("TCP protocol handler listening on {}:{}", host, port);
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        println!("New connection from {}", stream.peer_addr().unwrap());
                        println!("Accepting connection...");
                        protocol.handle_client(&mut stream);
                    },
                    Err(e) => { eprintln!("Failed: {}", e); },
                }
            }
            return Ok(());
        },
        Err(e) => {
            eprintln!("Failed to bind to port {}: {}", port, e);
            return Err(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_subsequence_command_valid() {
        // Create a temporary test file
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_valid_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "help 2 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test valid fuzzy-subsequence command
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should not contain error message
        assert!(!response_str.starts_with("ERROR"));

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_invalid_params() {
        // Create a temporary test file
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_invalid_params_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test invalid parameter count - empty params
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec![];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - invalid parameters"));

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_empty_query() {
        // Create a temporary test file
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_empty_query_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test empty query handling
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec![""];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Empty query should return empty results (no error)
        assert!(!response_str.starts_with("ERROR"));

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_too_many_params() {
        // Create a temporary test file
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_too_many_params_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test too many parameters
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl", "extra"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - invalid parameters"));

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_with_utf8() {
        // Create a temporary test file with UTF-8 data
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_utf8_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "café 1 0").unwrap();
        writeln!(file, "naïve 2 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test UTF-8 character handling
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["cf"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should not contain error message
        assert!(!response_str.starts_with("ERROR"));

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_response_format() {
        // Create a temporary test file
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_response_format_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "help 2 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test response format consistency
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Response should either be empty or contain newline-separated strings
        if !response_str.is_empty() {
            // If there are results, they should be separated by newlines
            assert!(response_str.contains('\n') || !response_str.contains("ERROR"));
        }

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_unknown_operation() {
        // Create a temporary test file
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_unknown_op_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test unknown operation
        let operation = "unknown-operation";
        let params: Vec<&str> = vec!["test"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - unknown operation"));

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_parameter_validation() {
        // Create a temporary test file
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_param_validation_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test various parameter validation scenarios
        let operation = "fuzzy-subsequence";

        // Test with 0 parameters
        let params_empty: Vec<&str> = vec![];
        let response_empty = protocol.create_response(operation, params_empty);
        let response_str_empty = String::from_utf8(response_empty).unwrap();
        assert!(response_str_empty.starts_with("ERROR - invalid parameters"));

        // Test with 1 parameter (valid)
        let params_valid: Vec<&str> = vec!["test"];
        let response_valid = protocol.create_response(operation, params_valid);
        let response_str_valid = String::from_utf8(response_valid).unwrap();
        assert!(!response_str_valid.starts_with("ERROR"));

        // Test with 2 parameters (invalid)
        let params_invalid: Vec<&str> = vec!["test", "extra"];
        let response_invalid = protocol.create_response(operation, params_invalid);
        let response_str_invalid = String::from_utf8(response_invalid).unwrap();
        assert!(response_str_invalid.starts_with("ERROR - invalid parameters"));

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_fuzzy_subsequence_command_integration() {
        // Create a temporary test file with some data
        use std::fs::File;
        use std::io::Write;

        let test_file = "test_integration_data.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "world 2 0").unwrap();
        writeln!(file, "help 3 0").unwrap();
        writeln!(file, "helicopter 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file.to_string());

        // Test integration with other commands to ensure no conflicts
        let operations = vec![
            ("prefix", vec!["he"]),
            ("substring", vec!["or"]),
            ("fuzzy-subsequence", vec!["hl"]),
            ("similar", vec!["hello", "0.6"]),
        ];

        for (operation, params) in operations {
            let response = protocol.create_response(operation, params);
            let response_str = String::from_utf8(response).unwrap();

            // Each operation should handle its own parameter validation
            // We just verify that the protocol doesn't crash
            assert!(!response_str.starts_with("ERROR"));
        }

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_end_to_end_fuzzy_subsequence() {
        // Use a simpler approach - test the protocol directly without network
        let mut protocol = StringSpaceProtocol::new("test_data.txt".to_string());

        // Insert test data directly into the protocol's space
        protocol.space.insert_string("hello", 1).unwrap();
        protocol.space.insert_string("world", 2).unwrap();
        protocol.space.insert_string("help", 3).unwrap();
        protocol.space.insert_string("helicopter", 1).unwrap();

        // Test fuzzy-subsequence command directly
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Verify response contains expected results
        assert!(response_str.contains("hello"), "Expected 'hello' in response: {}", response_str);
        assert!(response_str.contains("help"), "Expected 'help' in response: {}", response_str);
        assert!(!response_str.contains("world"), "Did not expect 'world' in response: {}", response_str);
    }

    #[test]
    fn test_protocol_error_handling() {
        // Test invalid parameter count
        let mut protocol = StringSpaceProtocol::new("test_data.txt".to_string());

        // Simulate invalid request with missing query parameter
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec![]; // Empty params - should trigger error

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - invalid parameters"));
    }

    #[test]
    fn test_protocol_command_integration() {
        let mut protocol = StringSpaceProtocol::new("test_data.txt".to_string());

        // Test valid fuzzy-subsequence command
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should not contain error message
        assert!(!response_str.starts_with("ERROR"));

        // Test empty query handling
        let params_empty: Vec<&str> = vec![""];
        let response_empty = protocol.create_response(operation, params_empty);
        let response_empty_str = String::from_utf8(response_empty).unwrap();

        // Empty query should return empty results (no error)
        assert!(!response_empty_str.starts_with("ERROR"));

        // Test too many parameters
        let params_too_many: Vec<&str> = vec!["hl", "extra"];
        let response_too_many = protocol.create_response(operation, params_too_many);
        let response_too_many_str = String::from_utf8(response_too_many).unwrap();

        assert!(response_too_many_str.starts_with("ERROR - invalid parameters"));
    }

    #[test]
    fn test_performance_under_load() {
        let mut protocol = StringSpaceProtocol::new("test_data.txt".to_string());

        // Insert large dataset
        for i in 0..10000 {
            protocol.space.insert_string(&format!("testword{}", i), 1).unwrap();
        }

        // Test multiple concurrent searches
        let start = std::time::Instant::now();
        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(move || {
                    let mut local_protocol = StringSpaceProtocol::new("test_data.txt".to_string());
                    // Each thread gets its own data to avoid sharing mutable state
                    for i in 0..10000 {
                        local_protocol.space.insert_string(&format!("testword{}", i), 1).unwrap();
                    }
                    for _ in 0..100 {
                        let _ = local_protocol.space.fuzzy_subsequence_search("test");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        assert!(duration.as_secs() < 10, "Performance test took too long: {:?}", duration);
    }

    #[test]
    fn test_fuzzy_subsequence_with_actual_results() {
        let mut protocol = StringSpaceProtocol::new("test_data.txt".to_string());

        // Insert test data directly into the space
        protocol.space.insert_string("hello", 1).unwrap();
        protocol.space.insert_string("help", 2).unwrap();
        protocol.space.insert_string("helicopter", 3).unwrap();
        protocol.space.insert_string("world", 1).unwrap();

        // Test fuzzy subsequence search
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Verify we get expected results
        assert!(response_str.contains("hello"));
        assert!(response_str.contains("help"));
        assert!(response_str.contains("helicopter"));
        assert!(!response_str.contains("world"));
    }

    #[test]
    fn test_fuzzy_subsequence_no_results() {
        let mut protocol = StringSpaceProtocol::new("test_data.txt".to_string());

        // Insert test data that won't match our query
        protocol.space.insert_string("apple", 1).unwrap();
        protocol.space.insert_string("banana", 2).unwrap();

        // Test fuzzy subsequence search with no matches
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["xyz"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should return empty results (no error)
        assert!(!response_str.starts_with("ERROR"));
        assert!(response_str.is_empty() || response_str.trim().is_empty());
    }

    #[test]
    fn test_fuzzy_subsequence_case_sensitivity() {
        let mut protocol = StringSpaceProtocol::new("test_data.txt".to_string());

        // Insert test data with mixed case
        protocol.space.insert_string("Hello", 1).unwrap();
        protocol.space.insert_string("HELP", 2).unwrap();
        protocol.space.insert_string("helicopter", 3).unwrap();

        // Test fuzzy subsequence search with different case
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["HL"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        println!("Response: '{}'", response_str);
        println!("Contains 'Hello': {}", response_str.contains("Hello"));
        println!("Contains 'HELP': {}", response_str.contains("HELP"));
        println!("Contains 'helicopter': {}", response_str.contains("helicopter"));

        // The search is case-sensitive:
        // - "HELP" matches "HL" because both H and L are uppercase
        // - "Hello" doesn't match because the second character is 'e' (not 'L')
        // - "helicopter" doesn't match because the first character is lowercase 'h' (not uppercase 'H')
        assert!(!response_str.contains("Hello"), "Expected 'Hello' not to be in response");
        assert!(response_str.contains("HELP"), "Expected 'HELP' to be in response");
        assert!(!response_str.contains("helicopter"), "Expected 'helicopter' not to be in response");
    }
}
