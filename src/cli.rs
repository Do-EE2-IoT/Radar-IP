use clap::Parser;
use std::path::PathBuf;

/// CLI arguments for the radar-ip tool.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Scan an IP range via SSH and find which host owns a given MAC address",
    long_about = None
)]
pub struct CliArgs {
    /// Target MAC address to search for (e.g. aa:bb:cc:dd:ee:ff)
    #[arg(short = 'm', long)]
    pub target_mac: String,

    /// IP range in CIDR notation (e.g. 192.168.1.0/24)
    #[arg(short = 'r', long = "range")]
    pub ip_range: String,

    /// Path to private key file for SSH authentication
    #[arg(short = 'k', long = "key")]
    pub key_path: Option<PathBuf>,

    /// Password for SSH authentication (also used as key passphrase when --key is set)
    #[arg(short = 'p', long)]
    pub password: Option<String>,

    /// SSH username
    #[arg(short = 'u', long, default_value = "root")]
    pub user: String,

    /// SSH connection timeout in seconds
    #[arg(long, default_value_t = 5)]
    pub timeout_sec: u64,
}