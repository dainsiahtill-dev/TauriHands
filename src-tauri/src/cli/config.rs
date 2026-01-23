use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspace: Option<PathBuf>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub max_steps: Option<usize>,
    pub auto_confirm: bool,
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace: None,
            model: None,
            api_key: None,
            max_steps: None,
            auto_confirm: false,
            log_level: "info".to_string(),
        }
    }
}

pub fn load_config(config_path: Option<&PathBuf>) -> Result<Config> {
    let config_path = config_path
        .or_else(|| dirs::config_dir().map(|dir| dir.join("taurihands").join("config.toml")))
        .unwrap_or_else(|| PathBuf::from("taurihands.toml"));

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config file")?;
        
        log::debug!("Loaded configuration from: {:?}", config_path);
        Ok(config)
    } else {
        log::debug!("No config file found, using defaults");
        Ok(Config::default())
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = dirs::config_dir()
        .map(|dir| dir.join("taurihands").join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("taurihands.toml"));

    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(config)
        .with_context(|| "Failed to serialize config")?;
    
    std::fs::write(&config_path, content)
        .with_context(|| format!("Failed to write config file: {:?}", config_path))?;
    
    log::debug!("Saved configuration to: {:?}", config_path);
    Ok(())
}

pub fn get_workspace_path(config: &Config) -> Result<PathBuf> {
    Ok(config.workspace
        .clone()
        .or_else(|| std::env::var("TAURIHANDS_WORKSPACE").ok().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap()))
}
