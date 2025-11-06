use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectivesYaml {
    #[serde(default)]
    pub collectives: Vec<Collective>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collective {
    pub name: String,
    pub sources: Vec<String>,
}

impl CollectivesYaml {
    pub fn new() -> Self {
        Self {
            collectives: Vec::new(),
        }
    }

    pub fn from_str(content: &str) -> Result<Self> {
        if content.trim().is_empty() {
            Ok(Self::new())
        } else {
            serde_yaml::from_str(content)
                .context("Failed to parse collectives.yaml")
        }
    }

    pub fn to_string(&self) -> Result<String> {
        serde_yaml::to_string(self)
            .context("Failed to serialize collectives.yaml")
    }

    pub fn add_to_collective(&mut self, name: &str, urls: Vec<String>) {
        if let Some(collective) = self.collectives.iter_mut().find(|c| c.name == name) {
            for url in urls {
                if !collective.sources.contains(&url) {
                    collective.sources.push(url);
                }
            }
        } else {
            self.collectives.push(Collective {
                name: name.to_string(),
                sources: urls,
            });
        }
    }

    pub fn remove_collective(&mut self, name: &str) -> bool {
        let initial_len = self.collectives.len();
        self.collectives.retain(|c| c.name != name);
        self.collectives.len() < initial_len
    }

    pub fn get_all_sources(&self) -> Vec<String> {
        let mut sources = Vec::new();
        for collective in &self.collectives {
            sources.extend(collective.sources.clone());
        }
        sources.sort();
        sources.dedup();
        sources
    }
}

impl Default for CollectivesYaml {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesYaml {
    #[serde(default)]
    pub sources: Vec<String>,
}

impl SourcesYaml {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn from_str(content: &str) -> Result<Self> {
        if content.trim().is_empty() {
            Ok(Self::new())
        } else {
            serde_yaml::from_str(content)
                .context("Failed to parse sources.yaml")
        }
    }

    pub fn to_string(&self) -> Result<String> {
        serde_yaml::to_string(self)
            .context("Failed to serialize sources.yaml")
    }

    pub fn add_source(&mut self, url: String) {
        if !self.sources.contains(&url) {
            self.sources.push(url);
        }
    }

    pub fn remove_source(&mut self, url: &str) -> bool {
        let initial_len = self.sources.len();
        self.sources.retain(|s| s != url);
        self.sources.len() < initial_len
    }
}

impl Default for SourcesYaml {
    fn default() -> Self {
        Self::new()
    }
}

// Module-level functions for collectives management
use crate::config::Config;
use tokio::fs;

pub async fn add_to_collective(name: &str, urls: Vec<String>) -> Result<()> {
    let config = Config::new()?;
    config.ensure_directories().await?;
    
    let mut collectives_yaml = if config.collectives_file.exists() {
        let content = fs::read_to_string(&config.collectives_file).await?;
        CollectivesYaml::from_str(&content)?
    } else {
        CollectivesYaml::new()
    };
    
    collectives_yaml.add_to_collective(name, urls.clone());
    
    let content = collectives_yaml.to_string()?;
    fs::write(&config.collectives_file, content).await?;
    
    println!("Added {} source(s) to collective '{}'", urls.len(), name);
    Ok(())
}

pub async fn remove_collective(name: &str) -> Result<()> {
    let config = Config::new()?;
    
    if !config.collectives_file.exists() {
        anyhow::bail!("No collectives file found");
    }
    
    let content = fs::read_to_string(&config.collectives_file).await?;
    let mut collectives_yaml = CollectivesYaml::from_str(&content)?;
    
    if collectives_yaml.remove_collective(name) {
        let content = collectives_yaml.to_string()?;
        fs::write(&config.collectives_file, content).await?;
        println!("Removed collective: {}", name);
    } else {
        println!("Collective not found: {}", name);
    }
    
    Ok(())
}

pub async fn list_collectives() -> Result<()> {
    let config = Config::new()?;
    
    if !config.collectives_file.exists() {
        println!("No collectives configured");
        return Ok(());
    }
    
    let content = fs::read_to_string(&config.collectives_file).await?;
    let collectives_yaml = CollectivesYaml::from_str(&content)?;
    
    if collectives_yaml.collectives.is_empty() {
        println!("No collectives configured");
    } else {
        println!("Collectives:");
        for collective in &collectives_yaml.collectives {
            println!("  {} ({} sources)", collective.name, collective.sources.len());
            for source in &collective.sources {
                println!("    {}", source);
            }
        }
    }
    
    Ok(())
}

