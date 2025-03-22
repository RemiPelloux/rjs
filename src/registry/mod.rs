use anyhow::{Context, Result};
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
    pub dist: DistInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistInfo {
    pub shasum: String,
    pub tarball: String,
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub versions: HashMap<String, VersionInfo>,
    pub dist_tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct NpmPackageVersion {
    version: String,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
    dist: DistInfo,
}

#[derive(Debug, Deserialize)]
struct NpmPackageResponse {
    name: String,
    versions: HashMap<String, NpmPackageVersion>,
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
}

pub struct NpmRegistry {
    client: Client,
    registry_url: String,
}

impl NpmRegistry {
    pub fn new() -> Self {
        Self::with_registry(DEFAULT_REGISTRY)
    }

    pub fn with_registry(registry_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            registry_url: registry_url.to_string(),
        }
    }

    pub async fn get_package_info(&self, package_name: &str) -> Result<PackageInfo> {
        let url = format!("{}/{}", self.registry_url, package_name);
        debug!("Fetching package info from {}", url);

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

        // Convert to our internal model
        let versions = npm_package
            .versions
            .into_iter()
            .map(|(version, npm_version)| {
                let version_info = VersionInfo {
                    version: version.clone(),
                    dependencies: npm_version.dependencies.unwrap_or_default(),
                    dev_dependencies: npm_version.dev_dependencies.unwrap_or_default(),
                    dist: npm_version.dist,
                };
                (version, version_info)
            })
            .collect();

        Ok(PackageInfo {
            name: npm_package.name,
            versions,
            dist_tags: npm_package.dist_tags,
        })
    }

    pub async fn download_package(
        &self,
        tarball_url: &str,
        output_path: &std::path::Path,
    ) -> Result<()> {
        debug!("Downloading package from {}", tarball_url);

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

        let bytes = response
            .bytes()
            .await
            .with_context(|| "Failed to read response body")?;

        std::fs::write(output_path, bytes)
            .with_context(|| format!("Failed to write package to {}", output_path.display()))?;

        Ok(())
    }

    // Helper method to extract a tarball
    pub fn extract_tarball(
        &self,
        tarball_path: &std::path::Path,
        output_dir: &std::path::Path,
    ) -> Result<()> {
        debug!(
            "Extracting tarball {} to {}",
            tarball_path.display(),
            output_dir.display()
        );

        let file = std::fs::File::open(tarball_path)
            .with_context(|| format!("Failed to open tarball {}", tarball_path.display()))?;

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

        Ok(())
    }
}
