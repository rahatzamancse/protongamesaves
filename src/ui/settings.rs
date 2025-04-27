use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesPage, PreferencesWindow, MessageDialog, EntryRow};
use gtk::{Button, FileChooserAction, FileChooserDialog, ResponseType, gio, glib, Align, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::Config;
use crate::manifest;

// Callback type for when the manifest needs updating
type OnManifestUpdate = Rc<RefCell<dyn FnMut()>>; // Use Rc<RefCell<dyn FnMut>> for flexibility

pub struct SettingsDialog {
    dialog: PreferencesWindow,
    // Keep config ref to update manifest URL
    config: Rc<RefCell<Config>>,
    // Callback to trigger updates in the main window
    on_update: OnManifestUpdate,
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
            Self::show_steam_folder_chooser(&dialog_clone, config_clone.clone(), path_row_clone.clone());
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
        
        Self { dialog, config, on_update }
    }
    
    pub fn present(&self) {
        self.dialog.present();
    }
    
    // Renamed for clarity
    fn show_steam_folder_chooser(parent: &PreferencesWindow, config: Rc<RefCell<Config>>, row: ActionRow) {
        let file_dialog = FileChooserDialog::new(
            Some("Select Steam Directory"),
            Some(parent),
            FileChooserAction::SelectFolder,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );
        
        let current_path = config.borrow().steam_path().to_path_buf();
        if current_path.exists() {
             let _ = file_dialog.set_current_folder(Some(&gio::File::for_path(current_path))); // Ignore result
        }
        
        file_dialog.connect_response(glib::clone!(@strong config, @weak row => move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        if let Err(e) = config.borrow_mut().set_steam_path(path.clone()) {
                             eprintln!("Error setting steam path: {}", e);
                             // TODO: Show error dialog
                        } else {
                            row.set_subtitle(&path.to_string_lossy());
                        }
                    }
                }
            }
            dialog.destroy();
        }));
        
        file_dialog.present();
    }
} 