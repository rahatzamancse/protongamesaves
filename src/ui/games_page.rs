use adw::prelude::*;
use adw::{ActionRow, ExpanderRow}; // Import Adwaita widgets
use gtk::{
    gio, glib, Align, Box, Button, Label, ListBox, Orientation, PolicyType, ScrolledWindow,
    SelectionMode, SearchEntry,
}; // Import Button and SearchEntry
use humansize::{format_size, DECIMAL}; // For formatting size
use std::cell::RefCell;
use std::path::PathBuf; // Import PathBuf
use std::process::Command;
use std::rc::Rc;
use std::{collections::HashMap, fs}; // For storing game data & fs operations
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::compatdata::PrefixData; // Import PrefixData
use crate::config::Config;
use crate::manifest::{self, ManifestData}; // Import manifest structs

// Structure to hold combined game information
#[derive(Clone)] // Needed for potential sorting/filtering
pub struct GameInfo {
    pub app_id: String,
    pub name: String,
    pub entry: manifest::GameEntry, // Store the full entry for details
    pub save_locations: Vec<SaveLocationInfo>, // Store resolved/found locations
    pub total_size_bytes: u64,      // Store calculated size
}

// Structure to hold info about a specific save location for a game
#[derive(Clone, Debug)]
pub struct SaveLocationInfo {
    pub manifest_path: String,     // The original path string from the manifest
    pub resolved_path: PathBuf,    // The path resolved for the specific prefix
    pub size_bytes: u64,           // Size of this specific location
    pub exists: bool,              // Does the resolved path exist?
    pub tags: Option<Vec<String>>, // Tags from the manifest rule
}

pub struct GamesPage {
    widget: Box,
    list_container: ListBox, // Change from Box to ListBox for consistent styling
    config: Rc<RefCell<Config>>,
    // Store the parsed manifest data
    manifest_data: Option<Rc<ManifestData>>,
    // Store the combined game info, keyed by app_id for easy lookup
    games: Rc<RefCell<HashMap<String, GameInfo>>>,
    search_entry: SearchEntry,
    matcher: Rc<SkimMatcherV2>,
}

impl GamesPage {
    pub fn new(config: Rc<RefCell<Config>>) -> Self {
        let container = Box::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        // Add title and description like in compatdata_page
        let header = Label::new(Some("Proton Game Saves"));
        header.add_css_class("title-1");
        container.append(&header);
        
        let description = Label::new(Some("View and manage your Steam game save files"));
        description.add_css_class("subtitle-1");
        container.append(&description);

        // Add search entry
        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("🔍 Search Games or App IDs..."));
        search_entry.set_margin_top(12);
        search_entry.set_margin_bottom(6);
        search_entry.add_css_class("emoji");
        container.append(&search_entry);

        // Match the ScrolledWindow setup from compatdata_page
        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_hexpand(true);
        container.append(&scroll);
        
        // Create ListBox exactly like in compatdata_page
        let list_container = ListBox::new();
        list_container.set_selection_mode(SelectionMode::None);
        list_container.add_css_class("boxed-list");
        scroll.set_child(Some(&list_container));

        let matcher = Rc::new(SkimMatcherV2::default());

        let page = Self {
            widget: container,
            list_container,
            config,
            manifest_data: None, // Initially no manifest loaded
            games: Rc::new(RefCell::new(HashMap::new())), // Initialize empty games map
            search_entry: search_entry.clone(),
            matcher,
        };

        // Connect search signal for filtering
        let list_container_clone = page.list_container.clone();
        let matcher_clone = page.matcher.clone();
        page.search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_lowercase();
            Self::filter_game_list(&list_container_clone, &matcher_clone, &query);
        });

        page
    }

    // New function to filter the game list based on search query
    fn filter_game_list(container: &ListBox, matcher: &SkimMatcherV2, query: &str) {
        if query.is_empty() {
            // When query is empty, show all items
            let mut row = container.first_child();
            while let Some(child) = row {
                if let Some(widget) = child.downcast_ref::<gtk::Widget>() {
                    widget.set_visible(true);
                }
                row = child.next_sibling();
            }
            return;
        }

        // Check each row against the query
        let mut row = container.first_child();
        while let Some(child) = row {
            let mut visible = false;
            
            if let Some(expander) = child.downcast_ref::<ExpanderRow>() {
                // Get the title text (game name)
                let title = expander.title().to_string().to_lowercase();
                // Get the subtitle text (contains App ID)
                let subtitle = expander.subtitle().to_string().to_lowercase();
                
                // Check if query matches title or subtitle
                visible = matcher.fuzzy_match(&title, query).is_some() || 
                          matcher.fuzzy_match(&subtitle, query).is_some();
            } else {
                // For placeholder items or labels, show them with an empty query
                visible = true;
            }
            
            if let Some(widget) = child.downcast_ref::<gtk::Widget>() {
                widget.set_visible(visible);
            }
            
            row = child.next_sibling();
        }
    }

    pub fn widget(&self) -> &Box {
        &self.widget
    }

    // Method to load/update the manifest data
    pub fn update_manifest(&mut self) {
        match manifest::parse_manifest(&self.config.borrow()) {
            Ok(data) => {
                println!("DEBUG: Manifest parsed successfully."); // Log success
                self.manifest_data = Some(Rc::new(data));
                // After loading, we should refresh the list based on existing compatdata
                self.refresh_game_list();
            }
            Err(e) => {
                eprintln!("DEBUG: Failed to parse manifest: {}", e);
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
            self.refresh_game_list(); // Update UI (shows placeholder)
            return;
        }
        let manifest = self.manifest_data.as_ref().unwrap(); // Safe unwrap due to check above
        let config_borrow = self.config.borrow(); // Borrow config once
        println!(
            "DEBUG: Manifest data is loaded. Number of games in manifest: {}",
            manifest.games.len()
        );

        println!(
            "Populating games list using path matching from {} scanned prefixes...",
            scanned_prefixes.len()
        );

        let mut games_map = self.games.borrow_mut();
        games_map.clear(); // Clear previous entries

        // Iterate through prefixes found by the scan
        for prefix_data in scanned_prefixes {
            // Use the new path-matching function from manifest.rs
            match manifest::find_game_for_prefix_by_path(&manifest, prefix_data, &config_borrow) {
                Some((manifest_game_name, manifest_entry)) => {
                    // Found a matching game entry via path comparison
                    println!(
                        "  Identified game via path match: '{}' for App ID: {}",
                        manifest_game_name, prefix_data.game_id
                    );

                    // Check if we already processed this game ID (less likely but good practice)
                    if games_map.contains_key(&prefix_data.game_id) {
                        println!(
                            "  Skipping already processed App ID: {}",
                            prefix_data.game_id
                        );
                        continue;
                    }

                    // Proceed to calculate size and add game info
                    let mut game_save_locations: Vec<SaveLocationInfo> = Vec::new();
                    let mut total_size: u64 = 0;

                    // Resolve paths defined in the manifest for this game
                    if let Some(files) = &manifest_entry.files {
                        for (manifest_path_str, rule) in files {
                            if let Some(resolved_path) = manifest::resolve_manifest_path(
                                manifest_path_str,
                                &config_borrow,
                                &prefix_data.game_id,
                            ) {
                                // Calculate size for this path
                                let mut current_size: u64 = 0;
                                let exists = resolved_path.exists();
                                if exists {
                                    match Self::calculate_path_size(&resolved_path) {
                                        Ok(size) => current_size = size,
                                        Err(e) => eprintln!(
                                            "Error calculating size for {}: {}",
                                            resolved_path.display(),
                                            e
                                        ),
                                    }
                                }

                                let location_info = SaveLocationInfo {
                                    manifest_path: manifest_path_str.clone(),
                                    resolved_path: resolved_path.clone(),
                                    size_bytes: current_size,
                                    exists,
                                    tags: rule._tags.clone(),
                                };

                                total_size += current_size;
                                game_save_locations.push(location_info);
                            } else {
                                println!(
                                    "  Could not resolve manifest path: {} for game {}",
                                    manifest_path_str, manifest_game_name
                                );
                            }
                        }
                    }

                    // TODO: Consider adding registry paths from manifest_entry._registry if relevant

                    if !game_save_locations.is_empty() {
                        let game_info = GameInfo {
                            app_id: prefix_data.game_id.clone(),
                            name: manifest_game_name.clone(),
                            entry: manifest_entry.clone(),
                            save_locations: game_save_locations,
                            total_size_bytes: total_size,
                        };
                        games_map.insert(prefix_data.game_id.clone(), game_info);
                    } else {
                        // This case might be less common if path matching requires resolvable paths
                        println!(
                            "  Game '{}' identified, but no resolvable save locations found?",
                            manifest_game_name
                        );
                    }
                }
                None => {
                    // No matching game found via path matching for this prefix
                    println!(
                        "  No game identified via path matching for prefix_id: {}",
                        prefix_data.game_id
                    );
                }
            }
        }

        println!("DEBUG: Finished iterating through all prefixes."); // Add log after loop
                                                                     // Drop the mutable borrow before calling refresh_game_list
        drop(games_map);

        println!(
            "Finished processing prefixes. Found {} games with manifest entries.",
            self.games.borrow().len()
        );
        self.refresh_game_list(); // Update the UI
    }

    // Clears the list container
    fn clear_list(&self) {
        while let Some(child) = self.list_container.first_child() {
            self.list_container.remove(&child);
        }
    }

    // Rebuilds the list container with ExpanderRows from self.games data
    fn refresh_game_list(&self) {
        self.clear_list();
        
        let games_map = self.games.borrow();

        if games_map.is_empty() {
            // Show a placeholder if no games are found or manifest isn't loaded
            let placeholder_box = Box::new(Orientation::Vertical, 10);
            placeholder_box.set_vexpand(true);
            placeholder_box.set_valign(Align::Center);

            let placeholder_icon = gtk::Image::from_icon_name("dialog-information-symbolic");
            placeholder_icon.set_icon_size(gtk::IconSize::Large);
            placeholder_icon.set_margin_bottom(10);

            let placeholder_label = Label::new(Some(if self.manifest_data.is_none() {
                "📋 Manifest Not Loaded" // More consistent styling
            } else {
                "🎮 No Games Found" // More consistent styling
            }));
            placeholder_label.set_wrap(true);
            placeholder_label.set_justify(gtk::Justification::Center);
            placeholder_label.set_css_classes(&["title-4", "emoji"]); // Add emoji class

            let placeholder_sub_label = Label::new(Some(if self.manifest_data.is_none() {
                "Download the manifest in Settings to see game data."
            } else {
                "Scan results did not match any games in the manifest.\nTry refreshing or check Steam directory setting."
            }));
            placeholder_sub_label.set_wrap(true);
            placeholder_sub_label.set_justify(gtk::Justification::Center);
            placeholder_sub_label.set_css_classes(&["dim-label"]);

            placeholder_box.append(&placeholder_icon);
            placeholder_box.append(&placeholder_label);
            placeholder_box.append(&placeholder_sub_label);

            let row = gtk::ListBoxRow::new();
            row.set_selectable(false);
            row.set_child(Some(&placeholder_box));
            self.list_container.append(&row);
            return;
        }

        // Sort games by name for consistent display
        let mut sorted_games: Vec<&GameInfo> = games_map.values().collect();
        sorted_games.sort_by(|a, b| a.name.cmp(&b.name));

        // Create ExpanderRow for each game
        for game_info in sorted_games {
            let total_size_formatted = format_size(game_info.total_size_bytes, DECIMAL);
            let subtitle = format!(
                "App ID: {} | Total Size: {}",
                game_info.app_id, total_size_formatted
            );

            let expander_row = ExpanderRow::builder()
                .title(&format!("🎮 {}", game_info.name))
                .subtitle(&subtitle)
                .show_enable_switch(false)
                .build();
                
            // Add styling for consistent appearance with compatdata_page
            expander_row.add_css_class("activatable");
            expander_row.add_css_class("emoji");
            expander_row.set_margin_top(2);
            expander_row.set_margin_bottom(2);
            
            // Set widget name for search filtering
            let mut searchable_text = format!("{} {}", game_info.name, game_info.app_id);
            for location in &game_info.save_locations {
                if let Some(tags) = &location.tags {
                    for tag in tags {
                        searchable_text.push_str(&format!(" {}", tag));
                    }
                }
                searchable_text.push_str(&format!(" {}", location.manifest_path));
            }
            expander_row.set_widget_name(&searchable_text);

            // --- Create content for the expanded view ---
            let expanded_content_box = Box::new(Orientation::Vertical, 6);
            expanded_content_box.set_margin_start(12);
            expanded_content_box.set_margin_end(12);
            expanded_content_box.set_margin_top(6);
            expanded_content_box.set_margin_bottom(6);
            expander_row.add_row(&expanded_content_box); // Use add_row for nested content

            if game_info.save_locations.is_empty() {
                let no_saves_label =
                    Label::new(Some("🤷 No save locations defined or found"));
                no_saves_label.set_halign(Align::Center);
                no_saves_label.set_css_classes(&["dim-label", "emoji"]);
                no_saves_label.set_margin_top(12);
                no_saves_label.set_margin_bottom(12);
                expanded_content_box.append(&no_saves_label);
            } else {
                // Create a ListBox for the locations within the ExpanderRow
                let location_list_box = ListBox::new();
                location_list_box.set_selection_mode(SelectionMode::None);
                location_list_box.set_css_classes(&["boxed-list", "content-list"]); // Better styling
                location_list_box.set_margin_top(6);
                location_list_box.set_margin_bottom(6);
                expanded_content_box.append(&location_list_box);

                for location in &game_info.save_locations {
                    // Get path and size for the location
                    let path_display = location.resolved_path.display().to_string();
                    let size_formatted = format_size(location.size_bytes, DECIMAL);

                    // --- Create Title ---
                    // Start with tags if available and not empty, otherwise use manifest path
                    let title_base = location.tags
                        .as_ref()
                        .filter(|tags| !tags.is_empty())
                        .map(|tags| tags.join(", "))
                        .unwrap_or_else(|| location.manifest_path.clone());
                    // Append size
                    let final_title = format!("📁 {} ({})", title_base, size_formatted);
                    
                    // --- Create Subtitle (Abbreviated Path) ---
                    let app_id = &game_info.app_id; // Get app_id from the outer loop's game_info
                    let resolved_path = &location.resolved_path;
                    let config_borrow = self.config.borrow(); // Borrow config to get compatdata path
                    let compatdata_base_path = config_borrow.compatdata_path();

                    // Start with full path as fallback
                    let mut subtitle_path_str = path_display.clone();

                    // Try to create a shorter path display - Make it even shorter and more concise
                    if let Ok(stripped_path) = resolved_path.strip_prefix(compatdata_base_path) {
                        subtitle_path_str = format!("📂 [compatdata]/{}", stripped_path.display());
                    } else {
                        // If not in compatdata, just use the last 2-3 components of the path
                        let path_components: Vec<_> = resolved_path.components().collect();
                        if path_components.len() > 2 {
                            let last_components = &path_components[path_components.len() - 2..];
                            let short_path = last_components.iter()
                                .map(|c| c.as_os_str().to_string_lossy())
                                .collect::<Vec<_>>()
                                .join("/");
                            subtitle_path_str = format!("📂 …/{}", short_path);
                        }
                    }

                    // Add "Path not found" to subtitle if needed
                    if !location.exists {
                        subtitle_path_str = format!("{} | ⚠️ Path not found", subtitle_path_str);
                    }
                    
                    // Make subtitle more visible with a prefix
                    if subtitle_path_str.is_empty() {
                        subtitle_path_str = String::from("No path information available");
                    }
                    
                    // Escape special XML characters to prevent markup parsing errors
                    let escaped_subtitle = subtitle_path_str.replace("<", "&lt;").replace(">", "&gt;").replace("&", "&amp;");
                    let short_subtitle = format!("Path: {}", escaped_subtitle);
                    
                    // Create a simple ActionRow first
                    let row = ActionRow::new();
                    
                    // Set properties after creation - sometimes this works better
                    row.set_title(&final_title);
                    row.set_subtitle(&short_subtitle);
                    row.set_subtitle_lines(3);
                    row.add_css_class("activatable");
                    row.add_css_class("emoji");
                    
                    // Add dim-label class if path doesn't exist
                    if !location.exists {
                        row.add_css_class("dim-label"); 
                    }
                    
                    // Add "Open Folder" button if the path exists
                    if location.exists {
                        let open_button = Button::from_icon_name("folder-open-symbolic");
                        open_button.set_tooltip_text(Some("Open Folder"));
                        open_button.set_valign(Align::Center);
                        let folder_path = location.resolved_path.clone(); // Clone path for closure
                        open_button.connect_clicked(move |_| {
                            match Self::open_folder(&folder_path) {
                                Ok(_) => println!("Opened folder: {}", folder_path.display()),
                                Err(e) => eprintln!(
                                    "Failed to open folder {}: {}",
                                    folder_path.display(),
                                    e
                                ),
                            }
                        });
                        row.add_suffix(&open_button);
                        row.set_activatable_widget(Some(&open_button)); // Allow activating row clicks button
                    } else { 
                        // Add icon to indicate path not found
                        let warning_icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
                        warning_icon.set_tooltip_text(Some("Path not found"));
                        warning_icon.set_valign(Align::Center);
                        warning_icon.add_css_class("warning");
                        row.add_suffix(&warning_icon);
                   }

                    location_list_box.append(&row);
                }
            }

            self.list_container.append(&expander_row);
        }

        println!("Games list UI refreshed with ExpanderRows.");
    }

    // Helper function to calculate directory size
    fn calculate_path_size(path: &PathBuf) -> Result<u64, std::io::Error> {
        let mut total_size = 0;
        if path.is_file() {
            total_size = fs::metadata(path)?.len();
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();
                if entry_path.is_file() {
                    // Check if it's a symlink before getting metadata
                    if !entry_path.is_symlink() {
                        match fs::metadata(&entry_path) {
                            Ok(metadata) => total_size += metadata.len(),
                            Err(e) => eprintln!(
                                "Failed to get metadata for file {}: {}",
                                entry_path.display(),
                                e
                            ), // Log error but continue
                        }
                    }
                } else if entry_path.is_dir() {
                    // Avoid infinite loops with symlinks, and recursively sum size
                    if !entry_path.is_symlink() {
                        match Self::calculate_path_size(&entry_path) {
                            Ok(subdir_size) => total_size += subdir_size,
                            Err(e) => eprintln!(
                                "Failed to get size for subdir {}: {}",
                                entry_path.display(),
                                e
                            ), // Log error but continue
                        }
                    }
                }
            }
        }
        Ok(total_size)
    }

    // Helper function to open a folder in the default file manager
    fn open_folder(path: &PathBuf) -> Result<(), glib::Error> {
        if !path.exists() {
            return Err(glib::Error::new(
                gio::IOErrorEnum::NotFound,
                &format!("Path does not exist: {}", path.display()),
            ));
        }
        // Use xdg-open on Linux. Needs platform-specific handling for others.
        #[cfg(target_os = "linux")]
        {
            let status = Command::new("xdg-open").arg(path).status().map_err(|e| {
                glib::Error::new(
                    gio::IOErrorEnum::Failed, // Use a generic IO error type
                    &format!("Failed to execute xdg-open for {}: {}", path.display(), e),
                )
            })?; // Map the error here

            if !status.success() {
                return Err(glib::Error::new(
                    gio::IOErrorEnum::Failed,
                    &format!(
                        "xdg-open command failed for {} with status: {:?}",
                        path.display(),
                        status.code()
                    ),
                ));
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            eprintln!("Warning: Opening folders is currently only implemented for Linux.");
            // Placeholder for other OS implementations (e.g., using `open` on macOS, `explorer` on Windows)
            return Err(glib::Error::new(
                gio::IOErrorEnum::NotSupported, // Indicate it's not supported
                "Folder opening not supported on this OS",
            ));
        }

        Ok(())
    }

    // Public refresh method maybe needed later if triggered externally
    // pub fn refresh(&mut self) {
    //     self.update_manifest(); // Reload manifest
    //     // How do we get the latest compatdata dirs here? Needs coordination.
    // }
}

// Placeholder for the game row widget later
// mod game_row { ... }
