# aipkg - AppImage Package Manager

A full-featured, community-driven package manager for AppImages built in Rust.

## Vision

aipkg was created to solve a fundamental problem with AppImages: while they're portable and self-contained, there was no unified way to discover, install, and manage them. We envisioned a decentralized, community-driven ecosystem where anyone can host their own repository, and users can easily discover and install packages from multiple sources.

Unlike traditional package managers that rely on centralized repositories, aipkg embraces decentralization. Repositories can be hosted anywhere - GitHub, GitLab, personal servers, or any web-accessible location. The system aggregates these sources into a unified index, making it feel like a single repository while maintaining the flexibility of multiple independent sources.

## How It Works

### Architecture Overview

aipkg operates on a simple but powerful principle: aggregate, cache, and resolve. When you run `aipkg update`, the system:

1. **Fetches all sources** from your configured repositories
2. **Recursively resolves** any meta-repositories (index.yaml files that point to other repositories)
3. **Flattens everything** into a unified index of all available packages
4. **Caches the result** locally for fast queries and incremental updates

This unified index is stored in `~/.cache/aipkg/unified_index.yaml` and is used for all package operations. The system tracks source hashes to enable incremental updates - if a repository hasn't changed, it skips re-fetching it entirely.

### Repository System

aipkg supports two types of repository files:

**appimage.yaml** - A direct list of AppImage packages:
```yaml
apps:
  - name: myapp
    version: "1.0.0"
    file: releases/myapp-1.0.0.AppImage
    sha256: "abc123..."
    size: 52428800
    description: "My application"
    dependencies:
      - libappimage
    provides:
      - image-viewer
```

**index.yaml** - A meta-repository that aggregates other repositories:
```yaml
sources:
  - type: appimage
    url: https://github.com/user/repo/raw/main/appimage.yaml
  - type: index
    url: https://example.com/index.yaml
```

The resolver recursively follows index.yaml files, flattening all nested repositories into a single unified index. This allows communities to create curated collections of repositories without requiring users to manually add each source.

### Collectives

Collectives are a way to group multiple repositories under a single identifier. Instead of adding individual repositories one by one, you can create a collective that references multiple sources. When you add a collective, all its repositories are automatically included in your unified index.

This is particularly useful for community-maintained collections where multiple people contribute repositories, or when you want to organize repositories by category (e.g., "development-tools", "games", "multimedia").

### Dependency Resolution

When you install a package, aipkg automatically resolves and installs its dependencies. The resolver:

1. Reads the package's dependency list
2. Searches the unified index for each dependency
3. Uses fuzzy matching to find the best match if an exact name isn't found
4. Recursively resolves dependencies of dependencies
5. Installs everything in the correct order

The system uses semantic versioning where available, but also supports simple version strings. If multiple versions of a dependency are available, it selects the best match based on version requirements.

### Installation Process

When you install a package, aipkg:

1. **Downloads** the AppImage file from the repository
2. **Verifies** the SHA256 checksum to ensure integrity
3. **Extracts metadata** from the AppImage (name, version, icon, description)
4. **Creates** a versioned installation directory (`~/.local/share/aipkg/appimages/package-name/version/`)
5. **Generates** a desktop file for integration with your desktop environment
6. **Creates** a symlink in `~/.local/bin/` so you can run it from the command line
7. **Records** the installation in the package database

This ensures that:
- Multiple versions can coexist
- Desktop environments recognize the application
- The application is available in your PATH
- You can track what's installed and when

### Security

Every package installation requires SHA256 verification. The checksum is provided in the repository metadata and is verified before installation proceeds. This ensures that:
- The downloaded file hasn't been corrupted
- The file matches what the repository maintainer intended
- No tampering occurred during download

## Features

- **Installation & Integration**: Install AppImages with automatic desktop file and CLI symlink creation
- **Repository System**: Support for multiple repositories hosted on GitHub or via raw URLs
- **Meta-Repositories**: Recursive index aggregation for community-driven package discovery
- **Collectives**: Group multiple repositories under a single identifier
- **Dependency Resolution**: Automatic dependency resolution with best match selection
- **SHA256 Verification**: Mandatory integrity checking for all packages
- **YAML Automation**: Generate `appimage.yaml` files automatically from AppImage folders
- **Pacman-style CLI**: Familiar commands like `-S`, `-R`, `-Q`, `-Ss`, etc.
- **Incremental Updates**: Smart caching with hash-based change detection for fast updates

## Installation

```bash
cargo build --release
sudo cp target/release/aipkg /usr/local/bin/
```

## Usage

### Basic Commands

```bash
# Install from local file
aipkg install /path/to/app.AppImage
# or
aipkg -i /path/to/app.AppImage

# Install from repository
aipkg sync package-name
# or
aipkg -S package-name

# Update package database
aipkg update
# or
aipkg -Sy

# Upgrade all packages
aipkg upgrade
# or
aipkg -Su

# Remove a package
aipkg remove package-name
# or
aipkg -R package-name

# List installed packages
aipkg query
# or
aipkg -Q

# Search remote packages
aipkg search query
# or
aipkg -Ss query

# Show package information
aipkg info package-name
# or
aipkg -Si package-name
```

### Repository Management

```bash
# Add a repository source
aipkg add-source https://github.com/user/repo/raw/main/appimage.yaml

# Remove a repository source
aipkg remove-source https://github.com/user/repo/raw/main/appimage.yaml

# List all sources
aipkg list-sources
```

### Collectives

```bash
# Create or add to a collective
aipkg collectives add my-collective https://github.com/user/repo/raw/main/appimage.yaml

# Remove a collective
aipkg collectives remove my-collective

# List all collectives
aipkg collectives list
```

### YAML Generation

```bash
# Generate appimage.yaml from a folder
aipkg yaml appimage new /path/to/folder owner/repo
```

## Repository Format

### appimage.yaml

```yaml
apps:
  - name: myapp
    version: "1.0.0"
    file: releases/myapp-1.0.0.AppImage
    sha256: "abc123..."
    size: 52428800
    description: "My application"
    dependencies:
      - libappimage
    provides:
      - image-viewer
```

### index.yaml

```yaml
sources:
  - type: appimage
    url: https://github.com/user/repo/raw/main/appimage.yaml
  - type: index
    url: https://example.com/index.yaml
```

## Configuration

Configuration files are stored in `~/.config/aipkg/`:
- `config.toml` - Main configuration
- `sources.yaml` - Repository sources
- `collectives.yaml` - Collectives definitions
- `database.yaml` - Installed packages database

Cache files are stored in `~/.cache/aipkg/`:
- `unified_index.yaml` - Unified package index
- `cache_metadata.yaml` - Source hash tracking for incremental updates

## Design Decisions

### Why Decentralized?

Centralized package repositories create bottlenecks and single points of failure. By allowing anyone to host their own repository, we enable:
- Faster iteration (no approval process)
- Community ownership (maintainers control their packages)
- Resilience (if one repository goes down, others continue working)
- Flexibility (specialized repositories for specific use cases)

### Why YAML?

YAML is human-readable, easy to edit, and widely supported. Repository maintainers can create and update repositories without special tools. The format is simple enough to write by hand but structured enough for automated processing.

### Why Incremental Updates?

Fetching all repositories on every update would be slow and wasteful. By tracking source hashes, we only re-fetch repositories that have actually changed. This makes updates fast even with dozens of repositories configured.

### Why Versioned Installations?

AppImages are self-contained, but sometimes you need multiple versions (testing, compatibility, etc.). By installing each version in its own directory, we avoid conflicts while maintaining a clean structure.

## License

MIT
