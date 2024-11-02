// ./src/main.rs

use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        std::process::exit(1);
    }
    let command = args[1].as_str();
    match command {
        "run" => {
            let status = Command::new("cargo")
                .args(["run", "--release"])
                .status()
                .expect("Failed to run cargo run");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(-1));
            }
        }
        "build" => {
            let status = Command::new("cargo")
                .args(["build", "--release"])
                .status()
                .expect("Failed to run cargo build");
            if !status.success() {
                std::process::exit(status.code().unwrap_or(-1));
            }
        }
        _ => {
            eprintln!("Invalid command: {}", command);
            std::process::exit(1);
        }
    }
}
