use rand::Rng;

pub fn generate_random_words(num_words: usize, max_length: u32, alpha_max: u8) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let alpha_max = alpha_max as u32 - 1;

    if alpha_max > 25 {
        println!("Invalid alpha_max value (1-26): {}", alpha_max);
        return Vec::new();
    }

    // Preallocate the Vec<String> with the specified capacity
    let mut words = Vec::with_capacity(num_words);

    for _ in 0..num_words {
        let word_length = rng.gen_range(3..=max_length); // Random length between 3 and 20
        let word: String = (0..word_length)
            .map(|_| {
                // Generate a random character between 'a' (97) and 'a' + alpha_max (97 + alpha_max)
                rng.gen_range(97..=97 + alpha_max) as u8 as char // Lowercase letters
            })
            .collect(); // Collect into a String

        words.push(word); // Add the generated word to the Vec
    }

    words // Return the Vec<String>
}

use std::time::Instant;
use std::time::Duration;

pub fn time_execution<F>(closure: F) -> Duration
where
    F: FnOnce(),
{
    let start = Instant::now(); // Start the timer
    closure(); // Execute the closure
    let duration = start.elapsed(); // Calculate the elapsed time
    duration // Return the duration in milliseconds
}

use std::path::PathBuf;
use dirs;

#[allow(unused)]
pub fn expand_path(path: &str) -> String {
    let path_buf = if path.starts_with("~") {
        dirs::home_dir()
            .map(|mut home| {
                home.push(&path[2..]);
                home
            })
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    };

    path_buf.canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
}