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
    /// path to string database text file (will be created if it doesn't exist)
    #[arg(value_name = "data-file", index = 1)]
    data_file: String,

    /// run in background as daemon
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


fn main() {
    let args = Args::parse();

    if args.benchmark.is_some() {
        let v = vec![args.data_file, args.benchmark.unwrap().to_string()];
        benchmark(v);
        std::process::exit(0);
    }

    let file_path = args.data_file;
    let ssp: Box<dyn Protocol> = Box::new(StringSpaceProtocol::new(file_path.to_string())); // Use the trait here
    run_server(&args.host, args.port, ssp);
}