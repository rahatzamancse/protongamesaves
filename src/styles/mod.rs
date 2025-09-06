use gtk::CssProvider;
use std::path::Path;
use std::fs;

pub fn load_app_css() {
    // Create a new CSS provider
    let provider = CssProvider::new();

    // Try sources in order:
    // 1) Development path (cargo run)
    // 2) Flatpak install path
    // 3) Embedded fallback
    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/styles/app.css");
    let flatpak_path = Path::new("/app/share/proton-game-saves/styles/app.css");

    let css_content_result = if dev_path.exists() {
        fs::read_to_string(&dev_path).map_err(|e| (dev_path.display().to_string(), e))
    } else if flatpak_path.exists() {
        fs::read_to_string(&flatpak_path).map_err(|e| (flatpak_path.display().to_string(), e))
    } else {
        Err((dev_path.display().to_string(), std::io::Error::from(std::io::ErrorKind::NotFound)))
    };

    match css_content_result {
        Ok(css_content) => {
            provider.load_from_data(&css_content);
            gtk::style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Could not get default display"),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            if dev_path.exists() {
                println!("Loaded CSS from: {}", dev_path.display());
            } else {
                println!("Loaded CSS from: {}", flatpak_path.display());
            }
        }
        Err((path, e)) => {
            eprintln!("Failed to read CSS file {}: {}", path, e);
            load_fallback_css();
        }
    }
}

fn load_fallback_css() {
    // Create a new CSS provider
    let provider = CssProvider::new();
    
    // Default CSS as fallback
    provider.load_from_data(include_str!("app.css"));
    
    // Add the provider to the display
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    
    println!("Loaded fallback CSS");
} 