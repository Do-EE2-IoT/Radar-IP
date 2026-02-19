# radar-ip

A command-line tool that scans an IP range over SSH and finds which host owns a specific MAC address.

---

## Use Case

You know the MAC address of a device but not its current IP address on the local network.  
`radar-ip` SSHs into every host in a given subnet simultaneously, reads the network interface table, and tells you the first IP that matches.

---

## Prerequisites

| Requirement | Notes |
|---|---|
| Rust ≥ 1.75 | Install via [rustup.rs](https://rustup.rs) |
| OpenSSL / libssh2 | Usually already present on Linux/macOS. On Windows install via `vcpkg` or use the `vendored` feature of `ssh2` |
| SSH access to all target hosts | Password or private-key auth supported |

---

## Build

```bash
# Debug build
cargo build

# Optimised release build (recommended)
cargo build --release
```

The binary is placed at `target/release/radar-ip` (or `target\release\radar-ip.exe` on Windows).

---

## Usage

```
radar-ip [OPTIONS] --target-mac <TARGET_MAC> --range <CIDR>
```

### Options

| Flag | Short | Default | Description |
|---|---|---|---|
| `--target-mac` | `-m` | *(required)* | MAC address to search for, e.g. `aa:bb:cc:dd:ee:ff` |
| `--range` | `-r` | *(required)* | Subnet in CIDR notation, e.g. `192.168.1.0/24` |
| `--user` | `-u` | `root` | SSH username |
| `--password` | `-p` | — | SSH password (also used as key passphrase when `--key` is given) |
| `--key` | `-k` | — | Path to a private key file for public-key authentication |
| `--timeout-sec` | — | `5` | Per-host TCP connect + auth timeout (seconds) |

> **Note:** You must supply at least one of `--password` or `--key`.

---

## Examples

### Password authentication
```bash
radar-ip --target-mac aa:bb:cc:dd:ee:ff --range 192.168.1.0/24 --user pi --password raspberry
```

### Private key authentication
```bash
radar-ip -m aa:bb:cc:dd:ee:ff -r 10.0.0.0/23 -u admin --key ~/.ssh/id_rsa
```

### Private key with passphrase
```bash
radar-ip -m aa:bb:cc:dd:ee:ff -r 192.168.0.0/24 -k ~/.ssh/id_ed25519 -p "my passphrase"
```

```bash
PS D:\key\Tool\radar-ip\target\release> .\radar-ip.exe -m 84:fc:14:01:e0:73 -r 10.8.0.0/24 --user root -k "D:\key\hcg1_Lumi" -p "********"
```

### Enable debug logging
```bash
RUST_LOG=debug radar-ip -m aa:bb:cc:dd:ee:ff -r 192.168.1.0/24 -p secret
```

### Sample output (success)
```
radar-ip starting...
  Target MAC : aa:bb:cc:dd:ee:ff
  IP range   : 192.168.1.0/24
Scanning 254 host(s) in 192.168.1.0/24 ...
-------------------------------------------
SUCCESS  Device found.
IP Address : 192.168.1.42
-------------------------------------------
```

### Sample output (not found)
```
FAILED  MAC address 'aa:bb:cc:dd:ee:ff' not found on any host in the scanned range
```

---

## Code Flow

```
main()
  │
  ├─ 1. env_logger::init()            Set up logging (reads RUST_LOG env var)
  │
  ├─ 2. CliArgs::parse()              Parse CLI flags via clap
  │
  ├─ 3. Build AuthenticationMethod    Match (key_path, password):
  │      ├─ (Some, _)  → PrivateKey { path, passphrase }
  │      ├─ (None, Some) → Password(pwd)
  │      └─ (None, None) → eprintln + exit(1)
  │
  ├─ 4. Build SshConfig               { user, port: 22, auth, timeout }
  │
  └─ 5. Scanner::scan(&ip_range).await
           │
           ├─ Parse CIDR with `ipnet`     → Vec<Ipv4Addr> (all hosts)
           │
           ├─ Create Semaphore(50)        Limits to 50 concurrent SSH sessions
           │
           ├─ For each IP → tokio::spawn:
           │    ├─ Acquire semaphore permit
           │    └─ spawn_blocking → SshConfig::fetch_macs(ip)
           │              │
           │              ├─ TcpStream::connect_timeout
           │              ├─ ssh2::Session::handshake
           │              ├─ Authenticate (Password | PrivateKey)
           │              ├─ channel.exec("ip link show")
           │              ├─ Read stdout to String
           │              └─ Regex parse MAC addresses
           │                   Pattern: link/ether <xx:xx:xx:xx:xx:xx>
           │
           ├─ Collect JoinHandles in order
           └─ Iterate handles:
                ├─ Some(ip) found → return Ok(ip)  ← early exit on first match
                └─ None / Err    → skip, continue
                      └─ All done with no match → Err(MacNotFound)
```

---

## Project Structure

```
radar-ip/
├── Cargo.toml
└── src/
    ├── main.rs         Entry point — argument parsing, config assembly, result printing
    ├── cli.rs          Clap CLI struct definition (CliArgs)
    ├── errors.rs       RadarError enum (thiserror)
    ├── scanner.rs      Scanner struct — async concurrent scan loop with semaphore
    └── ssh_client.rs   SshConfig + fetch_macs() — blocking SSH + MAC extraction
```

### Module responsibilities

| Module | Responsibility |
|---|---|
| `main.rs` | Wire-up: parse args → build config → run scanner → print result |
| `cli.rs` | Declare all CLI flags with clap `#[derive(Parser)]` |
| `errors.rs` | Single `RadarError` enum for all domain errors |
| `scanner.rs` | Iterate hosts, spawn concurrent `spawn_blocking` tasks, return first match |
| `ssh_client.rs` | Open TCP → SSH handshake → authenticate → exec command → parse MACs |

---

## Error Handling

All errors are represented by `RadarError` (defined in `errors.rs`, powered by `thiserror`):

| Variant | Meaning |
|---|---|
| `SshConnection(ip, reason)` | TCP connect or SSH handshake failed |
| `CommandExecution(ip, reason)` | SSH channel or exec failed |
| `InvalidIpRange(cidr)` | Supplied CIDR string could not be parsed |
| `PrivateKey(reason)` | Private-key authentication failed |
| `Password(reason)` | Password authentication failed |
| `MacNotFound(mac)` | No host in the range had the target MAC |

Hosts that are unreachable or whose auth fails are **silently skipped** during a scan — only the final "not found" state is surfaced to the user.

---

## License

MIT
