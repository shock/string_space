use std::fs;
use std::io::{self};
mod modules {
    pub mod utils;
    pub mod string_space;
    pub mod protocol;
}

use modules::utils::generate_random_words;
use modules::utils::time_execution;
use modules::string_space::StringSpace;
use modules::string_space::StringRef;
use modules::protocol::Protocol;
use modules::protocol::StringSpaceProtocol;
use modules::protocol::run_server;

#[allow(unused)]
fn benchmark(args: Vec<String>) {
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

use clap::Parser;

/// String Space Server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// path to string database text file (will be created if it doesn't exist)
    #[arg(value_name = "data-file", index = 1)]
    data_file: String,

    /// TCP port to listen on
    #[arg(short, long, default_value_t = 7878)]
    port: u16,

    /// TCP host to bind on
    #[arg(short = 'H', long, default_value_t = String::from("127.0.0.1"))]
    host: String,

    /// Run benchmarks with COUNT words - WARNING: data-file will be overwritten!!
    #[arg(short, long, value_name = "COUNT")]
    benchmark: Option<u32>,
}


fn main() {
    let args = Args::parse();
    // If benchmark is false, data_file must be provided
    // if args.benchmark.is_none() && args.data_file.is_none() {
    //     eprintln!("Error: The data file argument is required unless the benchmark flag is used.");
    //     eprintln!("\nFor more information, try '--help'.");
    //     std::process::exit(1);
    // }

    if args.benchmark.is_some() {
        let v = vec![args.data_file, args.benchmark.unwrap().to_string()];
        benchmark(v);
        std::process::exit(0);
    }

    let file_path = args.data_file;
    let ssp: Box<dyn Protocol> = Box::new(StringSpaceProtocol::new(file_path.to_string())); // Use the trait here
    run_server(7878, ssp);
}