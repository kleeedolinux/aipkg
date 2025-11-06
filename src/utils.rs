use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct AppImageMetadata {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub size: u64,
}

pub async fn extract_metadata(appimage_path: &str) -> Result<AppImageMetadata> {
    let path = Path::new(appimage_path);
    
    // Get file size
    let metadata = fs::metadata(appimage_path).await?;
    let size = metadata.len();
    
    // Try to extract desktop entry from AppImage
    // AppImages can be type 1 (squashfs) or type 2 (ELF)
    // We'll try to extract using desktop-file-validate or by mounting
    
    let name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    // Try to extract desktop entry using AppImage runtime or desktop-file-validate
    let desktop_entry = extract_desktop_entry(appimage_path).await?;
    
    Ok(AppImageMetadata {
        name: desktop_entry.name.unwrap_or(name),
        version: desktop_entry.version,
        description: desktop_entry.comment,
        exec: desktop_entry.exec,
        icon: desktop_entry.icon,
        categories: desktop_entry.categories.unwrap_or_default(),
        size,
    })
}

#[derive(Debug, Default)]
struct DesktopEntry {
    name: Option<String>,
    version: Option<String>,
    comment: Option<String>,
    exec: Option<String>,
    icon: Option<String>,
    categories: Option<Vec<String>>,
}

async fn extract_desktop_entry(appimage_path: &str) -> Result<DesktopEntry> {
    let mut entry = DesktopEntry::default();
    
    // Try using AppImage runtime to extract desktop file
    // First, try to run the AppImage with --appimage-extract-and-run --appimage-help
    // Or use desktop-file-validate if available
    
    // For now, we'll try a simple approach: check if we can read the embedded desktop entry
    // This is a simplified version - in production, you'd want to use appimage-rs or mount the AppImage
    
    // Try to extract using strings command or by reading the ELF/squashfs structure
    // For type 2 AppImages, the desktop entry is embedded in the ELF
    // For type 1, it's in the squashfs filesystem
    
    // Simplified: try to find desktop entry in the file
    let content = fs::read(appimage_path).await?;
    
    // Look for desktop entry markers
    if let Some(desktop_start) = find_desktop_entry_start(&content) {
        if let Some(desktop_content) = extract_desktop_content(&content[desktop_start..]) {
            entry = parse_desktop_entry(&desktop_content)?;
        }
    }
    
    Ok(entry)
}

fn find_desktop_entry_start(content: &[u8]) -> Option<usize> {
    // Look for common desktop entry markers
    let markers: &[&[u8]] = &[
        b"[Desktop Entry]",
        b"X-AppImage",
    ];
    
    for marker in markers.iter() {
        if let Some(pos) = content.windows(marker.len()).position(|w| w == *marker) {
            return Some(pos);
        }
    }
    
    None
}

fn extract_desktop_content(content: &[u8]) -> Option<String> {
    // Try to extract readable text that looks like a desktop entry
    let text = String::from_utf8_lossy(content);
    
    // Find [Desktop Entry] section
    if let Some(start) = text.find("[Desktop Entry]") {
        let section = &text[start..];
        // Take up to next section or end
        let end = section.find("\n[").unwrap_or(section.len());
        Some(section[..end].to_string())
    } else {
        None
    }
}

fn parse_desktop_entry(content: &str) -> Result<DesktopEntry> {
    let mut entry = DesktopEntry::default();
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            
            match key {
                "Name" => entry.name = Some(value.to_string()),
                "Version" => entry.version = Some(value.to_string()),
                "Comment" => entry.comment = Some(value.to_string()),
                "Exec" => entry.exec = Some(value.to_string()),
                "Icon" => entry.icon = Some(value.to_string()),
                "Categories" => {
                    entry.categories = Some(
                        value.split(';')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    );
                }
                _ => {}
            }
        }
    }
    
    Ok(entry)
}

pub async fn get_appimage_size(path: &str) -> Result<u64> {
    let metadata = fs::metadata(path).await?;
    Ok(metadata.len())
}
