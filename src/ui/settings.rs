use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow, MessageDialog, EntryRow};
use gtk::{Button, glib, Align, FileDialog, Window, gio};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::Config;
use crate::manifest;

// Callback type for when the manifest needs updating
type OnManifestUpdate = Rc<RefCell<dyn FnMut()>>; // Use Rc<RefCell<dyn FnMut>> for flexibility

pub struct SettingsDialog {
    dialog: PreferencesWindow,
    _config: Rc<RefCell<Config>>,
    // Callback to trigger updates in the main window
    _on_update: OnManifestUpdate,
}

impl SettingsDialog {
    pub fn new(parent: adw::ApplicationWindow, config: Rc<RefCell<Config>>, on_update: OnManifestUpdate) -> Self {
        let dialog = PreferencesWindow::builder()
            .transient_for(&parent)
            .title("Settings")
            .modal(true)
            .build();
            
        let page = PreferencesPage::new();
        dialog.add(&page);

        // --- Steam Settings Group --- 
        let steam_group = PreferencesGroup::builder()
            .title("Steam Settings")
            .description("Configure Steam directory locations")
            .build();
        page.add(&steam_group);
            
        let path_row = ActionRow::builder()
            .title("Steam Directory")
            .subtitle(&*config.borrow().steam_path().to_string_lossy())
            .build();
        let browse_button = Button::with_label("Browse");
        browse_button.set_valign(Align::Center);
        path_row.add_suffix(&browse_button);
        let dialog_clone = dialog.clone(); // Clone for closure
        let config_clone = config.clone(); 
        let path_row_clone = path_row.clone();
        browse_button.connect_clicked(move |_| {
            // Use spawn_local for the async file dialog operation
            let config_clone_inner = config_clone.clone();
            let path_row_clone_inner = path_row_clone.clone();
            let parent_window = dialog_clone.clone().upcast::<Window>(); // Need Window for dialog parent
            glib::MainContext::default().spawn_local(async move {
                Self::show_steam_folder_chooser_async(parent_window, config_clone_inner, path_row_clone_inner).await;
            });
        });
        steam_group.add(&path_row);

        // --- Manifest Settings Group --- 
        let manifest_group = PreferencesGroup::builder()
            .title("Game Data Manifest")
            .description("Configure the source for game save/config definitions (Ludusavi format)")
            .build();
        page.add(&manifest_group);

        let url_row = EntryRow::builder()
            .title("Manifest URL")
            .text(config.borrow().manifest_url())
            .show_apply_button(true)
            .build();
        let config_clone_url = config.clone();
        url_row.connect_apply(move |row| {
             if let Err(e) = config_clone_url.borrow_mut().set_manifest_url(row.text().to_string()) {
                 eprintln!("Error setting manifest URL: {}", e); // Handle error display better
                 // TODO: Show an error message dialog
             }
        });
        manifest_group.add(&url_row);
        
        let update_row = ActionRow::builder()
            .title("Update Manifest Now")
            .subtitle(&format!("Cached at: {}", config.borrow().manifest_cache_path().display()))
            .build();
        let update_button = Button::with_label("Download/Update");
        update_button.set_valign(Align::Center);
        update_row.add_suffix(&update_button);
        let config_clone_update = config.clone();
        let dialog_clone_update = dialog.clone();
        let update_row_clone = update_row.clone();
        update_button.connect_clicked(move |_| {
            match manifest::download_manifest(&config_clone_update.borrow()) {
                Ok(_) => {
                    println!("Manifest downloaded successfully.");
                    // Update subtitle on success
                    update_row_clone.set_subtitle(&format!("Cached at: {}", config_clone_update.borrow().manifest_cache_path().display()));
                     // Optionally show success message
                    let success_dialog = MessageDialog::builder()
                         .transient_for(&dialog_clone_update)
                         .heading("Manifest Updated")
                         .body("Successfully downloaded the latest manifest.")
                         .build();
                     success_dialog.add_response("ok", "OK");
                     success_dialog.present();
                }
                Err(e) => {
                     eprintln!("Error downloading manifest: {}", e);
                    // Show error message
                    let error_dialog = MessageDialog::builder()
                         .transient_for(&dialog_clone_update)
                         .heading("Error Updating Manifest")
                         .body(&format!("Failed to download manifest: {}\n\nCheck the URL and your internet connection.", e))
                         .build();
                     error_dialog.add_response("ok", "OK");
                     error_dialog.present();
                }
            }
        });
        manifest_group.add(&update_row);
        
        Self { dialog, _config: config, _on_update: on_update }
    }
    
    pub fn present(&self) {
        self.dialog.present();
    }
    
    // Renamed for clarity and made async helper
    async fn show_steam_folder_chooser_async(parent: Window, config: Rc<RefCell<Config>>, row: ActionRow) {
        let file_dialog = FileDialog::new();
        file_dialog.set_title("Select Steam Directory");
        // FileChooserAction::SelectFolder isn't directly used, instead use appropriate method

        // Use select_folder_future
        match file_dialog.select_folder_future(Some(&parent)).await {
            Ok(folder) => { // Directly get the folder on Ok
                if let Some(path) = folder.path() {
                    println!("Selected folder: {}", path.display());
                    if let Err(e) = config.borrow_mut().set_steam_path(path.clone()) {
                         eprintln!("Error setting steam path: {}", e);
                         Self::show_error_dialog_transient(&parent, "Error Setting Path", &format!("Failed to set Steam path: {}", e));
                    } else {
                        row.set_subtitle(&path.to_string_lossy());
                    }
                }
            },
            Err(e) => {
                // Check if the error is due to user cancellation
                if e.kind::<gio::IOErrorEnum>() == Some(gio::IOErrorEnum::Cancelled) {
                     println!("Folder selection cancelled.");
                } else {
                    eprintln!("Error selecting folder: {}", e);
                    Self::show_error_dialog_transient(&parent, "Selection Error", &format!("Failed to select folder: {}", e));
                }
            }
        }
    }
    
    // Helper to show error dialog, requires parent window
    fn show_error_dialog_transient(parent: &impl IsA<Window>, title: &str, message: &str) {
        // Ensure this runs on the main thread if called from async context
        // glib::MainContext::default().spawn_local might be needed if calling from non-main thread
        let dialog = MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .heading(title)
            .body(message)
            .build();
        dialog.add_response("ok", "OK");
        dialog.present();
    }
} 