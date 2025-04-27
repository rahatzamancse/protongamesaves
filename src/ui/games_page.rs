use adw::prelude::*;
use gtk::{glib, Align, Box, Label, ListBox, Orientation, PolicyType, ScrolledWindow, SelectionMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap; // For storing game data
use std::path::PathBuf; // Import PathBuf

use crate::compatdata::PrefixData; // Import PrefixData
use crate::config::Config;
use crate::manifest::{self, ManifestData, GameEntry}; // Import manifest structs

// Structure to hold combined game information
#[derive(Clone)] // Needed for potential sorting/filtering
pub struct GameInfo {
    pub app_id: String,
    pub name: String,
    pub entry: manifest::GameEntry, // Store the full entry for details
    // pub save_paths: Vec<PathBuf>, // We'll add detected save paths later
}

pub struct GamesPage {
    widget: Box,
    list_box: ListBox,
    config: Rc<RefCell<Config>>,
    // Store the parsed manifest data
    manifest_data: Option<Rc<ManifestData>>, 
    // Store the combined game info, keyed by app_id for easy lookup
    games: Rc<RefCell<HashMap<String, GameInfo>>>, 
}

impl GamesPage {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        let container = Box::new(Orientation::Vertical, 0);

        let scrolled_window = ScrolledWindow::new();
        scrolled_window.set_policy(PolicyType::Never, PolicyType::Automatic);
        scrolled_window.set_vexpand(true);

        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::None); // No selection needed for now
        list_box.set_css_classes(&["boxed-list"]); // Use Adwaita style

        scrolled_window.set_child(Some(&list_box));
        container.append(&scrolled_window);

        Self {
            widget: container,
            list_box,
            config,
            manifest_data: None, // Initially no manifest loaded
            games: Rc::new(RefCell::new(HashMap::new())), // Initialize empty games map
        }
    }

    pub fn widget(&self) -> &Box {
        &self.widget
    }

    // Method to load/update the manifest data
    pub fn update_manifest(&mut self) {
        match manifest::parse_manifest(&self.config.borrow()) {
            Ok(data) => {
                 println!("Manifest parsed successfully."); // Log success
                 self.manifest_data = Some(Rc::new(data));
                 // After loading, we should refresh the list based on existing compatdata
                 self.refresh_game_list(); 
            },
            Err(e) => {
                 eprintln!("Failed to parse manifest: {}", e);
                 // Optionally show an error message to the user
                 self.manifest_data = None; // Clear manifest data on error
                 self.refresh_game_list(); // Refresh list (will be empty)
            }
        }
    }

    // Method to populate the list - Updated signature and logic
    pub fn populate_games(&self, scanned_prefixes: &[PrefixData]) {
        if self.manifest_data.is_none() {
             println!("Manifest not loaded, cannot populate games list.");
             self.clear_list(); // Clear the list if manifest isn't loaded
             return;
        }
        let manifest = self.manifest_data.as_ref().unwrap(); // Safe unwrap due to check above
        let config_borrow = self.config.borrow(); // Borrow config once
        
        println!("Populating games list using path matching from {} scanned prefixes...", scanned_prefixes.len());

        let mut games_map = self.games.borrow_mut();
        games_map.clear(); // Clear previous entries

        for prefix_data in scanned_prefixes {
            // Check if we already identified this game ID via another path match
            if games_map.contains_key(&prefix_data.game_id) {
                continue;
            }

            let mut game_identified_for_prefix = false;
            for save_loc in &prefix_data.save_locations {
                for entry in &save_loc.entries {
                    let found_path = &entry.path; // Path found on disk

                    // Now, iterate through the manifest to see if this path matches any rule
                    for (manifest_game_name, manifest_entry) in &manifest.games {
                        if let Some(files) = &manifest_entry.files {
                            for manifest_path_str in files.keys() {
                                // Resolve path and check existence
                                if let Some(resolved_manifest_path) = manifest::resolve_manifest_path(
                                    manifest_path_str,
                                    &config_borrow,
                                    &prefix_data.game_id
                                ) {
                                    // Matching Logic:
                                    // 1. Normalize both paths (e.g., remove trailing slashes, lowercase)
                                    let normalized_found = found_path.as_path().to_string_lossy().trim_end_matches('/').to_lowercase();
                                    let normalized_manifest = resolved_manifest_path.as_path().to_string_lossy().trim_end_matches('/').to_lowercase();
                                    
                                    // if found_path == &resolved_manifest_path { // Old exact match
                                    // if normalized_found.starts_with(&normalized_manifest) { // Old starts_with
                                    // New logic: Check if the resolved manifest path starts with the found path
                                    if normalized_manifest.starts_with(&normalized_found) { 
                                         
                                         let game_info = GameInfo {
                                             app_id: prefix_data.game_id.clone(),
                                             name: manifest_game_name.clone(),
                                             entry: manifest_entry.clone(), 
                                         };
                                         games_map.insert(prefix_data.game_id.clone(), game_info);
                                         game_identified_for_prefix = true;
                                         break; // Stop checking manifest paths for this found_path
                                    }
                                }
                            }
                        }
                        if game_identified_for_prefix { break; } // Stop checking manifest games for this found_path
                    }
                }
                 if game_identified_for_prefix { break; } // Stop checking save entries for this prefix
            }
             // if game_identified_for_prefix { break; } // Stop checking save locations for this prefix
        }
        
        // Drop the mutable borrow before calling refresh_game_list
        drop(games_map); 

        println!("Found {} games via path matching.", self.games.borrow().len());
        self.refresh_game_list(); // Update the UI
    }

    // Clears the listbox
    fn clear_list(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
    }

    // Rebuilds the listbox from the self.games data
    fn refresh_game_list(&self) {
        self.clear_list();
        let games_map = self.games.borrow();

        if games_map.is_empty() {
            // Show a placeholder if no games are found or manifest isn't loaded
            let placeholder = Label::new(Some(if self.manifest_data.is_none() {
                "Manifest not loaded. Download it in Settings."
            } else {
                "No games with recognized save data found.\nRefresh after playing more games or check Steam directory setting."
            }));
            placeholder.set_halign(Align::Center);
            placeholder.set_valign(Align::Center);
            placeholder.set_vexpand(true);
            placeholder.set_css_classes(&["dim-label"]);
            self.list_box.append(&placeholder);
            return;
        }

        // Sort games by name for consistent display
        let mut sorted_games: Vec<&GameInfo> = games_map.values().collect();
        sorted_games.sort_by(|a, b| a.name.cmp(&b.name));


        for game_info in sorted_games {
            // TODO: Create a custom row widget for each game
            let row_label = Label::new(Some(&format!("{} (App ID: {})", game_info.name, game_info.app_id)));
            row_label.set_halign(Align::Start);
            self.list_box.append(&row_label);
        }
        
        println!("Games list UI refreshed.");
    }
    
    // Public refresh method maybe needed later if triggered externally
    // pub fn refresh(&mut self) {
    //     self.update_manifest(); // Reload manifest
    //     // How do we get the latest compatdata dirs here? Needs coordination.
    // }
}

// Placeholder for the game row widget later
// mod game_row { ... } 