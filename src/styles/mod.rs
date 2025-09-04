use gtk::CssProvider;
use std::path::Path;
use std::fs;

pub fn load_app_css() {
    // Create a new CSS provider
    let provider = CssProvider::new();
    
    // Try to load from file
    let css_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/styles/app.css");
    
    if css_file.exists() {
        match fs::read_to_string(&css_file) {
            Ok(css_content) => {
                provider.load_from_data(&css_content);
                
                // Add the provider to the display using the new API
                gtk::style_context_add_provider_for_display(
                    &gtk::gdk::Display::default().expect("Could not get default display"),
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
                
                println!("Loaded CSS from: {}", css_file.display());
            },
            Err(e) => {
                eprintln!("Failed to read CSS file {}: {}", css_file.display(), e);
                load_fallback_css();
            }
        }
    } else {
        eprintln!("CSS file not found: {}", css_file.display());
        load_fallback_css();
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