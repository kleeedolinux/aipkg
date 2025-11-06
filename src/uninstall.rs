use anyhow::{Context, Result};
use tokio::fs;

use crate::config::Config;
use crate::install::load_database;

pub async fn uninstall(package: &str) -> Result<()> {
    let config = Config::new()?;
    
    // Load database
    let mut db = load_database(&config).await?;
    
    // Find package
    let pkg = db.remove_package(package)
        .ok_or_else(|| anyhow::anyhow!("Package not installed: {}", package))?;
    
    // Remove AppImage directory
    if pkg.path.exists() {
        let appimage_dir = pkg.path.parent()
            .context("Invalid package path")?;
        fs::remove_dir_all(appimage_dir).await
            .context("Failed to remove AppImage directory")?;
    }
    
    // Remove desktop file
    if pkg.desktop_file.exists() {
        fs::remove_file(&pkg.desktop_file).await
            .context("Failed to remove desktop file")?;
    }
    
    // Remove symlink
    if pkg.symlink.exists() {
        fs::remove_file(&pkg.symlink).await
            .context("Failed to remove symlink")?;
    }
    
    // Update database
    let content = db.to_string()?;
    fs::write(&config.database_file, content).await?;
    
    println!("Uninstalled {}", package);
    Ok(())
}
