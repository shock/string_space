use std::{env, fs};
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter, BufRead};
use std::time::Instant;
use rand::Rng;
use rand::distributions::Alphanumeric;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <file> <num>", args[0]);
        std::process::exit(1);
    }
    let insert_remove_words_count = 100;

    let file_path = &args[1];
    let num_words: usize = args[2].parse().expect("Invalid number of words");

    // TRUNCATE FILE IF IT EXISTS
    // let start = Instant::now();
    if let Err(e) = fs::remove_file(file_path) {
        if e.kind() != io::ErrorKind::NotFound {
            panic!("Error deleting file: {}", e);
        }
    }
    fs::File::create(file_path)?;
    // let truncate_time = start.elapsed();
    // println!("Truncating file... took {:?}", truncate_time);

    // GENRATE RANDOM WORDS
    let start = Instant::now();
    let mut words = generate_random_words(num_words);
    let gen_time = start.elapsed();
    assert!(words.len() == num_words);

    // SORT WORDS
    let start = Instant::now();
    words.sort();
    let sort_time = start.elapsed();

    // REMOVE DUPLICATES
    let start = Instant::now();
    words.dedup();
    let dedup_time = start.elapsed();
    let words_len: usize = words.len();

    // WRITE WORDS TO FILE
    let start = Instant::now();
    write_words_to_file(&words, file_path)?;
    let write_time = start.elapsed();

    // READ WORDS FROM FILE
    let mut words: Vec<String>;
    let start = Instant::now();
    words = read_words_from_file(file_path)?;
    let read_time = start.elapsed();
    assert!(words.len() == words_len);

    // GET FIRST insert_remove_words_count WORDS FROM WORDS
    let mut words_to_remove = Vec::with_capacity(insert_remove_words_count);
    for word in words.iter().take(insert_remove_words_count) {
        words_to_remove.push(word.clone());
    }

    // INSERT WORDS WHILE PRESENT 1
    let start = Instant::now();
    for word in words_to_remove.iter() {
        insert_word_if_unique(&mut words, word.clone());
    }
    let insert_present_time1 = start.elapsed();
    assert!(words.len() == words_len);


    // REMOVE WORDS 2
    let start = Instant::now();
    for word in words_to_remove.iter_mut() {
        remove_word_if_present(&mut words, word.clone());
    }
    let remove_time2 = start.elapsed();
    assert!(words.len() == words_len-insert_remove_words_count);

    // INSERT WORDS BACK IN 2
    let start = Instant::now();
    for word in words_to_remove.iter() {
        insert_word_if_unique(&mut words, word.clone());
    }
    let insert_not_present_time1 = start.elapsed();
    assert!(words.len() == words_len);

    // REVERSE WORDS
    let start = Instant::now();
    words.reverse();
    let reverse_time = start.elapsed();

    // WRITE WORDS BACK TO FILE
    let start = Instant::now();
    write_words_to_file(&words, file_path)?;
    let write_time_sorted = start.elapsed();

    println!("Creating random list of words... took {:?}", gen_time);
    println!("Sorting words... took {:?}", sort_time);
    println!("Removing duplicates... took {:?}", dedup_time);
    println!("Writing words to file... took {:?}", write_time);
    println!("Reading words from file... took {:?}", read_time);
    println!("Inserting {} words while present 1 ... took {:?}", insert_remove_words_count, insert_present_time1);
    println!("Removing {} words 2 ... took {:?}", insert_remove_words_count, remove_time2);
    println!("Inserting {} words back in 1 ... took {:?}", insert_remove_words_count, insert_not_present_time1);
    println!("Reversing words... took {:?}", reverse_time);
    println!("Writing sorted words back to file... took {:?}", write_time_sorted);
    println!("Total time to complete test: {:?}", read_time + sort_time + dedup_time + reverse_time + write_time_sorted);

    Ok(())
}

fn generate_random_words(num_words: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();

    // Preallocate the Vec<String> with the specified capacity
    let mut words = Vec::with_capacity(num_words);

    for _ in 0..num_words {
        let word_length = rng.gen_range(3..=20); // Random length between 3 and 20
        let word: String = (0..word_length)
            .map(|_| rng.sample(Alphanumeric) as char) // Generate random characters
            .collect(); // Collect into a String

        words.push(word); // Add the generated word to the Vec
    }

    words // Return the Vec<String>
}

fn insert_word_if_unique(words: &mut Vec<String>, word: String) -> bool {
    if !words.iter().any(|w| w == &word) {
        words.push(word);
        return true;
    }
    false
}

fn get_word_index_if_present(words: &Vec<String>, word: &String) -> Option<usize> {
    words.iter().position(|existing_word| existing_word == word)
}

fn remove_word_if_present(words: &mut Vec<String>, word: String) -> bool {
    if let Some(index) = words.iter().position(|existing_word| existing_word == &word) {
        words.remove(index);
        return true;
    }
    false
}

fn write_words_to_file(words: &[String], file_path: &str) -> io::Result<()> {
    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);
    for word in words {
        writeln!(writer, "{}", word)?;
    }
    Ok(())
}

fn read_words_from_file(file_path: &str) -> io::Result<Vec<String>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    reader.lines().collect()
}
