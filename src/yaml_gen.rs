use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;
use regex::Regex;

use crate::repo::appimage_yaml::AppImageYaml;
use crate::verify::calculate_sha256;
use crate::utils::extract_metadata;

pub async fn generate_appimage_yaml(folder: &str, repo: &str) -> Result<()> {
    let folder_path = Path::new(folder);
    
    if !folder_path.exists() {
        anyhow::bail!("Folder not found: {}", folder);
    }
    
    if !folder_path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", folder);
    }
    
    // Parse owner/repo (for documentation purposes)
    let _ = if let Some((_o, _r)) = repo.split_once('/') {
        // Valid format
    } else {
        anyhow::bail!("Invalid repo format. Expected 'owner/repo', got: {}", repo);
    };
    
    // Scan for AppImages
    let mut entries = Vec::new();
    let mut dir = fs::read_dir(folder_path).await?;
    
    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("AppImage") {
            println!("Processing: {}", path.display());
            
            // Calculate SHA256
            let sha256 = calculate_sha256(path.to_str().unwrap()).await?;
            
            // Extract metadata
            let metadata = extract_metadata(path.to_str().unwrap()).await?;
            
            // Get relative file path
            let file_path = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            // Create entry
            let app_entry = crate::repo::appimage_yaml::AppImageEntry {
                name: metadata.name.clone(),
                version: metadata.version.unwrap_or_else(|| {
                    // Try to extract from filename
                    extract_version_from_filename(&file_path)
                }),
                file: file_path,
                sha256,
                size: Some(metadata.size),
                description: metadata.description,
                dependencies: Vec::new(), // Could be extracted from AppImage metadata
                provides: Vec::new(),
            };
            
            entries.push(app_entry);
        }
    }
    
    if entries.is_empty() {
        anyhow::bail!("No AppImage files found in {}", folder);
    }
    
    // Generate YAML
    let appimage_yaml = AppImageYaml { apps: entries };
    let yaml_content = serde_yaml::to_string(&appimage_yaml)?;
    
    // Write to appimage.yaml
    let output_path = folder_path.join("appimage.yaml");
    fs::write(&output_path, yaml_content).await?;
    
    println!("Generated appimage.yaml with {} app(s)", appimage_yaml.apps.len());
    println!("Output: {}", output_path.display());
    
    Ok(())
}

fn extract_version_from_filename(filename: &str) -> String {
    // Try to extract version from filename patterns like:
    // app-1.2.3.AppImage
    // app-v1.2.3.AppImage
    // app-1.2.3-x86_64.AppImage
    
    // Pattern for semantic version
    if let Ok(re) = Regex::new(r"v?(\d+\.\d+\.\d+)") {
        if let Some(caps) = re.captures(filename) {
            if let Some(version) = caps.get(1) {
                return version.as_str().to_string();
            }
        }
    }
    
    // Fallback: try to find any version-like pattern
    if let Ok(re) = Regex::new(r"(\d+\.\d+)") {
        if let Some(caps) = re.captures(filename) {
            if let Some(version) = caps.get(1) {
                return version.as_str().to_string();
            }
        }
    }
    
    "unknown".to_string()
}
