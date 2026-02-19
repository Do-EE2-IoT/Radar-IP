mod cli;
mod errors;
mod gui;
mod scanner;
mod ssh_client;

use gui::RadarApp;

fn main() -> eframe::Result {
    // Load .env file (silently ignore if missing).
    let _ = dotenvy::dotenv();

    // Initialize logging.
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([480.0, 480.0])
            .with_min_inner_size([400.0, 400.0])
            .with_title("Radar-IP Scanner"),
        ..Default::default()
    };

    eframe::run_native(
        "Radar-IP",
        options,
        Box::new(|cc| Ok(Box::new(RadarApp::new(cc)))),
    )
}