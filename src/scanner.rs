use crate::errors::RadarError;
use crate::ssh_client::SshConfig;
use ipnet::Ipv4Net;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;

/// Maximum number of concurrent SSH connections.
const MAX_CONCURRENT: usize = 50;

/// Scans an IP range over SSH and looks for a specific MAC address.
pub struct Scanner {
    config: SshConfig,
    target_mac: String,
}

impl Scanner {
    /// Create a new scanner.
    pub fn new(config: SshConfig, target_mac: String) -> Self {
        Self { config, target_mac }
    }

    /// Scan every host in `cidr` (e.g. `"192.168.1.0/24"`) concurrently.
    ///
    /// Returns the first IP address whose ARP/link table contains
    /// `target_mac`, or [`RadarError::MacNotFound`] if none is found.
    pub async fn scan(&self, cidr: &str) -> Result<String, RadarError> {
        // ── 1. Parse CIDR ─────────────────────────────────────────────────
        let net: Ipv4Net = cidr
            .parse()
            .map_err(|_| RadarError::InvalidIpRange(cidr.to_string()))?;

        let hosts: Vec<_> = net.hosts().collect();
        info!("Scanning {} host(s) in {}", hosts.len(), cidr);
        println!("Scanning {} host(s) in {} ...", hosts.len(), cidr);

        // ── 2. Semaphore to cap concurrency ───────────────────────────────
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
        let target_mac = self.target_mac.to_lowercase();

        let mut handles = Vec::with_capacity(hosts.len());

        for ip in hosts {
            let ip_str = ip.to_string();
            let config = self.config.clone();
            let mac = target_mac.clone();
            let sem = semaphore.clone();

            let handle = task::spawn(async move {
                // Acquire permit before blocking the thread pool.
                let _permit = sem.acquire().await.ok()?;

                task::spawn_blocking(move || {
                    match config.fetch_macs(&ip_str) {
                        Ok(identity) => {
                            if identity.mac_list.iter().any(|m| m == &mac) {
                                info!("Found target MAC on {}", ip_str);
                                Some(ip_str)
                            } else {
                                None
                            }
                        }
                        Err(e) => {
                            // Silently skip unreachable / auth-failed hosts.
                            warn!("{}: {}", ip_str, e);
                            None
                        }
                    }
                })
                .await
                .ok()
                .flatten()
            });

            handles.push(handle);
        }

        // ── 3. Collect results, return on first match ─────────────────────
        for handle in handles {
            if let Ok(Some(found_ip)) = handle.await {
                return Ok(found_ip);
            }
        }

        Err(RadarError::MacNotFound(self.target_mac.clone()))
    }
}