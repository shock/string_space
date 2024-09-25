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

use clap::Parser;

/// String Space Server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to string database text file (will be created if it doesn't exist)
    #[arg(value_name = "data-file", index = 1)]
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

    /// Run benchmarks with COUNT words - WARNING: data-file will be overwritten!!
    #[arg(short, long, value_name = "COUNT")]
    benchmark: Option<u32>,
}

use std::fs;
use libc;

fn main() {
    let args = Args::parse();

    // If benchmark is specified, run the benchmark and exit
    if args.benchmark.is_some() {
        let v = vec![args.data_file.clone(), args.benchmark.unwrap().to_string()];
        benchmark(v);
        std::process::exit(0);
    }

    // Start the server with the provided arguments
    start_server(args);
}

use modules::utils::create_pid_file;
use modules::utils::get_pid_file_path;
use std::path::PathBuf;

fn start_server(args: Args) {
    let file_path = args.data_file;
    let ssp: Box<dyn Protocol> = Box::new(StringSpaceProtocol::new(file_path.to_string()));

    // If not running as a daemon, run the server directly
    if !args.daemon {
        let _ = run_server(&args.host, args.port, ssp, Some(|| {}));
        std::process::exit(0);
    }

    // Fork the process
    let pid = unsafe { libc::fork() };

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
        let result = run_server(&args.host, args.port, ssp, Some(bind_success_fn));

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
    let pid = fs::read_to_string(&pid_file_path).expect("Unable to read PID file");
    let pid: i32 = pid.trim().parse().expect("Invalid PID");
    let _ = unsafe { libc::kill(pid, libc::SIGTERM) }; // Send SIGTERM to the process
    remove_pid_file(&pid_file_path);
}

fn check_status() {
    // Check if the server is running by checking the PID file
    let app_name = env!("CARGO_PKG_NAME");
    let pid_file_path = get_pid_file_path(app_name);
    if fs::metadata(pid_file_path).is_ok() {
        println!("Server is running.");
    } else {
        println!("Server is not running.");
    }
}

fn restart_server(args: Args) {
    stop_server();
    // Add a delay here to ensure the server has stopped
    std::thread::sleep(std::time::Duration::from_secs(1));
    start_server(args); // Pass any necessary arguments here
}
