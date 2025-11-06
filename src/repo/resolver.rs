use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use url::Url;

use crate::repo::appimage_yaml::{AppImageEntry, UnifiedIndex};
use crate::repo::index_yaml::{IndexYaml, SourceType};
use crate::repo::fetcher::Fetcher;
use crate::repo::cache::calculate_yaml_hash;

pub struct Resolver {
    fetcher: Fetcher,
    visited: HashSet<String>,
}

impl Resolver {
    pub fn new() -> Result<Self> {
        Ok(Self {
            fetcher: Fetcher::new()?,
            visited: HashSet::new(),
        })
    }

    pub async fn resolve_sources(&mut self, sources: Vec<String>) -> Result<UnifiedIndex> {
        let mut index = UnifiedIndex::new();
        self.visited.clear();
        
        // Process sources sequentially to maintain visited set correctly
        for source_url in sources {
            let entries = self.resolve_source_flattened(&source_url).await?;
            for entry in entries {
                index.add_entry(entry.entry, entry.source_url);
            }
        }
        
        Ok(index)
    }

    pub async fn resolve_sources_incremental(
        &mut self,
        sources: Vec<String>,
        existing_index: &mut Option<UnifiedIndex>,
        source_hashes: &mut std::collections::HashMap<String, String>,
    ) -> Result<UnifiedIndex> {
        let mut index = existing_index.take().unwrap_or_else(UnifiedIndex::new);
        self.visited.clear();
        
        // Process sources sequentially, checking hashes for incremental updates
        for source_url in sources {
            let cached_hash = source_hashes.get(&source_url).cloned();
            let (entries, hash) = self.resolve_source_incremental(&source_url, cached_hash).await?;
            
            if let Some(h) = hash {
                source_hashes.insert(source_url.clone(), h);
            }
            
            for entry in entries {
                index.add_entry(entry.entry, entry.source_url);
            }
        }
        
        Ok(index)
    }

    async fn resolve_source_flattened(&mut self, url: &str) -> Result<Vec<crate::repo::appimage_yaml::AppImageEntryWithSource>> {
        let normalized = self.normalize_url(url)?;
        
        if self.visited.contains(&normalized) {
            return Ok(Vec::new());
        }
        
        self.visited.insert(normalized.clone());
        
        let content = self.fetcher.fetch_yaml(&normalized).await?;
        let entries = self.parse_yaml_content(&content, &normalized).await?;
        
        Ok(entries)
    }

    async fn resolve_source_incremental(
        &mut self,
        url: &str,
        cached_hash: Option<String>,
    ) -> Result<(Vec<crate::repo::appimage_yaml::AppImageEntryWithSource>, Option<String>)> {
        let normalized = self.normalize_url(url)?;
        
        if self.visited.contains(&normalized) {
            return Ok((Vec::new(), None));
        }
        
        self.visited.insert(normalized.clone());
        
        let content = self.fetcher.fetch_yaml(&normalized).await?;
        let current_hash = calculate_yaml_hash(&content).await;
        
        // Skip if hash hasn't changed
        if let Some(ref cached) = cached_hash {
            if cached == &current_hash {
                return Ok((Vec::new(), Some(current_hash)));
            }
        }
        
        let entries = self.parse_yaml_content(&content, &normalized).await?;
        
        Ok((entries, Some(current_hash)))
    }

    async fn parse_yaml_content(
        &mut self,
        content: &str,
        source_url: &str,
    ) -> Result<Vec<crate::repo::appimage_yaml::AppImageEntryWithSource>> {
        let mut entries = Vec::new();
        
        // Try to parse as index.yaml first
        if let Ok(index_yaml) = IndexYaml::from_str(content) {
            index_yaml.validate()?;
            
            // Flatten recursively - collect all URLs first
            let mut appimage_urls = Vec::new();
            let mut index_urls = Vec::new();
            
            for source in index_yaml.sources {
                let resolved_url = self.resolve_relative_url(source_url, &source.url)?;
                match source.source_type {
                    SourceType::Index => {
                        index_urls.push(resolved_url);
                    }
                    SourceType::Appimage => {
                        appimage_urls.push(resolved_url);
                    }
                }
            }
            
            // Process index URLs recursively (flattening) - sequential to maintain visited set
            // Use a work queue to avoid deep recursion
            let mut work_queue = index_urls;
            while let Some(index_url) = work_queue.pop() {
                if self.visited.contains(&index_url) {
                    continue;
                }
                self.visited.insert(index_url.clone());
                
                let sub_content = self.fetcher.fetch_yaml(&index_url).await?;
                // Try to parse as index.yaml
                if let Ok(sub_index) = IndexYaml::from_str(&sub_content) {
                    sub_index.validate()?;
                    // Add sub-sources to work queue
                    for sub_source in sub_index.sources {
                        let resolved = self.resolve_relative_url(&index_url, &sub_source.url)?;
                        match sub_source.source_type {
                            SourceType::Index => {
                                work_queue.push(resolved);
                            }
                            SourceType::Appimage => {
                                appimage_urls.push(resolved);
                            }
                        }
                    }
                } else if let Ok(sub_appimage) = crate::repo::appimage_yaml::AppImageYaml::from_str(&sub_content) {
                    sub_appimage.validate()?;
                    for entry in sub_appimage.apps {
                        entries.push(crate::repo::appimage_yaml::AppImageEntryWithSource {
                            entry,
                            source_url: index_url.clone(),
                        });
                    }
                }
            }
            
            // Process appimage URLs sequentially to maintain visited set
            for appimage_url in appimage_urls {
                if !self.visited.contains(&appimage_url) {
                    self.visited.insert(appimage_url.clone());
                    let appimage_content = self.fetcher.fetch_yaml(&appimage_url).await?;
                    let appimage_yaml = crate::repo::appimage_yaml::AppImageYaml::from_str(&appimage_content)?;
                    appimage_yaml.validate()?;
                    
                    for entry in appimage_yaml.apps {
                        entries.push(crate::repo::appimage_yaml::AppImageEntryWithSource {
                            entry,
                            source_url: appimage_url.clone(),
                        });
                    }
                }
            }
        } else {
            // Try to parse as appimage.yaml
            let appimage_yaml = crate::repo::appimage_yaml::AppImageYaml::from_str(content)?;
            appimage_yaml.validate()?;
            
            for entry in appimage_yaml.apps {
                entries.push(crate::repo::appimage_yaml::AppImageEntryWithSource {
                    entry,
                    source_url: source_url.to_string(),
                });
            }
        }
        
        Ok(entries)
    }

    fn normalize_url(&self, url: &str) -> Result<String> {
        let parsed = Url::parse(url)?;
        Ok(parsed.as_str().to_string())
    }

    fn resolve_relative_url(&self, base: &str, relative: &str) -> Result<String> {
        let base_url = Url::parse(base)?;
        let resolved = base_url.join(relative)?;
        Ok(resolved.as_str().to_string())
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new().expect("Failed to create resolver")
    }
}
