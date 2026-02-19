use thiserror::Error;

/// All domain-level errors for the radar-ip tool.
#[derive(Error, Debug)]
pub enum RadarError {
    #[error("SSH connection error to {0}: {1}")]
    SshConnection(String, String),

    #[error("SSH command execution error on {0}: {1}")]
    CommandExecution(String, String),

    #[error("Invalid IP range: '{0}'")]
    InvalidIpRange(String),

    #[error("Private key authentication error: {0}")]
    PrivateKey(String),

    #[error("Password authentication error: {0}")]
    Password(String),

    #[error("MAC address '{0}' not found on any host in the scanned range")]
    MacNotFound(String),
}