use crate::config::Config;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_yaml;
use std::collections::HashMap;
use std::fs; // Explicitly import serde_yaml
use std::path::PathBuf; // Ensure Path and PathBuf are imported
use crate::compatdata::PrefixData; // Need PrefixData for the new function

// --- Enums based on schema (can be expanded) ---
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Linux,
    Mac,
    Dos,
    // Add others if needed
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Store {
    Steam,
    Gog,
    Epic,
    Origin,
    Uplay,
    Microsoft,
    // Add others if needed
}

// --- Constraint Structs ---
#[derive(Debug, Deserialize, Clone)]
pub struct FileConstraint {
    pub _os: Option<Os>,
    pub _store: Option<Store>,
}

// --- ID Structs ---
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameSteamInfo {
    pub _id: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameGogInfo {
    pub _id: u32,
}

#[derive(Debug, Deserialize, Clone)] // New struct for nested IDs
#[serde(rename_all = "camelCase")]
pub struct IdField {
    pub _flatpak: Option<String>,
    pub _gog_extra: Option<Vec<u32>>,
    pub _lutris: Option<String>,
    pub _steam_extra: Option<Vec<u32>>,
}

// --- Main Manifest Structs ---
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameFileRule {
    // Removed incorrect 'path' field
    pub _tags: Option<Vec<String>>,
    pub _when: Option<Vec<FileConstraint>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GameEntry {
    pub files: Option<HashMap<String, GameFileRule>>, // Key is the path string
    #[serde(rename = "installDir")]
    pub _install_dir: Option<HashMap<String, serde_yaml::Value>>,
    pub _launch: Option<HashMap<String, serde_yaml::Value>>,
    pub _registry: Option<HashMap<String, serde_yaml::Value>>, // Added registry
    pub _steam: Option<GameSteamInfo>,
    pub _gog: Option<GameGogInfo>, // Added GOG info
    pub _id: Option<IdField>,      // Added nested ID field
    // Removed top-level steam_extra - it's now inside 'id'
    pub _alias: Option<String>,                 // Added alias
    pub _cloud: Option<HashMap<String, bool>>,  // Added cloud info
    pub _notes: Option<Vec<serde_yaml::Value>>, // Added notes
}

#[derive(Debug, Deserialize)]
pub struct ManifestData {
    #[serde(flatten)]
    pub games: HashMap<String, GameEntry>, // Map game name to its entry
}

pub fn download_manifest(config: &Config) -> Result<()> {
    let url = config.manifest_url();
    let cache_path = config.manifest_cache_path();

    println!(
        "Downloading manifest from {} to {}",
        url,
        cache_path.display()
    );

    let response =
        reqwest::blocking::get(url).context(format!("Failed to send request to {}", url))?;

    if !response.status().is_success() {
        bail!("Failed to download manifest: HTTP {}", response.status());
    }

    let content = response.text().context("Failed to read response body")?;

    fs::write(cache_path, content).context(format!(
        "Failed to write manifest to {}",
        cache_path.display()
    ))?;

    Ok(())
}

// --- Manifest Parsing Logic ---
pub fn parse_manifest(config: &Config) -> Result<ManifestData> {
    let cache_path = config.manifest_cache_path();
    if !cache_path.exists() {
        bail!(
            "Manifest cache file does not exist at {}. Please download it first.",
            cache_path.display()
        );
    }

    let content = fs::read_to_string(cache_path).context(format!(
        "Failed to read manifest cache file at {}",
        cache_path.display()
    ))?;

    // Attempt parsing and print detailed error on failure
    match serde_yaml::from_str::<ManifestData>(&content) {
        Ok(data) => Ok(data),
        Err(e) => {
            eprintln!("Detailed YAML parsing error: {:?}", e); // Print the specific error
                                                               // Optionally print location if available
            if let Some(location) = e.location() {
                eprintln!(
                    "Error location: line {}, column {}",
                    location.line(),
                    location.column()
                );
            }
            // Return the generic error context for anyhow
            Err(e).context("Failed to parse manifest YAML data (see detailed error above)")
        }
    }
}

// --- Placeholder Resolution ---

fn get_proton_drive_c(config: &Config, game_id: &str) -> PathBuf {
    config.compatdata_path().join(game_id).join("pfx/drive_c")
}

fn get_proton_steamuser(config: &Config, game_id: &str) -> PathBuf {
    get_proton_drive_c(config, game_id).join("users/steamuser")
}

/// Resolves manifest path placeholders relative to a specific Proton prefix.
/// Returns None if a required placeholder is unresolvable in the context.
pub fn resolve_manifest_path(manifest_path: &str, config: &Config, game_id: &str) -> Option<PathBuf> {
    let drive_c = get_proton_drive_c(config, game_id);
    let user = get_proton_steamuser(config, game_id);
    let os_user_name = "steamuser"; // Always steamuser in Proton

    // Early return for unsupported placeholders we can't easily resolve
    if manifest_path.contains("<base>") || 
       manifest_path.contains("<root>") || 
       manifest_path.contains("<game>") ||
       manifest_path.contains("<storeUserId>") {
        // Log this maybe? println!("Skipping manifest path with currently unsupported placeholder: {}", manifest_path);
        return None;
    }

    let mut resolved = manifest_path.to_string();

    // Replace placeholders - order might matter slightly if placeholders are nested (unlikely based on schema)
    // Windows specific paths
    resolved = resolved.replace("<winAppData>", &user.join("AppData/Roaming").to_string_lossy());
    resolved = resolved.replace("<winLocalAppData>", &user.join("AppData/Local").to_string_lossy());
    resolved = resolved.replace("<winLocalAppDataLow>", &user.join("AppData/LocalLow").to_string_lossy());
    resolved = resolved.replace("<winDocuments>", &user.join("Documents").to_string_lossy());
    resolved = resolved.replace("<winPublic>", &drive_c.join("users/Public").to_string_lossy());
    resolved = resolved.replace("<winProgramData>", &drive_c.join("ProgramData").to_string_lossy());
    resolved = resolved.replace("<winDir>", &drive_c.join("windows").to_string_lossy());
    
    // Common paths
    resolved = resolved.replace("<home>", &user.to_string_lossy());
    resolved = resolved.replace("<osUserName>", os_user_name);
    resolved = resolved.replace("<storeGameId>", game_id);

    // Linux/XDG paths - unlikely to be used with win* paths but handle defensively
    // We map them inside the prefix for consistency, though games using them might not store saves there.
    resolved = resolved.replace("<xdgData>", &user.join(".local/share").to_string_lossy());
    resolved = resolved.replace("<xdgConfig>", &user.join(".config").to_string_lossy());

    // Check if any placeholders remain unresolved (basic check)
    if resolved.contains('<') {
        // println!("Warning: Path may still contain unresolved placeholders: {}", resolved);
        // Decide if we should return None or the partially resolved path.
        // Let's return None for now if it looks like placeholders are left.
        return None; 
    }

    Some(PathBuf::from(resolved))
}

/// Tries to identify a game in the manifest by matching resolved manifest paths
/// against paths found within a specific prefix's save locations.
pub fn find_game_for_prefix_by_path<'a>(
    manifest: &'a ManifestData,
    prefix_data: &PrefixData,
    config: &Config,
) -> Option<(String, &'a GameEntry)> {
    // Iterate through locations found in the prefix scan
    for save_loc in &prefix_data.save_locations {
        for entry in &save_loc.entries {
            let found_path = &entry.path; // The actual path found on disk

            // Normalize the found path once
            let normalized_found = found_path
                .as_path()
                .to_string_lossy()
                .trim_end_matches('/')
                .to_lowercase();
            if normalized_found.is_empty() { continue; } // Skip empty paths

            // Now, iterate through the manifest to see if this path matches any rule
            for (manifest_game_name, manifest_entry) in &manifest.games {
                if let Some(files) = &manifest_entry.files {
                    for manifest_path_str in files.keys() {
                        // Resolve the manifest path string using the prefix's game_id
                        if let Some(resolved_manifest_path) = resolve_manifest_path(
                            manifest_path_str,
                            config,
                            &prefix_data.game_id,
                        ) {
                            // Normalize the resolved manifest path
                            let normalized_manifest = resolved_manifest_path
                                .as_path()
                                .to_string_lossy()
                                .trim_end_matches('/')
                                .to_lowercase();

                            // Check if the normalized manifest path starts with the normalized found path
                            if !normalized_manifest.is_empty() && normalized_manifest.starts_with(&normalized_found) {
                                // Found a match! Return the game name and entry
                                return Some((manifest_game_name.clone(), manifest_entry));
                            }
                        }
                    }
                }
            } // End manifest game iteration
        } // End save entry iteration
    } // End save location iteration

    // If no match was found after checking all paths and manifest entries
    None
}
