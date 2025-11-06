use anyhow::Result;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::config::Config;
use crate::repo::cache::load_unified_index;
use crate::install::load_database;

pub async fn query_packages(info: bool, package: Option<&str>) -> Result<()> {
    let config = Config::new()?;
    let db = load_database(&config).await?;
    
    if let Some(pkg_name) = package {
        if let Some(pkg) = db.get_package(pkg_name) {
            if info {
                println!("Name: {}", pkg.name);
                println!("Version: {}", pkg.version);
                println!("Path: {}", pkg.path.display());
                println!("Installed at: {}", pkg.installed_at);
            } else {
                println!("{} {}", pkg.name, pkg.version);
            }
        } else {
            println!("Package not installed: {}", pkg_name);
        }
    } else {
        let packages = db.list_packages();
        if packages.is_empty() {
            println!("No packages installed");
        } else {
            for pkg in packages {
                if info {
                    println!("{} {} - {}", pkg.name, pkg.version, pkg.path.display());
                } else {
                    println!("{} {}", pkg.name, pkg.version);
                }
            }
        }
    }
    
    Ok(())
}

pub async fn search_packages(query: &str) -> Result<()> {
    let index = load_unified_index().await?;
    
    let matcher = SkimMatcherV2::default();
    let mut matches: Vec<(&String, &crate::repo::appimage_yaml::AppImageEntryWithSource, i64)> = Vec::new();
    
    for (name, entries) in &index.apps {
        for entry in entries {
            if let Some(score) = matcher.fuzzy_match(name, query) {
                matches.push((name, entry, score));
            }
        }
    }
    
    // Sort by score (descending)
    matches.sort_by_key(|(_, _, score)| -score);
    
    if matches.is_empty() {
        println!("No packages found matching: {}", query);
    } else {
        println!("Found {} package(s):", matches.len());
        for (name, entry, _) in matches.iter().take(20) {
            println!("  {} {} - {}", name, entry.entry.version, 
                entry.entry.description.as_deref().unwrap_or(""));
        }
    }
    
    Ok(())
}

pub async fn show_package_info(package: &str) -> Result<()> {
    let index = load_unified_index().await?;
    
    // Try exact match first
    if let Some(entry) = index.find_best_match(package, None) {
        println!("Name: {}", entry.entry.name);
        println!("Version: {}", entry.entry.version);
        if let Some(desc) = &entry.entry.description {
            println!("Description: {}", desc);
        }
        if let Some(size) = entry.entry.size {
            println!("Size: {} bytes ({:.2} MB)", size, size as f64 / 1_000_000.0);
        }
        println!("SHA256: {}", entry.entry.sha256);
        println!("Source: {}", entry.source_url);
        if !entry.entry.dependencies.is_empty() {
            println!("Dependencies: {}", entry.entry.dependencies.join(", "));
        }
        if !entry.entry.provides.is_empty() {
            println!("Provides: {}", entry.entry.provides.join(", "));
        }
    } else {
        // Try fuzzy match
        let matcher = SkimMatcherV2::default();
        let mut best_match: Option<&crate::repo::appimage_yaml::AppImageEntryWithSource> = None;
        let mut best_score = 0;
        
        for (name, entries) in &index.apps {
            for entry in entries {
                if let Some(score) = matcher.fuzzy_match(name, package) {
                    if score > best_score {
                        best_score = score;
                        best_match = Some(entry);
                    }
                }
            }
        }
        
        if let Some(entry) = best_match {
            println!("Name: {}", entry.entry.name);
            println!("Version: {}", entry.entry.version);
            if let Some(desc) = &entry.entry.description {
                println!("Description: {}", desc);
            }
            if let Some(size) = entry.entry.size {
                println!("Size: {} bytes ({:.2} MB)", size, size as f64 / 1_000_000.0);
            }
            println!("SHA256: {}", entry.entry.sha256);
            println!("Source: {}", entry.source_url);
        } else {
            anyhow::bail!("Package not found: {}", package);
        }
    }
    
    Ok(())
}

