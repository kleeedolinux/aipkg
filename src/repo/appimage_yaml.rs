use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageYaml {
    pub apps: Vec<AppImageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageEntry {
    pub name: String,
    pub version: String,
    pub file: String,
    pub sha256: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
}

impl AppImageYaml {
    pub fn from_str(content: &str) -> Result<Self> {
        serde_yaml::from_str(content)
            .context("Failed to parse appimage.yaml")
    }

    pub fn validate(&self) -> Result<()> {
        for app in &self.apps {
            if app.sha256.is_empty() {
                anyhow::bail!("SHA256 is mandatory for app: {}", app.name);
            }
            if app.sha256.len() != 64 {
                anyhow::bail!("Invalid SHA256 length for app: {}", app.name);
            }
            if app.name.is_empty() {
                anyhow::bail!("App name cannot be empty");
            }
            if app.version.is_empty() {
                anyhow::bail!("Version cannot be empty for app: {}", app.name);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedIndex {
    pub apps: HashMap<String, Vec<AppImageEntryWithSource>>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageEntryWithSource {
    #[serde(flatten)]
    pub entry: AppImageEntry,
    pub source_url: String,
}

impl UnifiedIndex {
    pub fn new() -> Self {
        Self {
            apps: HashMap::new(),
            last_updated: None,
        }
    }

    pub fn add_entry(&mut self, entry: AppImageEntry, source_url: String) {
        self.apps
            .entry(entry.name.clone())
            .or_insert_with(Vec::new)
            .push(AppImageEntryWithSource {
                entry,
                source_url,
            });
    }

    pub fn find_best_match(&self, name: &str, version_req: Option<&str>) -> Option<&AppImageEntryWithSource> {
        if let Some(entries) = self.apps.get(name) {
            if let Some(req) = version_req {
                // Use semver for version matching
                if let Ok(version_req) = semver::VersionReq::parse(req) {
                    return entries.iter()
                        .filter_map(|e| {
                            semver::Version::parse(&e.entry.version).ok()
                                .and_then(|v| if version_req.matches(&v) { Some(e) } else { None })
                        })
                        .max_by_key(|e| semver::Version::parse(&e.entry.version).ok());
                }
            }
            // Return latest version
            return entries.iter()
                .max_by_key(|e| semver::Version::parse(&e.entry.version).ok());
        }
        None
    }
}

impl Default for UnifiedIndex {
    fn default() -> Self {
        Self::new()
    }
}

