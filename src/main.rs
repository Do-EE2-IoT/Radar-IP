mod cli;
mod errors;
mod scanner;
mod ssh_client;

use clap::Parser;
use cli::CliArgs;
use scanner::Scanner;
use ssh_client::{AuthenticationMethod, SshConfig};
use std::process;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Initialize the logger (controlled by the RUST_LOG environment variable).
    env_logger::init();

    // Parse CLI arguments.
    let args = CliArgs::parse();

    // Build the SSH authentication method.
    let auth = match (args.key_path, args.password) {
        (Some(path), passphrase) => AuthenticationMethod::PrivateKey { path, passphrase },
        (None, Some(pwd)) => AuthenticationMethod::Password(pwd),
        (None, None) => {
            eprintln!("Error: you must provide either --password or --key for SSH authentication.");
            process::exit(1);
        }
    };

    let config = SshConfig {
        user: args.user,
        port: 22,
        auth,
        timeout: Duration::from_secs(args.timeout_sec),
    };

    println!("radar-ip starting...");
    println!("  Target MAC : {}", args.target_mac);
    println!("  IP range   : {}", args.ip_range);

    let scanner = Scanner::new(config, args.target_mac);

    match scanner.scan(&args.ip_range).await {
        Ok(ip) => {
            println!("-------------------------------------------");
            println!("SUCCESS  Device found.");
            println!("IP Address : {}", ip);
            println!("-------------------------------------------");
        }
        Err(e) => {
            eprintln!("FAILED  {}", e);
            process::exit(1);
        }
    }
}