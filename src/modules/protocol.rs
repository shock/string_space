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
                let response_str = format!("ERROR\nInvalid parameters (length = {})", params.len());
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
pub fn run_server(host: &str, port: u16, mut protocol: Box<dyn Protocol>) {
    // let host = "127.0.0.1";
    let listener = TcpListener::bind(format!("{}:{}", host, port));
    match listener {
        Ok(listener) => {
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
        },
        Err(e) => {
            eprintln!("Failed to bind to port {}: {}", port, e);
        }
    }
}
