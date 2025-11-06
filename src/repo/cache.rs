use anyhow::{Context, Result};
use std::time::SystemTime;
use tokio::fs;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use hex;

use crate::config::Config;
use crate::repo::appimage_yaml::UnifiedIndex;
use crate::repo::collectives::{CollectivesYaml, SourcesYaml};
use crate::repo::resolver::Resolver;

#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    last_updated: String,
    source_hashes: std::collections::HashMap<String, String>,
}

pub async fn update_unified_index() -> Result<()> {
    let config = Config::new()?;
    config.ensure_directories().await?;
    
    // Load sources
    let sources = load_all_sources(&config).await?;
    
    // Load existing cache metadata if available
    let cache_metadata_path = config.cache_dir.join("cache_metadata.yaml");
    let mut existing_metadata = if cache_metadata_path.exists() {
        let content = fs::read_to_string(&cache_metadata_path).await?;
        serde_yaml::from_str::<CacheMetadata>(&content).unwrap_or_else(|_| CacheMetadata {
            last_updated: String::new(),
            source_hashes: std::collections::HashMap::new(),
        })
    } else {
        CacheMetadata {
            last_updated: String::new(),
            source_hashes: std::collections::HashMap::new(),
        }
    };
    
    // Load existing unified index if available for incremental updates
    let mut existing_index = if config.unified_index_cache.exists() {
        load_unified_index().await.ok()
    } else {
        None
    };
    
    // Resolve sources with incremental updates
    let mut resolver = Resolver::new()?;
    let index = resolver.resolve_sources_incremental(
        sources,
        &mut existing_index,
        &mut existing_metadata.source_hashes,
    ).await?;
    
    // Update cache metadata
    existing_metadata.last_updated = chrono::Utc::now().to_rfc3339();
    let metadata_yaml = serde_yaml::to_string(&existing_metadata)?;
    fs::write(&cache_metadata_path, metadata_yaml).await?;
    
    // Save unified index
    let index_yaml = serde_yaml::to_string(&index)
        .context("Failed to serialize unified index")?;
    
    fs::write(&config.unified_index_cache, index_yaml).await
        .context("Failed to write unified index cache")?;
    
    println!("Updated package database");
    Ok(())
}

pub async fn calculate_yaml_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn load_unified_index() -> Result<UnifiedIndex> {
    let config = Config::new()?;
    
    if !config.unified_index_cache.exists() {
        anyhow::bail!("Unified index not found. Run 'aipkg update' first.");
    }
    
    let content = fs::read_to_string(&config.unified_index_cache).await
        .context("Failed to read unified index cache")?;
    
    serde_yaml::from_str(&content)
        .context("Failed to parse unified index")
}

async fn load_all_sources(config: &Config) -> Result<Vec<String>> {
    let mut sources = Vec::new();
    
    // Load from sources.yaml
    if config.sources_file.exists() {
        let content = fs::read_to_string(&config.sources_file).await
            .context("Failed to read sources.yaml")?;
        let sources_yaml = SourcesYaml::from_str(&content)?;
        sources.extend(sources_yaml.sources);
    }
    
    // Load from collectives.yaml
    if config.collectives_file.exists() {
        let content = fs::read_to_string(&config.collectives_file).await
            .context("Failed to read collectives.yaml")?;
        let collectives_yaml = CollectivesYaml::from_str(&content)?;
        sources.extend(collectives_yaml.get_all_sources());
    }
    
    // Remove duplicates
    sources.sort();
    sources.dedup();
    
    Ok(sources)
}

pub async fn get_cache_timestamp(config: &Config) -> Result<Option<SystemTime>> {
    if !config.unified_index_cache.exists() {
        return Ok(None);
    }
    
    let metadata = fs::metadata(&config.unified_index_cache).await?;
    match metadata.modified() {
        Ok(time) => Ok(Some(time)),
        Err(e) => Err(anyhow::anyhow!("Failed to get modification time: {}", e)),
    }
}

