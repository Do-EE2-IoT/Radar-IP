# radar-ip

A native desktop tool that scans an IP range over SSH and finds which host owns a specific MAC address.  
Version **0.2.0** — GUI-based with built-in device profiles and env-based secret management.

---

## Use Case

You know the **MAC address** of a device but not its current **IP address** on the network.  
Radar-IP SSHs into every host in a given subnet concurrently, reads each host's network interface table, and returns the first IP that matches the target MAC.

---

## Quick Start

### 1. Prerequisites

| Requirement | Notes |
|---|---|
| Rust ≥ 1.75 | Install via [rustup.rs](https://rustup.rs) |
| OpenSSL / libssh2 | Usually pre-installed. On Windows, bundled by the `ssh2` crate |

### 2. Set Up Secrets

Create a `.env` file in the project root (already in `.gitignore`):

```env
SSH_PASSWORD=YourPasswordHere

HC_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----
...your HC gateway private key...
-----END RSA PRIVATE KEY-----"

AI3_PRIVATE_KEY="-----BEGIN OPENSSH PRIVATE KEY-----
...your AI box private key...
-----END OPENSSH PRIVATE KEY-----"
```

### 3. Build & Run

```bash
cargo run            # debug build + launch GUI
cargo build --release  # optimised release binary
```

---

## GUI Overview

```
┌──────────────────────────────────────────┐
│          🔍 Radar-IP Scanner             │
│   Find a device IP address by its MAC    │
├──────────────────────────────────────────┤
│                                          │
│  Device Type   [ 🏠 HC ] [📦 AI2] [🤖 AI3]│
│  SSH User      root                      │
│  MAC Address   [aa:bb:cc:dd:ee:ff      ] │
│  IP Range      [10.8.0.0/24           ] │
│                                          │
│            [ 🚀 Scan Now ]               │
│                                          │
├──────────────────────────────────────────┤
│                                          │
│          ✅ Device Found!                │
│       ┌─────────────────────┐            │
│       │  10.8.0.42   📋 Copy│            │
│       └─────────────────────┘            │
│                                          │
│            radar-ip v0.2.0               │
└──────────────────────────────────────────┘
```

### Device Profiles

When you switch the **Device Type**, the SSH user and IP range auto-fill:

| Profile | SSH Key Env Var | SSH User | Default IP Range |
|---------|----------------|----------|------------------|
| 🏠 HC   | `HC_PRIVATE_KEY`  | `root` | `10.8.0.0/24` |
| 📦 AI2  | `AI3_PRIVATE_KEY` | `nano` | `10.8.0.0/24` |
| 🤖 AI3  | `AI3_PRIVATE_KEY` | `pi`   | `192.168.255.0/24` |

---

## Detailed Code Flow

### Application Startup

```
main()
  │
  ├─ dotenvy::dotenv()         Load .env → sets SSH_PASSWORD,
  │                            HC_PRIVATE_KEY, AI3_PRIVATE_KEY
  │                            as environment variables
  │
  ├─ env_logger::init()        Set up logging (RUST_LOG env var)
  │
  └─ eframe::run_native()      Launch native GUI window (480×480)
       │
       └─ RadarApp::new()      Read env vars:
            ├─ SSH_PASSWORD    → stored in app state
            └─ default profile → HC (user="root", range="10.8.0.0/24")
```

### GUI Event Loop

```
RadarApp::update()    ← called every frame by egui
  │
  ├─ Profile change detected?
  │    YES → auto-fill ip_range and ssh_user from DeviceProfile
  │
  ├─ Render UI:
  │    ├─ Device Type selector  (HC / AI2 / AI3)
  │    ├─ SSH User label        (read-only, from profile)
  │    ├─ MAC Address input     (editable text field)
  │    ├─ IP Range input        (editable, pre-filled from profile)
  │    └─ Scan Now button
  │
  ├─ Render results:
  │    ├─ Idle     → "Enter a MAC address and press Scan"
  │    ├─ Scanning → spinner + "Scanning network..."
  │    ├─ Found    → green IP display + Copy button
  │    └─ Error    → red error message with details
  │
  └─ On button click → start_scan()
```

### Scan Execution Flow

```
start_scan()
  │
  ├─ Set scan_state = Scanning
  │
  ├─ Load private key from env var:
  │    HC  → std::env::var("HC_PRIVATE_KEY")
  │    AI2 → std::env::var("AI3_PRIVATE_KEY")
  │    AI3 → std::env::var("AI3_PRIVATE_KEY")
  │
  ├─ Build SshConfig:
  │    { user, port: 22, auth: PrivateKeyMemory, timeout: 3s }
  │
  └─ std::thread::spawn(background thread)
       │
       └─ tokio::Runtime::block_on
            │
            └─ tokio::time::timeout(15 seconds)
                 │
                 └─ Scanner::scan(&ip_range)
                      │         (see Scanner Flow below)
                      │
                      ├─ Ok(ip)  → scan_state = Found(ip)
                      ├─ Err(e)  → scan_state = Error(e)
                      └─ Timeout → scan_state = Error("timed out")
```

### Scanner Flow (Concurrent Network Scan)

```
Scanner::scan(cidr)
  │
  ├─ 1. Parse CIDR string → Vec<Ipv4Addr>
  │      e.g. "10.8.0.0/24" → 254 host addresses
  │
  ├─ 2. Create Semaphore(50)  ← limits to 50 concurrent SSH sessions
  │
  ├─ 3. For EACH host IP:
  │      │
  │      └─ tokio::spawn(async)
  │           │
  │           ├─ Acquire semaphore permit (wait if 50 already active)
  │           │
  │           └─ spawn_blocking → SshConfig::fetch_macs(ip)
  │                │
  │                ├─ Match found  → return Some(ip)
  │                ├─ No match     → return None
  │                └─ Error        → log warning, store first error, return None
  │
  ├─ 4. Iterate all JoinHandles:
  │      ├─ First Some(ip)  → return Ok(ip)     ← EARLY EXIT
  │      └─ All None        → return Err(MacNotFound + first error details)
  │
  └─ MacNotFound error includes the first SSH/auth error encountered
     for easier debugging
```

### SSH Connection Flow (Per Host, Blocking)

```
SshConfig::fetch_macs(ip)
  │
  ├─ 1. TCP Connect
  │      TcpStream::connect_timeout(ip:22, 3s)
  │
  ├─ 2. SSH Handshake
  │      ssh2::Session → handshake()
  │
  ├─ 3. Authenticate
  │      ├─ PrivateKeyMemory:
  │      │    ├─ Normalize line endings (CRLF → LF)
  │      │    ├─ Ensure trailing newline
  │      │    ├─ Write key to NamedTempFile
  │      │    └─ session.userauth_pubkey_file(user, tmp_path, passphrase)
  │      │
  │      ├─ PrivateKey (file path):
  │      │    └─ session.userauth_pubkey_file(user, path, passphrase)
  │      │
  │      └─ Password:
  │           └─ session.userauth_password(user, password)
  │
  ├─ 4. Execute Remote Command
  │      channel.exec("ip link show")
  │      → read stdout to String
  │
  └─ 5. Parse MAC Addresses
         Regex: link/ether ([0-9a-f]{2}(:[0-9a-f]{2}){5})
         → Vec<String> of lowercase MACs
         → DeviceIdentity { ip, mac_list }
```

---

## Project Structure

```
radar-ip/
├── .env               Private keys + password (git-ignored)
├── .gitignore          Excludes: /target, .env, /secret/
├── Cargo.toml          Dependencies and project metadata
├── README.md           This file
├── secret/             Raw private key files (git-ignored)
│   ├── hcg1_Lumi       HC Gateway private key (RSA PEM format)
│   └── hcg1_aibox      AI Box private key (OpenSSH format)
└── src/
    ├── main.rs          Entry point — loads .env, launches GUI window
    ├── gui.rs           GUI layout, device profiles, scan trigger
    ├── scanner.rs       Concurrent scan loop with semaphore
    ├── ssh_client.rs    SSH connect + auth + exec + MAC parsing
    ├── errors.rs        RadarError enum (thiserror)
    └── cli.rs           CLI argument definitions (legacy, still compiled)
```

### Module Responsibilities

| Module | Role |
|--------|------|
| `main.rs` | Load `.env` → init logger → launch eframe GUI |
| `gui.rs` | UI layout, device profile logic (HC/AI2/AI3), scan state machine, background scan trigger |
| `scanner.rs` | Parse CIDR, spawn concurrent `spawn_blocking` tasks with semaphore, collect first match |
| `ssh_client.rs` | TCP connect → SSH handshake → authenticate (password / key file / key-from-env) → exec command → regex parse MACs |
| `errors.rs` | `RadarError` enum: `SshConnection`, `CommandExecution`, `InvalidIpRange`, `PrivateKey`, `Password`, `MacNotFound` |
| `cli.rs` | Clap `#[derive(Parser)]` struct (retained for potential CLI mode) |

---

## Error Handling

| Error Variant | When |
|---|---|
| `SshConnection(ip, reason)` | TCP connect or SSH handshake failed |
| `CommandExecution(ip, reason)` | SSH channel/exec failed |
| `PrivateKey(reason)` | Private key auth failed (format, passphrase, permissions) |
| `Password(reason)` | Password auth failed |
| `InvalidIpRange(cidr)` | CIDR string could not be parsed |
| `MacNotFound(mac)` | No host matched + shows first SSH error for diagnostics |

Unreachable hosts are **silently skipped** during scanning. If no host matches, the **first error** encountered is surfaced to help debugging.

---

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `SSH_PASSWORD` | Yes | Passphrase for encrypted private keys |
| `HC_PRIVATE_KEY` | Yes (for HC) | PEM-encoded private key for HC Gateway |
| `AI3_PRIVATE_KEY` | Yes (for AI2/AI3) | Private key for AI Box devices |
| `RUST_LOG` | No | Log level: `debug`, `info`, `warn`, `error` |

---

## Security

- Private keys are loaded from **environment variables** (via `.env` file), never hardcoded
- `.env` and `secret/` directory are in `.gitignore` — never committed
- Keys are written to **temporary files** at runtime for SSH auth, then automatically deleted
- The GUI does **not display** any key material — only the SSH username is visible

---

## License

MIT
