use anyhow::{Context, Result};
use sha2::{Sha256, Digest};
use hex;
use std::path::Path;

pub async fn verify_sha256(file_path: &str, expected_hash: &str) -> Result<bool> {
    let data = tokio::fs::read(file_path).await
        .context(format!("Failed to read file: {}", file_path))?;
    
    verify_sha256_bytes(&data, expected_hash)
}

pub fn verify_sha256_bytes(data: &[u8], expected_hash: &str) -> Result<bool> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let computed_hash = hex::encode(hasher.finalize());
    
    Ok(computed_hash == expected_hash.to_lowercase())
}

pub async fn calculate_sha256(file_path: &str) -> Result<String> {
    let data = tokio::fs::read(file_path).await
        .context(format!("Failed to read file: {}", file_path))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

pub fn calculate_sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
