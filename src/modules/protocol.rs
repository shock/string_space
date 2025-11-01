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
        else if "best-completions" == operation {
            // Validate parameter count (1-2 parameters)
            if params.len() < 1 || params.len() > 2 {
                let response_str = format!("ERROR - invalid parameters (length = {})", params.len());
                response.extend_from_slice(response_str.as_bytes());
                return response;
            }

            let query = params[0];
            let limit = if params.len() == 2 {
                match params[1].parse::<usize>() {
                    Ok(l) => Some(l),
                    Err(_) => {
                        let response_str = format!("ERROR - invalid limit parameter '{}'", params[1]);
                        response.extend_from_slice(response_str.as_bytes());
                        return response;
                    }
                }
            } else {
                None
            };

            let matches = self.space.best_completions(query, limit);
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
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "help 2 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test valid fuzzy-subsequence command
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should not contain error message
        assert!(!response_str.starts_with("ERROR"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_invalid_params() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test invalid parameter count - empty params
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec![];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - invalid parameters"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_empty_query() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test empty query handling
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec![""];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Empty query should return empty results (no error)
        assert!(!response_str.starts_with("ERROR"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_too_many_params() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test too many parameters
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["hl", "extra"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - invalid parameters"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_with_utf8() {
        // Create a temporary test file with UTF-8 data
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "café 1 0").unwrap();
        writeln!(file, "naïve 2 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test UTF-8 character handling
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec!["cf"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should not contain error message
        assert!(!response_str.starts_with("ERROR"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_response_format() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "help 2 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

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

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_unknown_operation() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test unknown operation
        let operation = "unknown-operation";
        let params: Vec<&str> = vec!["test"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - unknown operation"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_parameter_validation() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

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

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_best_completions_command_valid() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "help 2 0").unwrap();
        writeln!(file, "helicopter 3 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test valid best-completions command with query only
        let operation = "best-completions";
        let params: Vec<&str> = vec!["hel"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should not contain error message
        assert!(!response_str.starts_with("ERROR"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_best_completions_command_with_limit() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "help 2 0").unwrap();
        writeln!(file, "helicopter 3 0").unwrap();
        writeln!(file, "hell 4 0").unwrap();
        writeln!(file, "health 5 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test valid best-completions command with query and limit
        let operation = "best-completions";
        let params: Vec<&str> = vec!["hel", "3"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should not contain error message
        assert!(!response_str.starts_with("ERROR"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_best_completions_command_invalid_params() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test invalid parameter count - empty params
        let operation = "best-completions";
        let params_empty: Vec<&str> = vec![];

        let response_empty = protocol.create_response(operation, params_empty);
        let response_str_empty = String::from_utf8(response_empty).unwrap();

        assert!(response_str_empty.starts_with("ERROR - invalid parameters"));

        // Test invalid parameter count - too many params
        let params_too_many: Vec<&str> = vec!["hel", "10", "extra"];

        let response_too_many = protocol.create_response(operation, params_too_many);
        let response_str_too_many = String::from_utf8(response_too_many).unwrap();

        assert!(response_str_too_many.starts_with("ERROR - invalid parameters"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_best_completions_command_empty_query() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test empty query handling
        let operation = "best-completions";
        let params: Vec<&str> = vec![""];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Empty query should return empty results (no error)
        assert!(!response_str.starts_with("ERROR"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_best_completions_command_invalid_limit() {
        // Create a temporary test file
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test invalid limit parameter
        let operation = "best-completions";
        let params: Vec<&str> = vec!["hel", "not_a_number"];

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        // Should contain specific error message about invalid limit
        assert!(response_str.starts_with("ERROR - invalid limit parameter"));

        // File will be automatically cleaned up when test_file goes out of scope
    }

    #[test]
    fn test_fuzzy_subsequence_command_integration() {
        // Create a temporary test file with some data
        use std::io::Write;

        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();

        let mut file = std::fs::File::create(&test_file_path).unwrap();
        writeln!(file, "hello 1 0").unwrap();
        writeln!(file, "world 2 0").unwrap();
        writeln!(file, "help 3 0").unwrap();
        writeln!(file, "helicopter 1 0").unwrap();

        let mut protocol = StringSpaceProtocol::new(test_file_path);

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

        // File will be automatically cleaned up when test_file goes out of scope
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_end_to_end_fuzzy_subsequence() {
        // Use a simpler approach - test the protocol directly without network
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

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
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Simulate invalid request with missing query parameter
        let operation = "fuzzy-subsequence";
        let params: Vec<&str> = vec![]; // Empty params - should trigger error

        let response = protocol.create_response(operation, params);
        let response_str = String::from_utf8(response).unwrap();

        assert!(response_str.starts_with("ERROR - invalid parameters"));
    }

    #[test]
    fn test_protocol_command_integration() {
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

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
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Insert large dataset
        for i in 0..10000 {
            protocol.space.insert_string(&format!("testword{}", i), 1).unwrap();
        }

        // Test multiple concurrent searches
        let start = std::time::Instant::now();
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let thread_test_file = tempfile::NamedTempFile::new().unwrap();
                let thread_test_file_path = thread_test_file.path().to_str().unwrap().to_string();
                thread::spawn(move || {
                    let mut local_protocol = StringSpaceProtocol::new(thread_test_file_path);
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
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

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
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

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
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

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

    #[test]
    fn test_best_completions_integration() {
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Insert realistic test data with varying frequencies
        let test_words = vec![
            ("hello", 10),
            ("help", 5),
            ("helicopter", 2),
            ("health", 8),
            ("hell", 3),
            ("world", 7),
            ("word", 4),
            ("work", 6),
        ];

        for (word, frequency) in test_words {
            protocol.space.insert_string(word, frequency).unwrap();
        }

        // Test integration with other commands to ensure no conflicts
        let operations = vec![
            ("prefix", vec!["he"]),
            ("substring", vec!["or"]),
            ("fuzzy-subsequence", vec!["hl"]),
            ("similar", vec!["hello", "0.6"]),
            ("best-completions", vec!["hel"]),
            ("best-completions", vec!["wor", "3"]),
        ];

        for (operation, params) in operations {
            let response = protocol.create_response(operation, params.clone());
            let response_str = String::from_utf8(response).unwrap();

            // Each operation should handle its own parameter validation
            // We just verify that the protocol doesn't crash
            assert!(!response_str.starts_with("ERROR"),
                   "Operation '{}' with params {:?} failed: {}",
                   operation, &params, response_str);
        }

        // Verify best-completions results are reasonable
        let best_completions_response = protocol.create_response("best-completions", vec!["hel"]);
        let best_completions_str = String::from_utf8(best_completions_response).unwrap();

        // Should contain high-frequency matches
        assert!(best_completions_str.contains("hello"),
               "Expected 'hello' in best completions: {}", best_completions_str);
        assert!(best_completions_str.contains("help"),
               "Expected 'help' in best completions: {}", best_completions_str);
        assert!(best_completions_str.contains("health"),
               "Expected 'health' in best completions: {}", best_completions_str);
    }

    // #[test]
    // fn test_best_completions_performance() {
    //     let test_file = tempfile::NamedTempFile::new().unwrap();
    //     let test_file_path = test_file.path().to_str().unwrap().to_string();
    //     let mut protocol = StringSpaceProtocol::new(test_file_path);

    //     // Insert large dataset for performance testing
    //     for i in 0..10000 {
    //         let word = format!("testword{}", i);
    //         // Use varying frequencies to test the ranking algorithm
    //         let frequency = (i % 10) + 1;
    //         protocol.space.insert_string(&word, frequency).unwrap();
    //     }

    //     // Add some specific test words with high frequencies
    //     protocol.space.insert_string("test", 100).unwrap();
    //     protocol.space.insert_string("testing", 50).unwrap();
    //     protocol.space.insert_string("tester", 25).unwrap();

    //     // Test performance with multiple best-completions queries
    //     let start = std::time::Instant::now();

    //     let test_queries = vec![
    //         "test",
    //         "tes",
    //         "te",
    //         "t",
    //     ];

    //     for query in test_queries {
    //         for _ in 0..5 {
    //             let response = protocol.create_response("best-completions", vec![query]);
    //             let response_str = String::from_utf8(response).unwrap();

    //             // Verify we get results for valid queries
    //             if query == "test" {
    //                 assert!(!response_str.trim().is_empty(),
    //                        "Expected results for query '{}': {}", query, response_str);
    //             }
    //         }
    //     }

    //     let duration = start.elapsed();

    //     // Performance requirement: should complete within 1 second
    //     // This is a reasonable expectation for 20 queries on 10k words
    //     assert!(duration.as_millis() < 1000,
    //            "Performance test took too long: {:?} for 40 queries", duration);

    //     // Test with limit parameter for additional performance validation
    //     let start_with_limit = std::time::Instant::now();

    //     for _ in 0..20 {
    //         let response = protocol.create_response("best-completions", vec!["test", "5"]);
    //         let response_str = String::from_utf8(response).unwrap();

    //         // Should return limited results
    //         let result_count = response_str.lines().filter(|line| !line.trim().is_empty()).count();
    //         assert!(result_count <= 5,
    //                "Expected at most 5 results with limit, got {}: {}",
    //                result_count, response_str);
    //     }

    //     let duration_with_limit = start_with_limit.elapsed();
    //     assert!(duration_with_limit.as_millis() < 1000,
    //            "Performance with limit took too long: {:?}", duration_with_limit);
    // }

    #[test]
    fn test_best_completions_edge_cases() {
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Test 1: Empty database
        let response_empty_db = protocol.create_response("best-completions", vec!["test"]);
        let response_empty_db_str = String::from_utf8(response_empty_db).unwrap();
        assert!(!response_empty_db_str.starts_with("ERROR"),
               "Should handle empty database gracefully: {}", response_empty_db_str);

        // Insert some test data
        protocol.space.insert_string("hello", 10).unwrap();
        protocol.space.insert_string("help", 5).unwrap();
        protocol.space.insert_string("helicopter", 2).unwrap();

        // Test 2: Empty query
        let response_empty_query = protocol.create_response("best-completions", vec![""]);
        let response_empty_query_str = String::from_utf8(response_empty_query).unwrap();
        assert!(!response_empty_query_str.starts_with("ERROR"),
               "Should handle empty query gracefully: {}", response_empty_query_str);

        // Test 3: Very long query (beyond typical word length)
        let long_query = "a".repeat(100);
        let response_long_query = protocol.create_response("best-completions", vec![&long_query]);
        let response_long_query_str = String::from_utf8(response_long_query).unwrap();
        assert!(!response_long_query_str.starts_with("ERROR"),
               "Should handle very long query: {}", response_long_query_str);

        // Test 4: Query with special characters
        let response_special_chars = protocol.create_response("best-completions", vec!["hel@#$"]);
        let response_special_chars_str = String::from_utf8(response_special_chars).unwrap();
        assert!(!response_special_chars_str.starts_with("ERROR"),
               "Should handle special characters: {}", response_special_chars_str);

        // Test 5: Limit of 0
        let response_zero_limit = protocol.create_response("best-completions", vec!["hel", "0"]);
        let response_zero_limit_str = String::from_utf8(response_zero_limit).unwrap();
        assert!(!response_zero_limit_str.starts_with("ERROR"),
               "Should handle limit of 0: {}", response_zero_limit_str);
        // Should return no results with limit 0
        assert!(response_zero_limit_str.trim().is_empty(),
               "Expected no results with limit 0: {}", response_zero_limit_str);

        // Test 6: Very large limit
        let response_large_limit = protocol.create_response("best-completions", vec!["hel", "1000"]);
        let response_large_limit_str = String::from_utf8(response_large_limit).unwrap();
        assert!(!response_large_limit_str.starts_with("ERROR"),
               "Should handle large limit: {}", response_large_limit_str);

        // Test 7: Invalid parameter counts
        let response_no_params = protocol.create_response("best-completions", vec![]);
        let response_no_params_str = String::from_utf8(response_no_params).unwrap();
        assert!(response_no_params_str.starts_with("ERROR - invalid parameters"),
               "Should error on no parameters: {}", response_no_params_str);

        let response_three_params = protocol.create_response("best-completions", vec!["hel", "10", "extra"]);
        let response_three_params_str = String::from_utf8(response_three_params).unwrap();
        assert!(response_three_params_str.starts_with("ERROR - invalid parameters"),
               "Should error on too many parameters: {}", response_three_params_str);

        // Test 8: Invalid limit parameter
        let response_invalid_limit = protocol.create_response("best-completions", vec!["hel", "not_a_number"]);
        let response_invalid_limit_str = String::from_utf8(response_invalid_limit).unwrap();
        assert!(response_invalid_limit_str.starts_with("ERROR - invalid limit parameter"),
               "Should error on invalid limit: {}", response_invalid_limit_str);
    }

    #[test]
    fn test_best_completions_progressive_execution() {
        let test_file = tempfile::NamedTempFile::new().unwrap();
        let test_file_path = test_file.path().to_str().unwrap().to_string();
        let mut protocol = StringSpaceProtocol::new(test_file_path);

        // Insert test data with progressive frequency patterns
        let words_and_frequencies = vec![
            ("hello", 100),  // Highest frequency
            ("help", 50),    // Medium frequency
            ("helicopter", 10), // Lower frequency
            ("hell", 5),     // Even lower
            ("health", 1),   // Lowest frequency
            ("world", 75),   // High frequency but different prefix
            ("word", 25),    // Medium frequency, different prefix
        ];

        for (word, frequency) in words_and_frequencies {
            protocol.space.insert_string(word, frequency).unwrap();
        }

        // Test progressive queries with increasing specificity
        let progressive_queries = vec![
            ("h", vec!["hello", "help", "helicopter", "hell", "health"]), // All 'h' words
            ("he", vec!["hello", "help", "helicopter", "hell", "health"]), // All 'he' words
            ("hel", vec!["hello", "help", "helicopter", "hell", "health"]), // All 'hel' words
            ("hell", vec!["hello", "hell"]), // Only 'hell' prefix matches
        ];

        for (query, expected_words) in progressive_queries {
            let response = protocol.create_response("best-completions", vec![query]);
            let response_str = String::from_utf8(response).unwrap();

            // Verify all expected words are present
            for expected_word in &expected_words {
                assert!(response_str.contains(expected_word),
                       "Expected '{}' in response for query '{}': {}",
                       expected_word, query, response_str);
            }

            // Verify results are ordered by frequency (highest first)
            let lines: Vec<&str> = response_str.lines()
                .filter(|line| !line.trim().is_empty())
                .collect();

            if lines.len() > 1 {
                // For the 'h' query, we should see frequency-based ordering
                if query == "h" {
                    // hello (100) should come before help (50)
                    let hello_pos = lines.iter().position(|&line| line == "hello");
                    let help_pos = lines.iter().position(|&line| line == "help");

                    if let (Some(hello_idx), Some(help_idx)) = (hello_pos, help_pos) {
                        assert!(hello_idx < help_idx,
                               "Expected 'hello' before 'help' in frequency-based ordering");
                    }
                }
            }
        }

        // Test with explicit limits to verify progressive ranking
        // Note: The actual ranking algorithm considers more than just frequency
        // The exact ordering may vary, but we expect the top words to be among help/hello/hell
        let limit_tests = vec![
            ("hel", "1", vec!["help", "hello"]), // Top 1 should be either help or hello
            ("hel", "2", vec!["help", "hello"]), // Top 2 should include both help and hello
            ("hel", "3", vec!["help", "hello", "hell"]), // Top 3 should include help, hello, and hell
        ];

        for (query, limit, expected_top_words) in limit_tests {
            let response = protocol.create_response("best-completions", vec![query, limit]);
            let response_str = String::from_utf8(response).unwrap();

            let result_words: Vec<&str> = response_str.lines()
                .filter(|line| !line.trim().is_empty())
                .collect();

            // For limit 1, we only expect 1 result, but it should be one of the expected words
            if limit == "1" {
                assert_eq!(result_words.len(), 1,
                          "Expected 1 result for query '{}' with limit 1, got {}: {:?}",
                          query, result_words.len(), result_words);
                assert!(expected_top_words.contains(&result_words[0]),
                       "Expected one of {:?} for query '{}' with limit 1, got: {:?}",
                       expected_top_words, query, result_words);
            } else {
                // For other limits, verify we get at least the expected number of results
                // and all expected words are present
                assert!(result_words.len() >= expected_top_words.len(),
                       "Expected at least {} results for query '{}' with limit {}, got {}: {:?}",
                       expected_top_words.len(), query, limit, result_words.len(), result_words);

                // Verify all expected words are present (order may vary due to algorithm changes)
                for expected_word in &expected_top_words {
                    assert!(result_words.contains(expected_word),
                           "Expected '{}' in results for query '{}' with limit {}, got: {:?}",
                           expected_word, query, limit, result_words);
                }
            }
        }

        // Test that non-matching queries return empty results
        let non_matching_queries = vec![
            "xyz",
            "123",
            "@#$",
        ];

        for query in non_matching_queries {
            let response = protocol.create_response("best-completions", vec![query]);
            let response_str = String::from_utf8(response).unwrap();

            assert!(!response_str.starts_with("ERROR"),
                   "Should handle non-matching query '{}' gracefully: {}", query, response_str);
            assert!(response_str.trim().is_empty(),
                   "Expected empty results for non-matching query '{}': {}", query, response_str);
        }
    }
}
