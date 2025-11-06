use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::repo::appimage_yaml::{AppImageEntry, AppImageEntryWithSource, UnifiedIndex};
use crate::repo::cache::load_unified_index;
use crate::repo::fetcher::Fetcher;
use crate::verify::{verify_sha256_bytes};
use crate::utils::extract_metadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub desktop_file: PathBuf,
    pub symlink: PathBuf,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDatabase {
    packages: HashMap<String, InstalledPackage>,
}

impl PackageDatabase {
    fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    fn load(content: &str) -> Result<Self> {
        if content.trim().is_empty() {
            Ok(Self::new())
        } else {
            serde_yaml::from_str(content)
                .context("Failed to parse package database")
        }
    }


    pub fn add_package(&mut self, pkg: InstalledPackage) {
        self.packages.insert(pkg.name.clone(), pkg);
    }

    pub fn remove_package(&mut self, name: &str) -> Option<InstalledPackage> {
        self.packages.remove(name)
    }

    pub fn get_package(&self, name: &str) -> Option<&InstalledPackage> {
        self.packages.get(name)
    }

    pub fn list_packages(&self) -> Vec<&InstalledPackage> {
        self.packages.values().collect()
    }

    pub fn to_string(&self) -> Result<String> {
        serde_yaml::to_string(self)
            .context("Failed to serialize package database")
    }
}

pub async fn install_from_file(path: &str) -> Result<()> {
    let config = Config::new()?;
    config.ensure_directories().await?;
    
    // Verify file exists
    if !Path::new(path).exists() {
        anyhow::bail!("File not found: {}", path);
    }
    
    // Extract metadata
    let metadata = extract_metadata(path).await?;
    
    // Create installation directory
    let version_str = metadata.version.as_ref().map(|v| v.as_str()).unwrap_or("unknown");
    let install_dir = config.appimages_dir
        .join(&metadata.name)
        .join(version_str);
    fs::create_dir_all(&install_dir).await?;
    
    // Copy AppImage
    let appimage_name = format!("{}.AppImage", metadata.name);
    let target_path = install_dir.join(&appimage_name);
    fs::copy(path, &target_path).await?;
    
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms).await?;
    }
    
    // Generate desktop file
    let desktop_file = generate_desktop_file(&config, &metadata, &target_path).await?;
    
    // Create symlink
    let symlink_path = config.bin_dir.join(&metadata.name);
    if symlink_path.exists() {
        fs::remove_file(&symlink_path).await?;
    }
    fs::symlink(&target_path, &symlink_path).await?;
    
    // Update database
    let version = metadata.version.clone().unwrap_or_else(|| "unknown".to_string());
    update_database(&config, InstalledPackage {
        name: metadata.name.clone(),
        version: version.clone(),
        path: target_path.clone(),
        desktop_file: desktop_file.clone(),
        symlink: symlink_path.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    }).await?;
    
    println!("Installed {} {}", metadata.name, version);
    Ok(())
}

pub async fn install_from_repo(package: &str) -> Result<()> {
    let config = Config::new()?;
    config.ensure_directories().await?;
    
    // Load unified index
    let index = load_unified_index().await?;
    
    // Find best match (with fuzzy matching)
    let entry = find_best_match(&index, package, None)?;
    
    // Resolve dependencies
    let dependencies = resolve_dependencies(&index, &entry.entry).await?;
    
    // Install dependencies sequentially to avoid conflicts
    // Parallel installation could cause issues with shared resources
    for dep in &dependencies {
        println!("Installing dependency: {}", dep.entry.name);
        install_appimage_entry(&config, dep, &index).await?;
    }
    
    // Install the requested package
    println!("Installing: {}", entry.entry.name);
    install_appimage_entry(&config, entry, &index).await?;
    
    Ok(())
}

async fn resolve_dependencies<'a>(
    index: &'a UnifiedIndex,
    entry: &AppImageEntry,
) -> Result<Vec<&'a AppImageEntryWithSource>> {
    let mut resolved = Vec::new();
    let mut visited = HashSet::new();
    let mut to_resolve = entry.dependencies.clone();
    
    while let Some(dep_name) = to_resolve.pop() {
        if visited.contains(&dep_name) {
            continue;
        }
        visited.insert(dep_name.clone());
        
        if let Some(dep_entry) = index.find_best_match(&dep_name, None) {
            resolved.push(dep_entry);
            to_resolve.extend(dep_entry.entry.dependencies.clone());
        }
    }
    
    Ok(resolved)
}

fn find_best_match<'a>(
    index: &'a UnifiedIndex,
    query: &str,
    version_req: Option<&str>,
) -> Result<&'a AppImageEntryWithSource> {
    // Try exact match first
    if let Some(entry) = index.find_best_match(query, version_req) {
        return Ok(entry);
    }
    
    // Try fuzzy matching
    use fuzzy_matcher::FuzzyMatcher;
    use fuzzy_matcher::skim::SkimMatcherV2;
    
    let matcher = SkimMatcherV2::default();
    let mut best_match: Option<(&String, &AppImageEntryWithSource, i64)> = None;
    
    for (name, entries) in &index.apps {
        for entry in entries {
            if let Some(score) = matcher.fuzzy_match(name, query) {
                if best_match.is_none() || score > best_match.unwrap().2 {
                    best_match = Some((name, entry, score));
                }
            }
        }
    }
    
    if let Some((_, entry, _)) = best_match {
        Ok(entry)
    } else {
        anyhow::bail!("Package not found: {}", query);
    }
}

pub async fn load_database(config: &Config) -> Result<PackageDatabase> {
    let content = if config.database_file.exists() {
        tokio::fs::read_to_string(&config.database_file).await?
    } else {
        String::new()
    };
    PackageDatabase::load(&content)
}

pub async fn install_appimage_entry(
    config: &Config,
    entry: &AppImageEntryWithSource,
    _index: &UnifiedIndex,
) -> Result<()> {
    // Resolve download URL
    let base_url = url::Url::parse(&entry.source_url)?;
    let download_url = base_url.join(&entry.entry.file)?;
    
    // Download AppImage
    let fetcher = Fetcher::new()?;
    let appimage_data = fetcher.fetch_appimage(
        download_url.as_str(),
        entry.entry.size,
    ).await?;
    
    // Verify SHA256
    if !verify_sha256_bytes(&appimage_data, &entry.entry.sha256)? {
        anyhow::bail!("SHA256 verification failed for {}", entry.entry.name);
    }
    
    // Create installation directory
    let install_dir = config.appimages_dir
        .join(&entry.entry.name)
        .join(&entry.entry.version);
    fs::create_dir_all(&install_dir).await?;
    
    // Save AppImage
    let appimage_name = format!("{}.AppImage", entry.entry.name);
    let target_path = install_dir.join(&appimage_name);
    fs::write(&target_path, appimage_data).await?;
    
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms).await?;
    }
    
    // Extract metadata for desktop file
    let metadata = extract_metadata(target_path.to_str().unwrap()).await?;
    
    // Generate desktop file
    let desktop_file = generate_desktop_file(config, &metadata, &target_path).await?;
    
    // Create symlink
    let symlink_path = config.bin_dir.join(&entry.entry.name);
    if symlink_path.exists() {
        fs::remove_file(&symlink_path).await?;
    }
    fs::symlink(&target_path, &symlink_path).await?;
    
    // Update database
    update_database(config, InstalledPackage {
        name: entry.entry.name.clone(),
        version: entry.entry.version.clone(),
        path: target_path.clone(),
        desktop_file: desktop_file.clone(),
        symlink: symlink_path.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    }).await?;
    
    Ok(())
}

async fn generate_desktop_file(
    config: &Config,
    metadata: &crate::utils::AppImageMetadata,
    appimage_path: &Path,
) -> Result<PathBuf> {
    let desktop_name = format!("{}.desktop", metadata.name);
    let desktop_path = config.desktop_files_dir.join(&desktop_name);
    
    let exec_path = appimage_path.to_string_lossy().to_string();
    let icon_path = metadata.icon.as_ref()
        .map(|i| i.clone())
        .unwrap_or_else(|| exec_path.clone());
    
    let desktop_content = format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Name={}\n\
        Exec={}\n\
        Icon={}\n\
        Categories={}\n\
        Comment={}\n\
        Terminal=false\n\
        StartupNotify=true\n",
        metadata.name,
        exec_path,
        icon_path,
        metadata.categories.join(";"),
        metadata.description.as_deref().unwrap_or(""),
    );
    
    fs::write(&desktop_path, desktop_content).await?;
    Ok(desktop_path)
}

async fn update_database(config: &Config, pkg: InstalledPackage) -> Result<()> {
    let content = if config.database_file.exists() {
        fs::read_to_string(&config.database_file).await?
    } else {
        String::new()
    };
    
    let mut db = PackageDatabase::load(&content)?;
    db.add_package(pkg);
    
    let content = db.to_string()?;
    fs::write(&config.database_file, content).await?;
    Ok(())
}

