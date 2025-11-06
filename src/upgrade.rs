use anyhow::Result;
use semver::Version;

use crate::config::Config;
use crate::repo::cache::load_unified_index;
use crate::install::{install_appimage_entry, load_database};

pub async fn upgrade_all() -> Result<()> {
    let config = Config::new()?;
    
    // Load installed packages
    let db = load_database(&config).await?;
    
    // Load unified index
    let index = load_unified_index().await?;
    
    let mut upgraded = 0;
    
    for pkg in db.list_packages() {
        // Find latest version in index
        if let Some(latest_entry) = index.find_best_match(&pkg.name, None) {
            // Compare versions
            let current_version = Version::parse(&pkg.version).ok();
            let latest_version = Version::parse(&latest_entry.entry.version).ok();
            
            if let (Some(current), Some(latest)) = (current_version, latest_version) {
                if latest > current {
                    println!("Upgrading {} from {} to {}", 
                        pkg.name, pkg.version, latest_entry.entry.version);
                    
                    // Uninstall old version
                    crate::uninstall::uninstall(&pkg.name).await?;
                    
                    // Install new version
                    install_appimage_entry(&config, latest_entry, &index).await?;
                    
                    upgraded += 1;
                }
            }
        }
    }
    
    if upgraded == 0 {
        println!("All packages are up to date");
    } else {
        println!("Upgraded {} package(s)", upgraded);
    }
    
    Ok(())
}
