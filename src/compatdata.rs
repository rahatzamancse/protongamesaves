use crate::config::Config;
use crate::IGNORE_DIRS;
use crate::SAVE_PATHS;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;

// Represents a save location for a game
pub struct SaveLocation {
    pub path: PathBuf,
    pub relative_path: String,
    pub entries: Vec<SaveEntry>,
}

// Represents a single save folder entry
pub struct SaveEntry {
    pub name: String,
    pub path: PathBuf,
}

// Represents a Proton prefix
pub struct PrefixData {
    pub game_id: String,
    pub _path: PathBuf,
    pub _drive_c_path: PathBuf,
    pub user_path: PathBuf,
    pub save_locations: Vec<SaveLocation>,
}

impl PrefixData {
    // Create a new PrefixData for a game ID
    pub fn new(config: &Config, game_id: &str) -> Self {
        let prefix_path = config.compatdata_path().join(game_id);
        let drive_c_path = config.drive_c_path(game_id);
        let user_path = config.user_path(game_id);
        
        // Initialize with empty save locations - they'll be populated when needed
        let save_locations = Vec::new();
        
        Self {
            game_id: game_id.to_string(),
            _path: prefix_path,
            _drive_c_path: drive_c_path,
            user_path,
            save_locations,
        }
    }
    
    // Scan for save locations
    pub fn scan_save_locations(&mut self) -> Result<()> {
        self.save_locations.clear();
        
        for &rel_path in SAVE_PATHS.iter() {
            let full_path = self.user_path.join(rel_path);
            
            if full_path.exists() && full_path.is_dir() {
                let mut entries = Vec::new();
                
                // Scan for game-specific folders
                if let Ok(dir_entries) = fs::read_dir(&full_path) {
                    for entry_result in dir_entries {
                        if let Ok(entry) = entry_result {
                            let entry_path = entry.path();
                            let file_name = entry.file_name();
                            let name = file_name.to_string_lossy().to_string();
                            
                            if entry_path.is_dir() && !IGNORE_DIRS.contains(name.as_str()) {
                                entries.push(SaveEntry {
                                    name,
                                    path: entry_path,
                                });
                            }
                        }
                    }
                }
                
                // Sort entries by name
                entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                
                self.save_locations.push(SaveLocation {
                    path: full_path,
                    relative_path: rel_path.to_string(),
                    entries,
                });
            }
        }
        
        Ok(())
    }
    
    // Delete the entire prefix directory
    pub fn _delete(&self) -> Result<()> {
        let prefix_path = &self._path; // Use the prefixed field
        println!("Attempting to delete prefix directory: {}", prefix_path.display());
        
        if !prefix_path.exists() {
            return Err(anyhow!("Prefix path does not exist"));
        }
        
        fs::remove_dir_all(prefix_path)?;
        Ok(())
    }
}

// Get all game IDs from the compatdata directory
pub fn list_game_ids(config: &Config) -> Result<Vec<String>> {
    let compatdata_path = config.compatdata_path();
    
    if !compatdata_path.exists() {
        return Err(anyhow!("Compatdata path does not exist"));
    }
    
    let mut game_ids = Vec::new();
    
    if let Ok(entries) = fs::read_dir(compatdata_path) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                let path = entry.path();
                
                if path.is_dir() {
                    if let Some(game_id) = path.file_name() {
                        let game_id = game_id.to_string_lossy().to_string();
                        
                        // Check if this is a valid prefix (has pfx directory)
                        if path.join("pfx").exists() {
                            game_ids.push(game_id);
                        }
                    }
                }
            }
        }
    }
    
    // Sort game IDs numerically if possible
    game_ids.sort_by(|a, b| {
        // Try to convert to integers for numerical sorting
        match (a.parse::<u64>(), b.parse::<u64>()) {
            (Ok(a_num), Ok(b_num)) => a_num.cmp(&b_num),
            _ => a.cmp(b), // Fall back to string comparison
        }
    });
    
    Ok(game_ids)
}

// Open a path in the default file manager
pub fn open_in_file_manager(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("Path does not exist"));
    }
    
    Command::new("xdg-open")
        .arg(path)
        .spawn()?;
    
    Ok(())
} 