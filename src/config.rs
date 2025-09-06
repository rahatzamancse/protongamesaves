use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, anyhow};
use dirs;
use serde::{Serialize, Deserialize};

const DEFAULT_MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

// Use serde for easier loading/saving
#[derive(Serialize, Deserialize)] 
pub struct ConfigData {
    steam_path: PathBuf,
    manifest_url: String,
    first_run: bool,
}

pub struct Config {
    data: ConfigData,
    config_path: PathBuf,
    cache_path: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config")) // Fallback
            .join("proton_game_saves");
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache")) // Fallback
            .join("proton_game_saves");

        let config_path = config_dir.join("config.json");
        let cache_path = cache_dir.join("manifest.yaml");

        // Ensure directories exist
        let _ = fs::create_dir_all(&config_dir);
        let _ = fs::create_dir_all(&cache_dir);

        // Load or create default config data
        let data = Self::load_config_data(&config_path).unwrap_or_else(|| {
            let default_steam_path = dirs::home_dir()
                .map(|home| home.join(".steam"))
                .unwrap_or_else(|| PathBuf::from("."));
            ConfigData {
                steam_path: default_steam_path,
                manifest_url: DEFAULT_MANIFEST_URL.to_string(),
                first_run: true,
            }
        });
        
        // Create the Config struct
        let config = Self {
            data,
            config_path,
            cache_path,
        };

        // Save immediately if it was newly created
        if Self::load_config_data(&config.config_path).is_none() {
            let _ = config.save_config();
        }

        config
    }

    // --- Path Getters ---
    pub fn steam_path(&self) -> &Path {
        &self.data.steam_path
    }
    pub fn manifest_url(&self) -> &str {
        &self.data.manifest_url
    }
    pub fn manifest_cache_path(&self) -> &Path {
        &self.cache_path
    }
    pub fn is_first_run(&self) -> bool {
        self.data.first_run
    }
    pub fn compatdata_path(&self) -> PathBuf {
        self.data.steam_path.join("steam/steamapps/compatdata")
    }
    pub fn drive_c_path(&self, game_id: &str) -> PathBuf {
        self.compatdata_path()
            .join(game_id)
            .join("pfx/drive_c")
    }
    pub fn user_path(&self, game_id: &str) -> PathBuf {
        self.drive_c_path(game_id)
            .join("users/steamuser")
    }

    // --- Setters that save --- 
    pub fn set_steam_path(&mut self, path: PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(anyhow!("Steam path does not exist"));
        }
        self.data.steam_path = path;
        self.save_config()
    }
    pub fn set_manifest_url(&mut self, url: String) -> Result<()> {
        // Basic validation (could be more robust)
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("Invalid URL format"));
        }
        self.data.manifest_url = url;
        self.save_config()
    }
    pub fn mark_first_run_complete(&mut self) -> Result<()> {
        self.data.first_run = false;
        self.save_config()
    }

    // --- Load/Save Logic --- 
    fn load_config_data(path: &Path) -> Option<ConfigData> {
        if !path.exists() {
            return None;
        }
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    }
    fn save_config(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }
} 