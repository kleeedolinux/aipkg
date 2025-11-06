use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::install;
use crate::repo;
use crate::upgrade;
use crate::uninstall;
use crate::query;
use crate::yaml_gen;

#[derive(Parser)]
#[command(name = "aipkg")]
#[command(about = "A full-featured package manager for AppImages", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install an AppImage from local file
    #[command(alias = "-i")]
    Install {
        /// Path to AppImage file
        path: String,
    },
    /// Install a package from repository
    #[command(alias = "-S")]
    Sync {
        /// Package name(s) to install
        packages: Vec<String>,
        /// Update package database before installing
        #[arg(short = 'y')]
        refresh: bool,
    },
    /// Update package database
    #[command(alias = "-Sy")]
    Update,
    /// Upgrade all packages
    #[command(alias = "-Su")]
    Upgrade,
    /// Remove a package
    #[command(alias = "-R")]
    Remove {
        /// Package name(s) to remove
        packages: Vec<String>,
    },
    /// List installed packages
    #[command(alias = "-Q")]
    Query {
        /// Show detailed information
        #[arg(short = 'i')]
        info: bool,
        /// Package name to query (optional)
        package: Option<String>,
    },
    /// Search remote packages
    #[command(alias = "-Ss")]
    Search {
        /// Search query
        query: String,
    },
    /// Show package information
    #[command(alias = "-Si")]
    Info {
        /// Package name
        package: String,
    },
    /// Add a repository source
    AddSource {
        /// Source URL
        url: String,
    },
    /// Remove a repository source
    RemoveSource {
        /// Source URL
        url: String,
    },
    /// List all sources
    ListSources,
    /// Manage collectives
    Collectives {
        #[command(subcommand)]
        cmd: CollectiveCommands,
    },
    /// YAML generation tools
    Yaml {
        #[command(subcommand)]
        cmd: YamlCommands,
    },
}

#[derive(Subcommand)]
pub enum CollectiveCommands {
    /// Add sources to a collective
    Add {
        /// Collective name
        name: String,
        /// Source URL(s)
        urls: Vec<String>,
    },
    /// Remove a collective
    Remove {
        /// Collective name
        name: String,
    },
    /// List all collectives
    List,
}

#[derive(Subcommand)]
pub enum YamlCommands {
    /// Generate appimage.yaml
    Appimage {
        #[command(subcommand)]
        cmd: AppimageYamlCommands,
    },
}

#[derive(Subcommand)]
pub enum AppimageYamlCommands {
    /// Generate new appimage.yaml
    New {
        /// Folder containing AppImages
        folder: String,
        /// GitHub owner/repo (e.g., "user/repo")
        repo: String,
    },
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

pub async fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Install { path } => {
            install::install_from_file(&path).await?;
        }
        Commands::Sync { packages, refresh } => {
            if refresh {
                repo::update_database().await?;
            }
            for package in packages {
                install::install_from_repo(&package).await?;
            }
        }
        Commands::Update => {
            repo::update_database().await?;
        }
        Commands::Upgrade => {
            upgrade::upgrade_all().await?;
        }
        Commands::Remove { packages } => {
            for package in packages {
                uninstall::uninstall(&package).await?;
            }
        }
        Commands::Query { info, package } => {
            query::query_packages(info, package.as_deref()).await?;
        }
        Commands::Search { query } => {
            query::search_packages(&query).await?;
        }
        Commands::Info { package } => {
            query::show_package_info(&package).await?;
        }
        Commands::AddSource { url } => {
            repo::add_source(&url).await?;
        }
        Commands::RemoveSource { url } => {
            repo::remove_source(&url).await?;
        }
        Commands::ListSources => {
            repo::list_sources().await?;
        }
        Commands::Collectives { cmd } => {
            match cmd {
                CollectiveCommands::Add { name, urls } => {
                    repo::collectives::add_to_collective(&name, urls).await?;
                }
                CollectiveCommands::Remove { name } => {
                    repo::collectives::remove_collective(&name).await?;
                }
                CollectiveCommands::List => {
                    repo::collectives::list_collectives().await?;
                }
            }
        }
        Commands::Yaml { cmd } => {
            match cmd {
                YamlCommands::Appimage { cmd } => {
                    match cmd {
                        AppimageYamlCommands::New { folder, repo } => {
                            yaml_gen::generate_appimage_yaml(&folder, &repo).await?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

