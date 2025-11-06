pub mod appimage_yaml;
pub mod index_yaml;
pub mod collectives;
pub mod fetcher;
pub mod resolver;
pub mod cache;

pub use appimage_yaml::*;
pub use index_yaml::*;
pub use collectives::*;
pub use fetcher::*;
pub use resolver::*;
pub use cache::*;

use anyhow::Result;
use crate::config::Config;
use tokio::fs;

pub async fn update_database() -> Result<()> {
    cache::update_unified_index().await
}

pub async fn add_source(url: &str) -> Result<()> {
    let config = Config::new()?;
    config.ensure_directories().await?;
    
    let mut sources_yaml = if config.sources_file.exists() {
        let content = fs::read_to_string(&config.sources_file).await?;
        collectives::SourcesYaml::from_str(&content)?
    } else {
        collectives::SourcesYaml::new()
    };
    
    sources_yaml.add_source(url.to_string());
    
    let content = sources_yaml.to_string()?;
    fs::write(&config.sources_file, content).await?;
    
    println!("Added source: {}", url);
    Ok(())
}

pub async fn remove_source(url: &str) -> Result<()> {
    let config = Config::new()?;
    
    if !config.sources_file.exists() {
        anyhow::bail!("No sources file found");
    }
    
    let content = fs::read_to_string(&config.sources_file).await?;
    let mut sources_yaml = collectives::SourcesYaml::from_str(&content)?;
    
    if sources_yaml.remove_source(url) {
        let content = sources_yaml.to_string()?;
        fs::write(&config.sources_file, content).await?;
        println!("Removed source: {}", url);
    } else {
        println!("Source not found: {}", url);
    }
    
    Ok(())
}

pub async fn list_sources() -> Result<()> {
    let config = Config::new()?;
    
    let mut sources = Vec::new();
    
    if config.sources_file.exists() {
        let content = fs::read_to_string(&config.sources_file).await?;
        let sources_yaml = collectives::SourcesYaml::from_str(&content)?;
        sources.extend(sources_yaml.sources);
    }
    
    if config.collectives_file.exists() {
        let content = fs::read_to_string(&config.collectives_file).await?;
        let collectives_yaml = collectives::CollectivesYaml::from_str(&content)?;
        for collective in &collectives_yaml.collectives {
            println!("Collective '{}':", collective.name);
            for source in &collective.sources {
                println!("  {}", source);
            }
        }
        sources.extend(collectives_yaml.get_all_sources());
    }
    
    if sources.is_empty() {
        println!("No sources configured");
    } else {
        sources.sort();
        sources.dedup();
        println!("Sources:");
        for source in sources {
            println!("  {}", source);
        }
    }
    
    Ok(())
}

