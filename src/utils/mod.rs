use anyhow::{Context, Result};
use log::debug;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use url::Url;

// File system utilities
#[allow(dead_code)]
pub async fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        debug!("Creating directory: {}", path.display());
        fs::create_dir_all(path)
            .await
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn write_file(path: &Path, content: &[u8]) -> Result<()> {
    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }

    debug!("Writing file: {}", path.display());
    fs::write(path, content)
        .await
        .with_context(|| format!("Failed to write file: {}", path.display()))?;

    Ok(())
}

#[allow(dead_code)]
pub async fn read_file(path: &Path) -> Result<Vec<u8>> {
    debug!("Reading file: {}", path.display());
    fs::read(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))
}

#[allow(dead_code)]
pub async fn read_file_string(path: &Path) -> Result<String> {
    debug!("Reading file as string: {}", path.display());
    fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file as string: {}", path.display()))
}

#[allow(dead_code)]
pub async fn file_exists(path: &Path) -> bool {
    path.exists()
}

// Hash utilities
#[allow(dead_code)]
pub fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

#[allow(dead_code)]
pub async fn calculate_file_sha256(path: &Path) -> Result<String> {
    let content = read_file(path).await?;
    Ok(calculate_sha256(&content))
}

// URL utilities
#[allow(dead_code)]
pub fn get_package_name_from_url(url_str: &str) -> Result<String> {
    let url = Url::parse(url_str).with_context(|| format!("Failed to parse URL: {}", url_str))?;

    let path = url.path();
    let segments: Vec<&str> = path.split('/').collect();

    // The package name is typically the last segment before the version
    // This is a simplified approach and may need refinement
    segments
        .last()
        .map(|s| s.trim_end_matches(".tgz").to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract package name from URL: {}", url_str))
}

// Path utilities
#[allow(dead_code)]
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine cache directory"))?
        .join("rjs");

    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;
    }

    Ok(cache_dir)
}

#[allow(dead_code)]
pub fn get_temp_dir() -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join("rjs");

    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir)
            .with_context(|| format!("Failed to create temp directory: {}", temp_dir.display()))?;
    }

    Ok(temp_dir)
}
