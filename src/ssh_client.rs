use crate::errors::RadarError;
use regex::Regex;
use ssh2::Session;
use std::io::Read;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;

/// SSH authentication method.
#[derive(Debug, Clone)]
pub enum AuthenticationMethod {
    /// Authenticate with a username/password pair.
    Password(String),
    /// Authenticate with a private key file and an optional passphrase.
    PrivateKey {
        path: PathBuf,
        passphrase: Option<String>,
    },
}

/// SSH connection configuration.
#[derive(Debug, Clone)]
pub struct SshConfig {
    /// Remote username.
    pub user: String,
    /// Remote port (typically 22).
    pub port: u16,
    /// Authentication method.
    pub auth: AuthenticationMethod,
    /// TCP connect + auth timeout.
    pub timeout: Duration,
}

/// Information gathered from a single device.
#[derive(Debug)]
pub struct DeviceIdentity {
    /// The IP address that was probed.
    #[allow(dead_code)]
    pub ip: String,
    /// All MAC addresses found on that host (lowercase, colon-separated).
    pub mac_list: Vec<String>,
}

impl SshConfig {
    /// Connect to `ip`, run `ip link show`, parse every MAC address, and
    /// return a [`DeviceIdentity`].  This is a **blocking** function and is
    /// intended to be called from inside `tokio::task::spawn_blocking`.
    pub fn fetch_macs(&self, ip: &str) -> Result<DeviceIdentity, RadarError> {
        // ── 1. TCP connect with timeout ───────────────────────────────────
        let addr = format!("{}:{}", ip, self.port);
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| RadarError::SshConnection(ip.to_string(), e.to_string()))?
            .next()
            .ok_or_else(|| {
                RadarError::SshConnection(ip.to_string(), "could not resolve address".into())
            })?;

        let stream = TcpStream::connect_timeout(&socket_addr, self.timeout)
            .map_err(|e| RadarError::SshConnection(ip.to_string(), e.to_string()))?;

        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|e| RadarError::SshConnection(ip.to_string(), e.to_string()))?;

        // ── 2. SSH handshake ─────────────────────────────────────────────
        let mut session = Session::new()
            .map_err(|e| RadarError::SshConnection(ip.to_string(), e.to_string()))?;
        session.set_tcp_stream(stream);
        session
            .handshake()
            .map_err(|e| RadarError::SshConnection(ip.to_string(), e.to_string()))?;

        // ── 3. Authenticate ───────────────────────────────────────────────
        match &self.auth {
            AuthenticationMethod::Password(pwd) => {
                session
                    .userauth_password(&self.user, pwd)
                    .map_err(|e| RadarError::Password(e.to_string()))?;
            }
            AuthenticationMethod::PrivateKey { path, passphrase } => {
                session
                    .userauth_pubkey_file(
                        &self.user,
                        None,
                        path,
                        passphrase.as_deref(),
                    )
                    .map_err(|e| RadarError::PrivateKey(e.to_string()))?;
            }
        }

        if !session.authenticated() {
            return Err(RadarError::SshConnection(
                ip.to_string(),
                "authentication failed".into(),
            ));
        }

        // ── 4. Run command ────────────────────────────────────────────────
        let mut channel = session
            .channel_session()
            .map_err(|e| RadarError::CommandExecution(ip.to_string(), e.to_string()))?;

        channel
            .exec("ip link show")
            .map_err(|e| RadarError::CommandExecution(ip.to_string(), e.to_string()))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| RadarError::CommandExecution(ip.to_string(), e.to_string()))?;

        let _ = channel.wait_close();

        // ── 5. Parse MAC addresses ────────────────────────────────────────
        // Matches patterns like  "link/ether aa:bb:cc:dd:ee:ff"
        let re = Regex::new(r"(?i)link/ether\s+([0-9a-f]{2}(?::[0-9a-f]{2}){5})")
            .expect("MAC regex is valid");

        let mac_list: Vec<String> = re
            .captures_iter(&output)
            .map(|cap| cap[1].to_lowercase())
            .collect();

        Ok(DeviceIdentity {
            ip: ip.to_string(),
            mac_list,
        })
    }
}
