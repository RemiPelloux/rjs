use anyhow::{Context, Result};
use futures::StreamExt;
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use std::sync::Arc;

const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VersionInfo {
    pub version: String,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
    pub dist: DistInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DistInfo {
    pub shasum: String,
    pub tarball: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PackageInfo {
    pub name: String,
    pub versions: HashMap<String, VersionInfo>,
    pub dist_tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NpmPackageVersion {
    version: String,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
    dist: DistInfo,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NpmPackageResponse {
    name: String,
    versions: HashMap<String, NpmPackageVersion>,
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct NpmRegistry {
    client: Client,
    registry_url: String,
    rate_limiter: Arc<Semaphore>,
}

impl NpmRegistry {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_registry(DEFAULT_REGISTRY)
    }

    pub fn with_registry(registry_url: &str) -> Self {
        // Create a client with connection pooling and http2
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .http2_keep_alive_interval(Some(Duration::from_secs(20)))
            .http2_keep_alive_timeout(Duration::from_secs(20))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            registry_url: registry_url.to_string(),
            // Allow up to 100 concurrent HTTP requests
            rate_limiter: Arc::new(Semaphore::new(100)),
        }
    }

    #[allow(dead_code)]
    pub async fn get_package_info(&self, package_name: &str) -> Result<PackageInfo> {
        let start = Instant::now();
        let url = format!("{}/{}", self.registry_url, package_name);
        debug!("Fetching package info from {}", url);

        // Acquire permit for rate limiting
        let _permit = self.rate_limiter.acquire().await?;

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .with_context(|| format!("Failed to fetch package info for {}", package_name))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch package {}: HTTP {}",
                package_name,
                response.status()
            ));
        }

        let npm_package: NpmPackageResponse = response
            .json()
            .await
            .with_context(|| format!("Failed to parse package info for {}", package_name))?;

        // Convert to our internal model with zero-copy optimization
        let mut versions = HashMap::with_capacity(npm_package.versions.len());
        for (version, npm_version) in npm_package.versions {
            let version_info = VersionInfo {
                version: version.clone(),
                dependencies: npm_version.dependencies.unwrap_or_default(),
                dev_dependencies: npm_version.dev_dependencies.unwrap_or_default(),
                dist: npm_version.dist,
            };
            versions.insert(version, version_info);
        }

        debug!("Fetched {} package info in {:?}", package_name, start.elapsed());

        Ok(PackageInfo {
            name: npm_package.name,
            versions,
            dist_tags: npm_package.dist_tags,
        })
    }

    #[allow(dead_code)]
    pub async fn download_package(
        &self,
        tarball_url: &str,
        output_path: &std::path::Path,
    ) -> Result<()> {
        let start = Instant::now();
        debug!("Downloading package from {}", tarball_url);

        // Acquire permit for rate limiting
        let _permit = self.rate_limiter.acquire().await?;

        // Use streaming to handle large tarballs efficiently
        let response = self
            .client
            .get(tarball_url)
            .send()
            .await
            .with_context(|| format!("Failed to download package from {}", tarball_url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download package: HTTP {}",
                response.status()
            ));
        }

        // Get content length for progress tracking
        let total_size = response
            .content_length()
            .unwrap_or(0);

        // Create file for streaming
        let mut file = fs::File::create(output_path).await
            .with_context(|| format!("Failed to create file {}", output_path.display()))?;

        // Stream the download in chunks
        let mut stream = response.bytes_stream();
        let mut downloaded = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.with_context(|| format!("Error while downloading {}", tarball_url))?;
            file.write_all(&chunk).await
                .with_context(|| format!("Failed to write to {}", output_path.display()))?;
            
            downloaded += chunk.len() as u64;
            
            // Log progress for large packages
            if total_size > 1024 * 1024 && downloaded % (1024 * 1024) == 0 {
                debug!(
                    "Downloaded {:.1}MB of {:.1}MB ({:.1}%)",
                    downloaded as f64 / 1024.0 / 1024.0,
                    total_size as f64 / 1024.0 / 1024.0,
                    (downloaded as f64 / total_size as f64) * 100.0
                );
            }
        }

        // Ensure all data is flushed to disk
        file.flush().await
            .with_context(|| format!("Failed to flush file {}", output_path.display()))?;

        debug!(
            "Downloaded {}KB in {:?}", 
            downloaded / 1024, 
            start.elapsed()
        );

        Ok(())
    }

    // Helper method to extract a tarball using tokio
    #[allow(dead_code)]
    pub fn extract_tarball(
        &self,
        tarball_path: &std::path::Path,
        output_dir: &std::path::Path,
    ) -> Result<()> {
        let start = Instant::now();
        debug!(
            "Extracting tarball {} to {}",
            tarball_path.display(),
            output_dir.display()
        );

        // Open the tarball file
        let file = std::fs::File::open(tarball_path)
            .with_context(|| format!("Failed to open tarball {}", tarball_path.display()))?;

        // Create a decompression reader
        let decompressed = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressed);

        // Create the output directory if it doesn't exist
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)
                .with_context(|| format!("Failed to create directory {}", output_dir.display()))?;
        }

        // Extract the tarball to the output directory
        archive
            .unpack(output_dir)
            .with_context(|| format!("Failed to extract tarball to {}", output_dir.display()))?;

        debug!("Extracted tarball in {:?}", start.elapsed());

        Ok(())
    }

    // Add a method to get the registry URL
    pub fn get_registry_url(&self) -> &str {
        &self.registry_url
    }
}
