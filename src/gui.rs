use crate::scanner::Scanner;
use crate::ssh_client::{AuthenticationMethod, SshConfig};
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The supported device profiles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceProfile {
    HC,
    AI2,
    AI3,
}

impl DeviceProfile {
    /// Environment variable name that holds the private key for this profile.
    fn env_key_name(self) -> &'static str {
        match self {
            DeviceProfile::HC  => "HC_PRIVATE_KEY",
            DeviceProfile::AI2 => "AI3_PRIVATE_KEY",
            DeviceProfile::AI3 => "AI3_PRIVATE_KEY",
        }
    }

    /// Default IP range for this device type.
    fn default_ip_range(self) -> &'static str {
        match self {
            DeviceProfile::HC  => "10.8.0.0/24",
            DeviceProfile::AI2 => "10.8.0.0/24",
            DeviceProfile::AI3 => "192.168.255.0/24",
        }
    }

    /// Default SSH username for this device type.
    fn default_user(self) -> &'static str {
        match self {
            DeviceProfile::HC  => "root",
            DeviceProfile::AI2 => "nano",
            DeviceProfile::AI3 => "pi",
        }
    }
}

/// Possible scan states.
#[derive(Debug, Clone)]
enum ScanState {
    Idle,
    Scanning,
    Found(String),
    Error(String),
}

/// Main application state.
pub struct RadarApp {
    mac_input: String,
    ip_range: String,
    profile: DeviceProfile,
    prev_profile: DeviceProfile,
    scan_state: Arc<Mutex<ScanState>>,
    ssh_password: String,
    ssh_user: String,
}

impl RadarApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let default_profile = DeviceProfile::HC;
        let ssh_password = std::env::var("SSH_PASSWORD").unwrap_or_default();

        Self {
            mac_input: String::new(),
            ip_range: default_profile.default_ip_range().to_string(),
            profile: default_profile,
            prev_profile: default_profile,
            scan_state: Arc::new(Mutex::new(ScanState::Idle)),
            ssh_password,
            ssh_user: default_profile.default_user().to_string(),
        }
    }
}

impl eframe::App for RadarApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-fill IP range and SSH user when the user switches device profile.
        if self.profile != self.prev_profile {
            self.ip_range = self.profile.default_ip_range().to_string();
            self.ssh_user = self.profile.default_user().to_string();
            self.prev_profile = self.profile;
        }

        // Request repaint while scanning so the UI stays responsive.
        let current_state = self.scan_state.lock().unwrap().clone();
        if matches!(current_state, ScanState::Scanning) {
            ctx.request_repaint();
        }

        // ── Custom dark theme with accent ────────────────────────────────
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        ctx.set_style(style);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                // ── Title ─────────────────────────────────────────────
                ui.heading(
                    egui::RichText::new("🔍 Radar-IP Scanner")
                        .size(28.0)
                        .strong()
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Find a device IP address by its MAC")
                        .size(14.0)
                        .color(egui::Color32::from_gray(160)),
                );
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(15.0);
            });

            // ── Input section ─────────────────────────────────────────
            egui::Grid::new("input_grid")
                .num_columns(2)
                .spacing([12.0, 10.0])
                .min_col_width(120.0)
                .show(ui, |ui| {
                    // Device profile
                    ui.label(
                        egui::RichText::new("Device Type")
                            .size(15.0)
                            .color(egui::Color32::from_rgb(180, 220, 255)),
                    );
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.profile, DeviceProfile::HC, "🏠 HC");
                        ui.selectable_value(&mut self.profile, DeviceProfile::AI2, "📦 AI2");
                        ui.selectable_value(&mut self.profile, DeviceProfile::AI3, "🤖 AI3");
                    });
                    ui.end_row();

                    // SSH User (read-only, auto-filled from profile)
                    ui.label(
                        egui::RichText::new("SSH User")
                            .size(15.0)
                            .color(egui::Color32::from_rgb(180, 220, 255)),
                    );
                    ui.label(
                        egui::RichText::new(&self.ssh_user)
                            .size(15.0)
                            .family(egui::FontFamily::Monospace)
                            .color(egui::Color32::from_rgb(200, 200, 200)),
                    );
                    ui.end_row();

                    // MAC address
                    ui.label(
                        egui::RichText::new("MAC Address")
                            .size(15.0)
                            .color(egui::Color32::from_rgb(180, 220, 255)),
                    );
                    let mac_edit = egui::TextEdit::singleline(&mut self.mac_input)
                        .hint_text("aa:bb:cc:dd:ee:ff")
                        .desired_width(260.0)
                        .font(egui::TextStyle::Monospace);
                    ui.add(mac_edit);
                    ui.end_row();

                    // IP range
                    ui.label(
                        egui::RichText::new("IP Range")
                            .size(15.0)
                            .color(egui::Color32::from_rgb(180, 220, 255)),
                    );
                    let range_edit = egui::TextEdit::singleline(&mut self.ip_range)
                        .hint_text("192.168.1.0/24")
                        .desired_width(260.0)
                        .font(egui::TextStyle::Monospace);
                    ui.add(range_edit);
                    ui.end_row();
                });

            ui.add_space(20.0);

            // ── Scan button ───────────────────────────────────────────
            let is_scanning = matches!(current_state, ScanState::Scanning);

            ui.vertical_centered(|ui| {
                let button = if is_scanning {
                    egui::Button::new(
                        egui::RichText::new("⏳ Scanning...")
                            .size(18.0)
                            .color(egui::Color32::from_gray(180)),
                    )
                } else {
                    egui::Button::new(
                        egui::RichText::new("🚀 Scan Now")
                            .size(18.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(30, 120, 200))
                };

                let btn = ui.add_sized([200.0, 45.0], button);

                if btn.clicked() && !is_scanning && !self.mac_input.trim().is_empty() {
                    self.start_scan(ctx.clone());
                }
            });

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(15.0);

            // ── Result section ────────────────────────────────────────
            ui.vertical_centered(|ui| {
                match &current_state {
                    ScanState::Idle => {
                        ui.label(
                            egui::RichText::new("Enter a MAC address and press Scan")
                                .size(14.0)
                                .color(egui::Color32::from_gray(120)),
                        );
                    }
                    ScanState::Scanning => {
                        ui.spinner();
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Scanning network, please wait...")
                                .size(14.0)
                                .color(egui::Color32::from_rgb(255, 200, 80)),
                        );
                    }
                    ScanState::Found(ip) => {
                        ui.label(
                            egui::RichText::new("✅ Device Found!")
                                .size(18.0)
                                .strong()
                                .color(egui::Color32::from_rgb(80, 220, 100)),
                        );
                        ui.add_space(10.0);

                        // Big IP display with copy button
                        egui::Frame::default()
                            .fill(egui::Color32::from_rgb(30, 50, 30))
                            .rounding(8.0)
                            .inner_margin(egui::Margin::same(16.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(ip)
                                            .size(32.0)
                                            .strong()
                                            .color(egui::Color32::from_rgb(100, 255, 130))
                                            .family(egui::FontFamily::Monospace),
                                    );
                                    if ui.button("📋 Copy").clicked() {
                                        ui.output_mut(|o| o.copied_text = ip.clone());
                                    }
                                });
                            });
                    }
                    ScanState::Error(msg) => {
                        ui.label(
                            egui::RichText::new("❌ Scan Failed")
                                .size(18.0)
                                .strong()
                                .color(egui::Color32::from_rgb(255, 90, 90)),
                        );
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new(msg)
                                .size(13.0)
                                .color(egui::Color32::from_rgb(255, 160, 160)),
                        );
                    }
                }
            });

            // ── Footer ────────────────────────────────────────────────
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("radar-ip v0.2.0")
                        .size(11.0)
                        .color(egui::Color32::from_gray(80)),
                );
            });
        });
    }
}

impl RadarApp {
    /// Kick off the scan in a background Tokio task.
    fn start_scan(&mut self, ctx: egui::Context) {
        let mac = self.mac_input.trim().to_string();
        let ip_range = self.ip_range.trim().to_string();
        let profile = self.profile;
        let password = self.ssh_password.clone();
        let user = self.ssh_user.clone();
        let state = self.scan_state.clone();

        // Mark as scanning.
        *state.lock().unwrap() = ScanState::Scanning;

        // Load the private key from the environment variable.
        let key_env = profile.env_key_name();
        let key_data = match std::env::var(key_env) {
            Ok(k) if !k.is_empty() => k,
            _ => {
                *state.lock().unwrap() = ScanState::Error(format!(
                    "Private key not found in environment variable '{}'.\n\
                     Make sure .env is present and contains {}.",
                    key_env, key_env
                ));
                ctx.request_repaint();
                return;
            }
        };

        let auth = AuthenticationMethod::PrivateKeyMemory {
            key_data,
            passphrase: Some(password),
        };

        let config = SshConfig {
            user,
            port: 22,
            auth,
            timeout: Duration::from_secs(3), // per-host TCP timeout
        };

        // Spawn a background thread with a 15-second overall scan deadline.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            rt.block_on(async {
                let scanner = Scanner::new(config, mac);
                let scan_future = scanner.scan(&ip_range);
                let result = tokio::time::timeout(Duration::from_secs(15), scan_future).await;

                let mut s = state.lock().unwrap();
                match result {
                    Ok(Ok(ip)) => *s = ScanState::Found(ip),
                    Ok(Err(e)) => *s = ScanState::Error(e.to_string()),
                    Err(_) => *s = ScanState::Error("Scan timed out after 15 seconds".into()),
                }
            });
            ctx.request_repaint();
        });
    }
}
