# Hosting Your Own AppImage Repository

This guide explains how to host your own AppImage repository or create an index that aggregates multiple repositories. Understanding how aipkg works internally will help you set up repositories correctly and build custom solutions on top of the system.

## Repository Types

aipkg supports two types of repository files:

### appimage.yaml

A direct list of AppImage packages. This is what you create when you have AppImages to distribute.

```yaml
apps:
  - name: myapp
    version: "1.0.0"
    file: releases/myapp-1.0.0.AppImage
    sha256: "abc123def456..."
    size: 52428800
    description: "My application"
    dependencies:
      - libappimage
    provides:
      - image-viewer
```

**Required fields:**
- `name`: Package name
- `version`: Version string (semantic versioning recommended)
- `file`: Relative path to the AppImage file
- `sha256`: SHA256 checksum (64 hex characters)

**Optional fields:**
- `size`: File size in bytes
- `description`: Human-readable description
- `dependencies`: List of package names this depends on
- `provides`: List of virtual packages this provides

### index.yaml

A meta-repository that aggregates other repositories. Use this to create curated collections or organize multiple sources.

```yaml
sources:
  - type: appimage
    url: https://github.com/user/repo/raw/main/appimage.yaml
  - type: index
    url: https://example.com/index.yaml
```

**Fields:**
- `type`: Either `appimage` or `index`
- `url`: Full URL to the repository file

Index files can reference other index files, creating nested structures. aipkg automatically flattens everything into a unified index.

## How aipkg Works

When a user runs `aipkg update`, here's what happens:

1. **Source Loading**: aipkg reads all repository URLs from:
   - `~/.config/aipkg/sources.yaml` (direct sources)
   - `~/.config/aipkg/collectives.yaml` (grouped sources)

2. **Incremental Updates**: For each source URL, aipkg:
   - Calculates a SHA256 hash of the YAML content
   - Compares it with the cached hash from previous updates
   - Skips fetching if the hash hasn't changed (fast updates)

3. **Recursive Resolution**: For each source:
   - Fetches the YAML file via HTTP/HTTPS
   - If it's an `index.yaml`, recursively follows all referenced sources
   - If it's an `appimage.yaml`, extracts all package entries
   - Resolves relative URLs using the YAML file's location as base
   - Tracks visited URLs to prevent infinite loops

4. **Unified Index**: All packages from all sources are flattened into a single unified index stored at `~/.cache/aipkg/unified_index.yaml`. Each entry includes:
   - Package metadata (name, version, description, etc.)
   - Source URL (where it came from)
   - Resolved download URL (base URL + relative file path)

5. **Installation**: When installing a package:
   - Resolves the download URL by joining the source URL with the `file` field
   - Downloads the AppImage
   - Verifies SHA256 checksum
   - Installs to `~/.local/share/aipkg/appimages/package-name/version/`

This architecture means:
- Repositories are independent and can be hosted anywhere
- The YAML structure is the common protocol everyone follows
- You can build custom tools as long as they generate valid YAML
- The system is decentralized - no central authority required

## Hosting Options

### AppImages on GitHub

The most common setup is hosting AppImages on GitHub:

1. Create a repository
2. Upload your AppImage files (e.g., in a `releases/` folder)
3. Create an `appimage.yaml` file in the root
4. Use the raw URL format: `https://github.com/owner/repo/raw/branch/appimage.yaml`

aipkg automatically converts GitHub blob URLs to raw URLs, so either format works.

**Why GitHub?** It's free, reliable, and provides CDN-backed file hosting. But AppImages can be hosted anywhere - GitLab, personal servers, object storage (S3, etc.), or any HTTP-accessible location.

### Index Files Anywhere

While AppImages are commonly hosted on GitHub, `index.yaml` files can be hosted on **any web server**:

- Your own domain: `https://example.com/index.yaml`
- GitLab: `https://gitlab.com/user/repo/raw/main/index.yaml`
- GitHub Pages: `https://username.github.io/repo/index.yaml`
- Static hosting: Netlify, Vercel, Cloudflare Pages
- Object storage: S3, Google Cloud Storage, Azure Blob Storage
- Any HTTP/HTTPS endpoint

This flexibility lets you:
- Create curated indexes without hosting AppImages yourself
- Build custom repository management systems
- Aggregate repositories from multiple sources
- Create community-maintained collections

### Other Web Servers

Any web server that can serve files over HTTP/HTTPS works. Just make sure:
- The YAML file is accessible via URL
- AppImage files are accessible via the paths specified in `file` fields
- URLs are absolute or relative to the YAML file location
- CORS headers allow cross-origin requests (if needed)

## Generating appimage.yaml

aipkg includes a built-in utility command to generate `appimage.yaml` files automatically. This is the easiest way to create a repository:

```bash
aipkg yaml appimage new /path/to/folder owner/repo
```

**What it does:**
- Scans the folder for all `.AppImage` files
- **Automatically calculates SHA256 checksums** for each file
- Extracts metadata from AppImages (name, version, description, size)
- Attempts to extract version from filename if not in metadata
- Generates `appimage.yaml` in the folder with all required fields

**Example:**
```bash
# You have a folder with AppImages
~/my-apps/
├── myapp-1.0.0.AppImage
├── myapp-1.1.0.AppImage
└── other-app-2.0.0.AppImage

# Generate the YAML file
aipkg yaml appimage new ~/my-apps/ myusername/myrepo

# Result: ~/my-apps/appimage.yaml is created with:
# - All three AppImages listed
# - SHA256 checksums already calculated
# - Metadata extracted automatically
```

**After generation:**
- Review the generated `appimage.yaml`
- Manually add `dependencies` and `provides` fields if needed
- Adjust descriptions if the extracted metadata isn't ideal
- Verify file paths are correct for your hosting setup

**Manual creation:**
If you prefer to create the YAML manually, you'll need to:
- Calculate SHA256 checksums yourself: `sha256sum file.AppImage`
- Extract metadata from AppImages
- Write the YAML structure following the format above

The utility command saves time by handling checksum calculation and metadata extraction automatically.

## File Paths and URL Resolution

The `file` field in `appimage.yaml` can be:
- **Relative**: Relative to the YAML file's URL
- **Absolute**: Full URL to the AppImage

aipkg uses URL joining to resolve relative paths. If your `appimage.yaml` is at:
```
https://github.com/owner/repo/raw/main/appimage.yaml
```

And you specify:
```yaml
file: releases/myapp-1.0.0.AppImage
```

aipkg resolves it to:
```
https://github.com/owner/repo/raw/main/releases/myapp-1.0.0.AppImage
```

**Important**: The base URL is always the YAML file's location, not the repository root. This means:
- If your YAML is in a subdirectory, relative paths are relative to that subdirectory
- You can organize files however you want - just make sure paths are correct
- Absolute URLs work from any location

**Examples**:
```yaml
# Relative path (recommended)
file: releases/myapp-1.0.0.AppImage

# Absolute URL (works from anywhere)
file: https://example.com/downloads/myapp-1.0.0.AppImage

# Relative to subdirectory
# If YAML is at: https://example.com/repo/v1/appimage.yaml
# And file is at: https://example.com/repo/v1/binaries/myapp.AppImage
file: binaries/myapp.AppImage
```

## Creating an Index

To aggregate multiple repositories, create an `index.yaml`:

```yaml
sources:
  - type: appimage
    url: https://github.com/user1/repo1/raw/main/appimage.yaml
  - type: appimage
    url: https://github.com/user2/repo2/raw/main/appimage.yaml
  - type: index
    url: https://community.example.com/index.yaml
```

**How Index Resolution Works**:

When a user adds your index, aipkg:

1. Fetches your `index.yaml` file
2. For each source in the list:
   - If `type: appimage`: Fetches the YAML and extracts packages
   - If `type: index`: Recursively fetches and processes that index too
3. Flattens everything into a single unified index (no nested structure)
4. Tracks visited URLs to prevent infinite loops
5. Caches the result for fast queries

**Nested Indexes**:

Indexes can reference other indexes, creating nested structures:

```yaml
# Main index at https://example.com/main-index.yaml
sources:
  - type: index
    url: https://dev-tools.example.com/index.yaml
  - type: index
    url: https://games.example.com/index.yaml
```

aipkg will recursively follow all nested indexes and flatten them. This lets you:
- Create hierarchical organization (categories, subcategories)
- Delegate curation to different maintainers
- Build complex repository networks

**Relative URLs in Indexes**:

URLs in `index.yaml` can be relative too:

```yaml
# If index.yaml is at https://example.com/repo/index.yaml
sources:
  - type: appimage
    url: ../other-repo/appimage.yaml  # Resolves to https://example.com/other-repo/appimage.yaml
  - type: index
    url: subdirectory/index.yaml      # Resolves to https://example.com/repo/subdirectory/index.yaml
```

This lets you organize related repositories together.

## Best Practices

**Versioning:**
- Use semantic versioning (e.g., `1.2.3`) when possible
- Keep versions consistent across releases

**SHA256 Checksums:**
- Always include SHA256 checksums
- Verify checksums before publishing
- Use `sha256sum` command: `sha256sum file.AppImage`

**File Organization:**
- Keep AppImages in a dedicated folder (e.g., `releases/`)
- Use consistent naming: `appname-version.AppImage`
- Include architecture in filename if relevant

**Descriptions:**
- Write clear, concise descriptions
- Include what the app does, not just marketing copy

**Dependencies:**
- List actual runtime dependencies
- Use package names that exist in other repositories
- Keep dependency lists minimal

**Updates:**
- Update the YAML file when releasing new versions
- Keep old versions for compatibility
- Test that URLs resolve correctly

## Example Repository Structure

```
my-repo/
├── appimage.yaml
└── releases/
    ├── myapp-1.0.0.AppImage
    ├── myapp-1.1.0.AppImage
    └── myapp-2.0.0.AppImage
```

## Testing Your Repository

Before sharing your repository:

1. Validate the YAML syntax
2. Verify all file URLs are accessible
3. Check that SHA256 checksums match
4. Test with aipkg:
   ```bash
   aipkg add-source https://github.com/owner/repo/raw/main/appimage.yaml
   aipkg update
   aipkg search your-app-name
   ```

## Sharing Your Repository

Once your repository is ready:

1. Make sure it's publicly accessible
2. Share the raw URL to your YAML file
3. Users can add it with:
   ```bash
   aipkg add-source https://github.com/owner/repo/raw/main/appimage.yaml
   ```

For indexes, users add the index URL and get access to all aggregated repositories automatically.

## Building Custom Solutions

The YAML structure is the common protocol that everyone follows. This means:

**You can build your own tools** that generate or manage repositories:
- Custom CI/CD pipelines that auto-generate `appimage.yaml` from releases
- Web interfaces for managing repositories
- Scripts that aggregate repositories from different sources
- Custom package managers that use the same YAML format
- Repository mirrors or proxies

**As long as you follow the YAML structure**, your repositories will work with aipkg and any other tools that understand the format.

**Example Custom Workflow**:

```bash
# Your custom script that generates appimage.yaml
#!/bin/bash
for appimage in releases/*.AppImage; do
    sha256=$(sha256sum "$appimage" | cut -d' ' -f1)
    # Generate YAML entry...
done
```

**Example Custom Index Generator**:

```python
# Python script that creates an index.yaml from a list of repos
repos = [
    "https://github.com/user1/repo1/raw/main/appimage.yaml",
    "https://github.com/user2/repo2/raw/main/appimage.yaml",
]

index = {"sources": [{"type": "appimage", "url": url} for url in repos]}
# Write index.yaml...
```

The key is: **the YAML format is the interface**. Build whatever you want on top of it.

## System Architecture Summary

**Decentralized Design**:
- No central repository or authority
- Anyone can host repositories anywhere
- Users aggregate sources they trust
- The system scales horizontally

**YAML as Protocol**:
- Human-readable and editable
- Machine-parseable
- Version-controllable (Git-friendly)
- Tool-agnostic (any tool can generate/consume it)

**Flexible Hosting**:
- AppImages: GitHub, GitLab, personal servers, object storage, anywhere
- Indexes: Any web-accessible location
- Mix and match hosting solutions
- No vendor lock-in

**Incremental Updates**:
- Hash-based change detection
- Only re-fetch changed repositories
- Fast updates even with many sources
- Efficient bandwidth usage

This architecture enables the ecosystem to grow organically while maintaining compatibility through the shared YAML structure.

