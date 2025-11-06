use anyhow::{Context, Result};
use dirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    pub appimages_dir: PathBuf,
    pub desktop_files_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub sources_file: PathBuf,
    pub collectives_file: PathBuf,
    pub unified_index_cache: PathBuf,
    pub database_file: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub appimages_dir: Option<PathBuf>,
    pub desktop_files_dir: Option<PathBuf>,
    pub bin_dir: Option<PathBuf>,
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_home = dirs::config_dir()
            .context("Failed to find config directory")?
            .join("aipkg");
        
        let cache_home = dirs::cache_dir()
            .context("Failed to find cache directory")?
            .join("aipkg");
        
        let data_home = dirs::data_dir()
            .context("Failed to find data directory")?
            .join("aipkg");

        let config = Config {
            appimages_dir: data_home.join("appimages"),
            desktop_files_dir: dirs::data_dir()
                .context("Failed to find data directory")?
                .join("applications"),
            bin_dir: dirs::home_dir()
                .context("Failed to find home directory")?
                .join(".local/bin"),
            config_dir: config_home.clone(),
            cache_dir: cache_home.clone(),
            sources_file: config_home.join("sources.yaml"),
            collectives_file: config_home.join("collectives.yaml"),
            unified_index_cache: cache_home.join("unified_index.yaml"),
            database_file: config_home.join("database.yaml"),
        };

        // Load config file if it exists and override defaults
        let config_file_path = config_home.join("config.toml");
        let mut final_config = config;
        
        if config_file_path.exists() {
            let config_file: ConfigFile = toml::from_str(
                &std::fs::read_to_string(&config_file_path)
                    .context("Failed to read config file")?
            )?;
            
            if let Some(dir) = config_file.appimages_dir {
                final_config.appimages_dir = dir;
            }
            if let Some(dir) = config_file.desktop_files_dir {
                final_config.desktop_files_dir = dir;
            }
            if let Some(dir) = config_file.bin_dir {
                final_config.bin_dir = dir;
            }
        }

        Ok(final_config)
    }

    pub async fn ensure_directories(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.appimages_dir).await
            .context("Failed to create appimages directory")?;
        tokio::fs::create_dir_all(&self.desktop_files_dir).await
            .context("Failed to create desktop files directory")?;
        tokio::fs::create_dir_all(&self.bin_dir).await
            .context("Failed to create bin directory")?;
        tokio::fs::create_dir_all(&self.config_dir).await
            .context("Failed to create config directory")?;
        tokio::fs::create_dir_all(&self.cache_dir).await
            .context("Failed to create cache directory")?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new().expect("Failed to initialize config")
    }
}

