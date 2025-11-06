use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexYaml {
    pub sources: Vec<IndexSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSource {
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Appimage,
    Index,
}

impl IndexYaml {
    pub fn from_str(content: &str) -> Result<Self> {
        serde_yaml::from_str(content)
            .context("Failed to parse index.yaml")
    }

    pub fn validate(&self) -> Result<()> {
        for source in &self.sources {
            if source.url.is_empty() {
                anyhow::bail!("Source URL cannot be empty");
            }
            // Basic URL validation
            url::Url::parse(&source.url)
                .context(format!("Invalid URL: {}", source.url))?;
        }
        Ok(())
    }
}

