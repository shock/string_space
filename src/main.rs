mod modules {
    pub mod utils;
    pub mod string_space;
    pub mod protocol;
    pub mod benchmark;
}

use modules::protocol::Protocol;
use modules::protocol::StringSpaceProtocol;
use modules::protocol::run_server;
use modules::benchmark::benchmark;

use clap::{Parser, Subcommand};
use std::fs;
use libc;
use std::path::PathBuf;

/// String Space Server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Command to execute
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the server
    Start {
        /// Path to string database text file (will be created if it doesn't exist)
        data_file: String,

        /// Run in background as daemon
        #[arg(short, long, default_value_t = false)]
        daemon: bool,

        /// TCP port to listen on
        #[arg(short, long, default_value_t = 7878)]
        port: u16,

        /// TCP host to bind on
        #[arg(short = 'H', long, default_value_t = String::from("127.0.0.1"))]
        host: String,
    },
    /// Stop the server
    Stop,
    /// Check the server status and display PID file information
    Status,
    /// Restart the server
    Restart {
        /// Path to string database text file (will be created if it doesn't exist)
        data_file: String,

        /// Run in background as daemon
        #[arg(short, long, default_value_t = false)]
        daemon: bool,

        /// TCP port to listen on
        #[arg(short, long, default_value_t = 7878)]
        port: u16,

        /// TCP host to bind on
        #[arg(short = 'H', long, default_value_t = String::from("127.0.0.1"))]
        host: String,

    },
    /// Run benchmarks
    Benchmark {
        /// Path to string database text file (will be created if it doesn't exist)
        data_file: String,
        /// Number of words to benchmark
        #[arg(short, long)]
        count: u32,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Start { daemon, port, host, data_file } => {
            start_server(daemon, host, port, data_file);
        }
        Command::Stop => {
            stop_server();
        }
        Command::Status => {
            check_status();
        }
        Command::Restart { daemon: _, port, host, data_file } => {
            restart_server(true, host.clone(), port, data_file);
        }
        Command::Benchmark { data_file, count } => {
            let v = vec![data_file, count.to_string()];
            benchmark(v);
            std::process::exit(0);
        }
    }
}

use modules::utils::create_pid_file;
use modules::utils::get_pid_file_path;

fn start_server(daemon: bool, host: String, port: u16, data_file: String) {
    let ssp: Box<dyn Protocol> = Box::new(StringSpaceProtocol::new(data_file.to_string()));

    // If running as a daemon, check for existing PID file
    if daemon {
        let app_name = env!("CARGO_PKG_NAME");
        let pid_file_path = get_pid_file_path(app_name);

        // Check if the PID file exists
        if fs::metadata(&pid_file_path).is_ok() {
            // Read the PID from the file
            let pid = fs::read_to_string(&pid_file_path)
                .expect("Unable to read PID file")
                .trim()
                .parse::<i32>()
                .expect("Invalid PID");

            // Check if the process with that PID is running
            if is_process_running(pid) {
                eprintln!("Server is already running with PID: {}", pid);
                std::process::exit(1); // Exit if the server is already running
            } else {
                // If the process is not running, we can proceed to start a new one
                eprintln!("Found stale PID file. Starting a new server instance.");
            }
        }
    }

    let mut pid = 0;

    if daemon {
        pid = unsafe { libc::fork() };
    }

    if pid < 0 {
        eprintln!("Failed to fork process");
        std::process::exit(1);
    } else if pid == 0 {
        // Child process
        let app_name = env!("CARGO_PKG_NAME");
        let pid_file_path = get_pid_file_path(app_name);

        let mut bind_success = false;
        let bind_success_fn = || {
            // Write the child's PID to the PID file
            if let Err(e) = create_pid_file(&pid_file_path) {
                eprintln!("Error creating PID file: {}", e);
                std::process::exit(1);
            }
            // Set up signal handling for graceful shutdown
            setup_signal_handling();
            bind_success = true; // Indicate that binding was successful
        };

        // Now run the server, passing the bind success function
        let result = run_server(&host, port, ssp, Some(bind_success_fn));

        match result {
            Ok(_) => {
                // Cleanup PID file before exiting
                if bind_success {
                    remove_pid_file(&pid_file_path);
                    std::process::exit(0); // Exit gracefully
                } else {
                    std::process::exit(1); // Exit gracefully with error code 1
                }
            },
            Err(e) => {
                eprintln!("Error running server: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Parent process
        std::process::exit(0); // Exit the parent process immediately
    }
}

fn setup_signal_handling() {
    unsafe {
        let sa_mask: libc::sigset_t = std::mem::zeroed();
        let sigaction = libc::sigaction {
            sa_sigaction: signal_handler as usize,
            sa_mask,
            sa_flags: libc::SA_SIGINFO | libc::SA_RESTART, // Add SA_RESTART to restart interrupted system calls
        };
        if libc::sigaction(libc::SIGTERM, &sigaction, std::ptr::null_mut()) < 0 {
            eprintln!("Failed to set up signal handler");
            std::process::exit(1);
        }
    }
}

extern "C" fn signal_handler(_signum: i32) {
    // Clean up PID file before exiting
    let app_name = env!("CARGO_PKG_NAME");
    let pid_file_path = get_pid_file_path(app_name);
    remove_pid_file(&pid_file_path);
    std::process::exit(0); // Exit the child process
}

fn remove_pid_file(pid_file_path: &PathBuf) {
    if let Err(e) = fs::remove_file(pid_file_path) {
        eprintln!("Unable to remove PID file: {}", e);
    }
}

fn stop_server() {
    // Read the PID from the file and kill the process
    let app_name = env!("CARGO_PKG_NAME");
    let pid_file_path = get_pid_file_path(app_name);

    // Check if the PID file exists
    if !fs::metadata(&pid_file_path).is_ok() {
        eprintln!("PID file does not exist. Server may not be running.");
        return;
    }

    let pid = fs::read_to_string(&pid_file_path).expect("Unable to read PID file");
    let pid: i32 = pid.trim().parse().expect("Invalid PID");

    // Verify that the process is running and is our process
    if is_process_running(pid) {
        eprintln!("Server is running with PID: {}", pid);
        eprintln!("Sending SIGTERM to the process...");
        let _ = unsafe { libc::kill(pid, libc::SIGTERM) }; // Send SIGTERM to the process
        // wait one second for the process to exit
        std::thread::sleep(std::time::Duration::from_secs(1));
        // check if the process is still running
        if is_process_running(pid) {
            eprintln!("Server is still running with PID: {}", pid);
            eprintln!("Sending SIGKILL to the process...");
            // send SIGKILL to the process
            let _ = unsafe { libc::kill(pid, libc::SIGKILL) };
            // wait one second for the process to exit
            std::thread::sleep(std::time::Duration::from_secs(1));
            if is_process_running(pid) {
                eprintln!("Server is still running with PID: {}", pid);
                std::process::exit(1); // Exit if the server is still running
            }
        }
        eprintln!("Server terminated successfully");
        // remove the PID file if it exists
        let pid_file_exists = fs::metadata(&pid_file_path).is_ok();
        if pid_file_exists {
            remove_pid_file(&pid_file_path);
        }
    } else {
        eprintln!("No running process found with PID: {}", pid);
    }
}

fn check_status() {
    // Check if the server is running by checking the PID file
    let app_name = env!("CARGO_PKG_NAME");
    let pid_file_path = get_pid_file_path(app_name).clone(); // Clone the value to avoid moving

    println!("PID file location: {}", pid_file_path.display());

    // Check if the PID file exists
    if fs::metadata(&pid_file_path).is_ok() {
        println!("PID file exists: Yes");
        let pid = fs::read_to_string(&pid_file_path).expect("Unable to read PID file");
        let pid: i32 = pid.trim().parse().expect("Invalid PID");

        // Verify that the process is running and is our process
        if is_process_running(pid) {
            println!("Server is running with PID: {}", pid);
        } else {
            println!("Server is not running (stale PID).");
        }
    } else {
        println!("PID file exists: No");
        println!("Server is not running (PID file does not exist).");
    }
}

fn restart_server(daemon: bool, host: String, port: u16, data_file: String) {
    stop_server();
    // Add a delay here to ensure the server has stopped
    std::thread::sleep(std::time::Duration::from_secs(1));
    start_server(daemon, host, port, data_file);
}

fn is_process_running(pid: i32) -> bool {
    // Check if a process with the given PID is running using libc::kill
    unsafe {
        libc::kill(pid, 0) == 0 // Returns 0 if the process exists
    }
}
