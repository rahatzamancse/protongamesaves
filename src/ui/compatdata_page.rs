use adw::prelude::*;
use adw::{ExpanderRow, MessageDialog, ActionRow};
use gtk::{
    Box, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, 
    SelectionMode, Align, SearchEntry // Import Accessible trait itself
};
 
use gtk;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::HashMap;
use anyhow::{Result, anyhow}; // Import anyhow

use crate::compatdata::{self, PrefixData};
use crate::config::Config;

pub struct CompatDataPage {
    widget: Box,
    window: adw::ApplicationWindow,
    config: Rc<RefCell<Config>>,
    listbox: ListBox, // Keep using ListBox directly
    search_entry: SearchEntry,
    matcher: Rc<SkimMatcherV2>,
    // Store detected directories (AppID -> Path)
    _detected_dirs: Rc<RefCell<HashMap<String, PathBuf>>>, 
}

impl CompatDataPage {
    pub fn new(window: adw::ApplicationWindow, config: Rc<RefCell<Config>>) -> Self {
        // --- Widget Setup --- 
        let widget = Box::new(Orientation::Vertical, 12);
        // Set margins individually
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);
        
        let header = Label::new(Some("Proton Compatdata Folders"));
        header.add_css_class("title-1");
        widget.append(&header);
        let description = Label::new(Some("Manage your Proton prefixes and game save files"));
        description.add_css_class("subtitle-1");
        widget.append(&description);

        let search_entry = SearchEntry::new();
        search_entry.set_placeholder_text(Some("🔍 Search Game IDs or Save Folders..."));
        search_entry.set_margin_top(12);
        search_entry.set_margin_bottom(6);
        widget.append(&search_entry);

        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_hexpand(true); 
        widget.append(&scroll);
        
        let listbox = ListBox::new();
        listbox.set_selection_mode(SelectionMode::None);
        listbox.add_css_class("boxed-list");
        scroll.set_child(Some(&listbox));
        
        let matcher = Rc::new(SkimMatcherV2::default());
        let detected_dirs = Rc::new(RefCell::new(HashMap::new()));

        let page = Self {
            widget,
            window: window.clone(),
            config,
            listbox: listbox.clone(), // Clone for struct
            search_entry: search_entry.clone(),
            matcher,
            _detected_dirs: detected_dirs.clone(),
        };
        
        // --- Connect Search Signal for Manual Filtering ---
        let listbox_clone = page.listbox.clone();
        let matcher_clone = page.matcher.clone();
        page.search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_lowercase();
            Self::filter_listbox(&listbox_clone, &matcher_clone, &query);
        });

        // --- Initial Data Load --- 
        // Defer initial load to the window's refresh button emit_clicked()
        // We don't call scan or update_listbox here anymore.
        
        page
    }
    
    pub fn widget(&self) -> &Box {
        &self.widget
    }

    // Renamed from refresh_listbox_public - now just scans data
    pub fn scan_compatdata(&self) -> Result<Vec<PrefixData>> {
        println!("Scanning compatdata...");
        let config_borrow = self.config.borrow();
        let compatdata_path = config_borrow.compatdata_path();
        let mut scanned_prefixes = Vec::new();

        if !compatdata_path.exists() {
            // Return error instead of modifying UI here
            return Err(anyhow!("Compatdata path does not exist: {}", compatdata_path.display()));
        }
        
        let game_ids = compatdata::list_game_ids(&config_borrow)?;

        if game_ids.is_empty() {
            println!("No Proton prefixes found in {}", compatdata_path.display());
            // Return Ok with empty vec, not an error
            return Ok(scanned_prefixes); 
        }

        println!("Found {} potential prefixes. Scanning for saves...", game_ids.len());
        for game_id in game_ids {
            let mut prefix_data = PrefixData::new(&config_borrow, &game_id);
            // Scan save locations for this prefix
            if let Err(e) = prefix_data.scan_save_locations() {
                 eprintln!("Error scanning saves for game ID {}: {}", game_id, e);
                 // Decide whether to skip this prefix or continue without saves
                 // Let's include it anyway, maybe manifest matching works differently
            }
            scanned_prefixes.push(prefix_data);
        }
        println!("Finished scanning compatdata.");
        Ok(scanned_prefixes)
    }
    
    // New function to update UI from scanned data
    pub fn update_listbox(&self, prefixes: &[PrefixData]) { // Accept slice
         println!("Updating CompatDataPage listbox with {} prefixes...", prefixes.len());
         // Clear existing items
         while let Some(child) = self.listbox.first_child() {
             self.listbox.remove(&child);
         }

         // Clear the internal detected_dirs map (if we still need it?)
         // For now, let's assume it's not the primary source of truth anymore
         // self.detected_dirs.borrow_mut().clear(); 

         if prefixes.is_empty() {
             let placeholder_label = Label::new(Some("No Proton prefixes found.")); // Simpler message
             placeholder_label.set_margin_start(12);
             placeholder_label.set_margin_end(12);
             placeholder_label.set_margin_top(12);
             placeholder_label.set_margin_bottom(12);
             placeholder_label.set_halign(Align::Center); // Center placeholder
             placeholder_label.set_css_classes(&["dim-label"]);
             self.listbox.append(&placeholder_label);
             return;
         }

         let config_borrow = self.config.borrow(); // Borrow once
         for prefix_data in prefixes {
             // Populate the detected_dirs map (maybe still useful?)
             // self.detected_dirs.borrow_mut().insert(prefix_data.game_id.clone(), prefix_data.path.clone());

             // Pass the borrowed config, not the Rc<RefCell>
             let row = Self::create_game_prefix_expander_row(&self.listbox, &config_borrow, &self.window, prefix_data);
             self.listbox.append(&row); 
         }
         println!("CompatDataPage listbox updated.");
    }

    // Filter ListBox children based on search query
    fn filter_listbox(listbox: &ListBox, matcher: &SkimMatcherV2, query: &str) {
        let mut child = listbox.first_child();
        while let Some(row_widget) = child {
            // Get the text from the widget name
            let searchable_text = row_widget.widget_name().to_string(); 
            
            let visible = query.is_empty() || 
                          matcher.fuzzy_match(&searchable_text.to_lowercase(), query).is_some();
            row_widget.set_visible(visible);
            
            child = row_widget.next_sibling();
        }
    }

    // Creates the ExpanderRow and sets its widget name for searching
    fn create_game_prefix_expander_row(listbox: &ListBox, config: &Config, window: &adw::ApplicationWindow, prefix_data: &PrefixData) -> ExpanderRow {
        let game_id = &prefix_data.game_id;
        let mut searchable_text = format!("Game ID: {}", game_id);
        for loc in &prefix_data.save_locations {
            searchable_text.push_str(&format!(" {} ", loc.relative_path));
            for entry in &loc.entries {
                searchable_text.push_str(&format!(" {} ", entry.name));
            }
        }
        
        let expander_row = ExpanderRow::builder()
            .title(format!("🎮 Game ID: {}", game_id))
            .show_enable_switch(false)
            .build();
        
        expander_row.set_widget_name(&searchable_text);

        let drive_c_path = config.drive_c_path(game_id);
        let open_drive_c_button = Button::from_icon_name("folder-open-symbolic");
        open_drive_c_button.set_tooltip_text(Some("Open drive_c Folder"));
        open_drive_c_button.set_valign(Align::Center);
        let drive_c_path_clone = drive_c_path.clone();
        let window_clone = window.clone();
        open_drive_c_button.connect_clicked(move |_| {
            Self::open_file_manager(&window_clone, &drive_c_path_clone);
        });
        expander_row.add_suffix(&open_drive_c_button);
        let delete_button = Button::from_icon_name("user-trash-symbolic");
        delete_button.set_tooltip_text(Some("Delete Prefix"));
        delete_button.add_css_class("destructive-action");
        delete_button.set_valign(Align::Center);
        let prefix_path = config.compatdata_path().join(game_id);
        let game_id_clone = game_id.to_string();
        let prefix_path_clone = prefix_path.clone();
        let window_clone = window.clone();
        let listbox_weak = listbox.downgrade(); 
        let row_weak = expander_row.downgrade(); 
        delete_button.connect_clicked(move |_| {
            if let (Some(row_strong), Some(listbox_strong)) = (row_weak.upgrade(), listbox_weak.upgrade()) {
                 let list_box_row = row_strong.upcast::<ListBoxRow>(); 
                 Self::delete_prefix(&window_clone, &prefix_path_clone, &game_id_clone, &listbox_strong, &list_box_row);
            }
        });
        expander_row.add_suffix(&delete_button);

        // --- Add Save Location Rows Directly to ExpanderRow --- 
        let mut found_any_saves = false;
        for save_loc in &prefix_data.save_locations {
             if !save_loc.entries.is_empty() {
                 found_any_saves = true;
                
                 // Row for the base save location (e.g., AppData/Roaming)
                 let save_loc_row = ActionRow::builder()
                    .title(&save_loc.relative_path)
                    .build();

                let open_button = Button::from_icon_name("document-open-symbolic");
                open_button.set_tooltip_text(Some("Open Location"));
                open_button.set_valign(Align::Center);
                let path_clone = save_loc.path.clone();
                let window_clone = window.clone();
                open_button.connect_clicked(move |_| {
                    Self::open_file_manager(&window_clone, &path_clone);
                });
                save_loc_row.add_suffix(&open_button);
                expander_row.add_row(&save_loc_row); 

                // Rows for the specific game save folders within that location
                for entry in &save_loc.entries {
                    let game_save_row = ActionRow::builder()
                        .title(&entry.name)
                        .css_classes(vec!["compact"]) 
                        .build();
                    game_save_row.set_margin_start(24); 
                    
                    let open_save_button = Button::from_icon_name("document-open-symbolic");
                    open_save_button.set_tooltip_text(Some("Open Save Folder"));
                    open_save_button.set_valign(Align::Center);
                    let entry_path = entry.path.clone();
                    let window_clone = window.clone();
                    open_save_button.connect_clicked(move |_| {
                        Self::open_file_manager(&window_clone, &entry_path);
                    });
                    game_save_row.add_suffix(&open_save_button);
                    expander_row.add_row(&game_save_row); 
                }
            }
        }

        if !found_any_saves {
             let no_saves_label = Label::new(Some("🤷 No known save folders found"));
            no_saves_label.set_halign(Align::Center);
            no_saves_label.set_css_classes(&["dim-label"]);
            no_saves_label.set_margin_top(12);
            no_saves_label.set_margin_bottom(12);
            let placeholder_row = ActionRow::new();
            placeholder_row.set_child(Some(&no_saves_label));
            placeholder_row.set_selectable(false);
            expander_row.add_row(&placeholder_row);
        }
        
        expander_row
    }
    fn open_file_manager(window: &adw::ApplicationWindow, path: &Path) {
        if let Err(err) = compatdata::open_in_file_manager(path) {
            Self::show_error_dialog(window, &format!("Path does not exist: {}", err));
        }
    }
    fn delete_prefix(window: &adw::ApplicationWindow, prefix_path: &Path, game_id: &str, listbox: &ListBox, row: &gtk::ListBoxRow) { 
        let dialog = MessageDialog::builder()
            .transient_for(window)
            .heading(&format!("🗑️ Delete Prefix for Game ID {}?", game_id))
            .body("This will permanently delete the prefix folder and all save files. This action cannot be undone.")
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("delete", "Delete");
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        let window_clone = window.clone();
        let prefix_path_clone = prefix_path.to_path_buf();
        let row_clone = row.clone(); 
        let listbox_clone = listbox.clone(); 
        dialog.connect_response(None, move |dialog, response| {
            if response == "delete" {
                if let Err(err) = std::fs::remove_dir_all(&prefix_path_clone) {
                    Self::show_error_dialog(&window_clone, &format!("Error deleting prefix: {}", err));
                } else {
                    listbox_clone.remove(&row_clone);
                }
            }
            dialog.destroy();
        });
        dialog.present();
    }
     fn show_error_dialog(window: &adw::ApplicationWindow, message: &str) {
        let dialog = MessageDialog::builder()
            .transient_for(window)
            .heading("Error")
            .body(message)
            .build();
        dialog.add_response("ok", "OK");
        dialog.present();
    }
}
