use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::time::Duration;
use futures_util::StreamExt;

pub struct Fetcher {
    client: Client,
}

impl Fetcher {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("aipkg/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;
        
        Ok(Self { client })
    }

    pub async fn fetch_yaml(&self, url: &str) -> Result<String> {
        let url = self.normalize_github_url(url)?;
        
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} Fetching {msg}...")
                .unwrap()
        );
        pb.set_message(url.clone());
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to fetch: {}", url))?;
        
        pb.finish_with_message("Done");
        
        if !response.status().is_success() {
            anyhow::bail!("HTTP error {}: {}", response.status(), url);
        }
        
        let content = response.text().await
            .context(format!("Failed to read response from: {}", url))?;
        
        Ok(content)
    }

    pub async fn fetch_appimage(&self, url: &str, expected_size: Option<u64>) -> Result<Vec<u8>> {
        let url = self.normalize_github_url(url)?;
        
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context(format!("Failed to fetch AppImage: {}", url))?;
        
        if !response.status().is_success() {
            anyhow::bail!("HTTP error {}: {}", response.status(), url);
        }
        
        let total_size = expected_size
            .or_else(|| response.content_length())
            .unwrap_or(0);
        
        pb.set_length(total_size);
        
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        
        while let Some(item) = stream.next().await {
            let chunk = item.context("Failed to read chunk")?;
            bytes.extend_from_slice(&chunk);
            pb.set_position(bytes.len() as u64);
        }
        
        pb.finish_with_message("Download complete");
        
        Ok(bytes)
    }

    fn normalize_github_url(&self, url: &str) -> Result<String> {
        // Convert GitHub blob URLs to raw URLs
        if url.contains("github.com") && url.contains("/blob/") {
            Ok(url.replace("/blob/", "/raw/"))
        } else {
            Ok(url.to_string())
        }
    }
}

impl Default for Fetcher {
    fn default() -> Self {
        Self::new().expect("Failed to create fetcher")
    }
}

