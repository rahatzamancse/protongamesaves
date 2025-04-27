use adw::prelude::*;
use gtk::glib;
use once_cell::sync::Lazy;
use std::collections::HashSet;

// Constants for ignored directories and save paths
static IGNORE_DIRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "Microsoft",
        "Temp",
        "Packages",
        "ConnectedDevicesPlatform",
        "Comms",
        "Apps",
    ])
});

static SAVE_PATHS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "AppData/Local",
        "AppData/LocalLow",
        "AppData/Roaming",
        "Saved Games",
    ]
});

// Import our application modules
mod ui;
mod compatdata;
mod config;
mod manifest;
mod styles;

fn main() -> glib::ExitCode {
    // Initialize GTK
    adw::init().expect("Failed to initialize libadwaita");
    
    // Load application CSS
    styles::load_app_css();
    
    // Create a new application
    let app = adw::Application::builder()
        .application_id("com.github.proton.gamesaves")
        .build();
        
    // Connect to the activate signal
    app.connect_activate(|app| {
        // Create the main window
        let window = ui::window::ProtonSavesWindow::new(app);
        window.present();
    });
    
    // Run the application
    app.run()
} 