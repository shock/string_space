use std::{env, fs};
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter, BufRead};
use std::time::Instant;
use rand::Rng;
use rand::distributions::Alphanumeric;

mod models {
    pub mod word_struct;
}

use models::word_struct::WordStruct;


fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <file> <num>", args[0]);
        std::process::exit(1);
    }
    let insert_remove_words_count = 100;

    let file_path = &args[1];
    let num_words: usize = args[2].parse().expect("Invalid number of w_structs");

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
    let mut w_structs: Vec<WordStruct> = generate_random_words(num_words);
    let gen_time = start.elapsed();
    assert!(w_structs.len() == num_words);

    // SORT w_structs
    let start = Instant::now();
    w_structs.sort();
    let sort_time = start.elapsed();
    assert!(w_structs.len() == num_words);

    // REMOVE DUPLICATES
    let start = Instant::now();
    w_structs.dedup();
    let dedup_time = start.elapsed();
    let words_len: usize = w_structs.len();

    // WRITE w_structs TO FILE
    let start = Instant::now();
    write_word_recs_to_file(&w_structs, file_path)?;
    let write_time = start.elapsed();

    // Read w_structs from file
    let mut w_structs: Vec<WordStruct>;
    let start = Instant::now();
    w_structs = read_word_recs_from_file(file_path)?;
    let read_time = start.elapsed();
    assert!(w_structs.len() == words_len);

    // Create a separate vector for w_structs to manipulate
    let mut words_to_manipulate: Vec<WordStruct> = Vec::new();
    for w_struct in w_structs.iter().take(insert_remove_words_count) {
        words_to_manipulate.push(w_struct.clone());
    }

    // INSERT w_structs WHILE PRESENT 1
    let start = Instant::now();
    for w_struct in words_to_manipulate.iter() {
        insert_word_if_unique(&mut w_structs, w_struct.get_word());
    }
    let insert_present_time1 = start.elapsed();
    println!("wstructs len: {}, words_len: {}", w_structs.len(), words_len);
    assert!(w_structs.len() == words_len);

    // REMOVE w_structs 2
    let start = Instant::now();
    for w_struct in words_to_manipulate.iter() {
        remove_word_if_present(&mut w_structs, w_struct.get_word());
    }
    let remove_time2 = start.elapsed();
    assert!(w_structs.len() == words_len-insert_remove_words_count);

    // INSERT w_structs BACK IN 2
    let start = Instant::now();
    for w_struct in words_to_manipulate.iter() {
        insert_word_if_unique(&mut w_structs, w_struct.get_word());
    }
    let insert_not_present_time1 = start.elapsed();
    assert!(w_structs.len() == words_len);

    // REVERSE w_structs
    let start = Instant::now();
    w_structs.reverse();
    let reverse_time = start.elapsed();

    // WRITE w_structs BACK TO FILE
    let start = Instant::now();
    write_word_recs_to_file(&w_structs, file_path)?;
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

fn generate_random_words(num_words: usize) -> Vec<WordStruct> {
    let mut rng = rand::thread_rng();

    // Preallocate the Vec<String> with the specified capacity
    let mut w_structs = Vec::with_capacity(num_words);

    for _ in 0..num_words {
        let word_length = rng.gen_range(3..=20); // Random length between 3 and 20
        let word: String = (0..word_length)
            .map(|_| rng.sample(Alphanumeric) as char) // Generate random characters
            .collect(); // Collect into a String

        w_structs.push(word); // Add the generated word to the Vec
    }

    let word_sructs: Vec<WordStruct> = w_structs.into_iter().map(|w| WordStruct::new(w, 0)).collect();

    word_sructs // Return the Vec<WordStruct>
}

fn insert_word_if_unique(w_structs: &mut Vec<WordStruct>, word: &String) -> bool {
    match w_structs.binary_search_by(|probe| probe.get_word().cmp(word)) {
        Ok(_) => false,
        Err(pos) => {
            w_structs.insert(pos, WordStruct::new(word.to_string(), 0));
            true
        }
    }
}

fn remove_word_if_present(w_structs: &mut Vec<WordStruct>, word: &String) -> bool {
    match w_structs.binary_search_by(|probe| probe.get_word().as_str().cmp(word)) {
        Ok(pos) => {
            w_structs.remove(pos);
            true
        }
        Err(_) => false,
    }
}

fn write_word_recs_to_file(w_structs: &[WordStruct], file_path: &str) -> io::Result<()> {
    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);
    for word in w_structs {
        writeln!(writer, "{} {}", word.get_word(), word.get_count())?;
    }
    Ok(())
}

fn read_word_recs_from_file(file_path: &str) -> io::Result<Vec<WordStruct>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let word_recs: Vec<WordStruct> = reader.lines().map(|line| {
        if let Ok(mut line) = line {
            let parts: Vec<&str> = line.split(" ").collect();
            let word = parts[0];
            let count = parts[1].parse::<u32>().unwrap();
            // trim line to length of word
            line.truncate(word.len());
            WordStruct::new(line, count)
        } else {
            println!("Error reading line from file");
            WordStruct::new(String::new(), 0)
        }
    }).collect();
    Ok(word_recs)
}
