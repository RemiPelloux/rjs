use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use log::{debug, info};
use semver::VersionReq;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs;
use rayon::prelude::*;
use std::time::Instant;
use crossbeam::queue::SegQueue;
use std::thread;

use crate::registry::NpmRegistry;

#[derive(Clone)]
#[allow(dead_code)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
}

#[allow(dead_code)]
pub struct DependencyTree {
    pub root: Package,
    pub dependencies: HashMap<String, Package>,
}

// Cache for package resolution to avoid redundant network requests
#[derive(Clone)]
struct PackageCache {
    cache: Arc<Mutex<HashMap<String, Arc<Package>>>>,
}

impl PackageCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get(&self, key: &str) -> Option<Arc<Package>> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).cloned()
    }

    fn insert(&self, key: String, package: Package) -> Arc<Package> {
        let package_arc = Arc::new(package);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key, package_arc.clone());
        package_arc
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct DependencyResolver {
    registry: NpmRegistry,
    visited: Arc<Mutex<HashSet<String>>>,
    concurrency: usize,
    package_cache: PackageCache,
    batch_size: usize,
}

impl DependencyResolver {
    #[allow(dead_code)]
    pub fn new(registry: NpmRegistry) -> Self {
        // Use 4x CPU cores for optimal concurrency with async I/O
        let optimal_concurrency = num_cpus::get() * 4;
        
        Self {
            registry,
            visited: Arc::new(Mutex::new(HashSet::new())),
            concurrency: optimal_concurrency,
            package_cache: PackageCache::new(),
            batch_size: 50, // Process packages in batches of 50 for better throughput
        }
    }

    // Allow setting custom concurrency level
    #[allow(dead_code)]
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1); // Ensure at least 1
        self
    }
    
    // Set custom batch size for processing
    #[allow(dead_code)]
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(10).min(100); // Between 10 and 100
        self
    }

    #[allow(dead_code)]
    pub async fn resolve_package(&self, name: &str, version_req: &str) -> Result<Package> {
        let key = format!("{}@{}", name, version_req);
        
        // Check cache first
        if let Some(cached_pkg) = self.package_cache.get(&key) {
            debug!("Cache hit for {}", key);
            return Ok((*cached_pkg).clone());
        }
        
        // Check if already visited using a mutex
        {
            let visited = self.visited.lock().unwrap();
            if visited.contains(&key) {
                debug!("Already visited {}", key);
                // Return a dummy package to avoid circular dependencies for now
                return Ok(Package {
                    name: name.to_string(),
                    version: "0.0.0".to_string(),
                    dependencies: HashMap::new(),
                    dev_dependencies: HashMap::new(),
                });
            }
        }
        
        // Mark as visited
        {
            let mut visited = self.visited.lock().unwrap();
            visited.insert(key.clone());
        }

        // Fetch package info from registry with timing
        let start = Instant::now();
        let package_info = self.registry.get_package_info(name).await?;
        debug!("Fetched package info for {} in {:?}", name, start.elapsed());

        // Find the best matching version
        let version_req_parsed = VersionReq::parse(version_req).unwrap_or(VersionReq::STAR);
        let version_req_str = version_req.to_string(); // Clone for error message

        // Optimize version selection using Rayon parallel iterators
        let versions: Vec<_> = package_info.versions.keys().cloned().collect();
        let best_version = thread::spawn(move || {
            versions.into_par_iter()
                .filter_map(|v| {
                    match semver::Version::parse(&v) {
                        Ok(parsed) => {
                            if version_req_parsed.matches(&parsed) {
                                Some((v, parsed))
                            } else {
                                None
                            }
                        },
                        Err(_) => None,
                    }
                })
                .max_by(|(_, a), (_, b)| a.cmp(b))
                .map(|(v, _)| v)
        }).join().unwrap();
        
        let best_version = best_version
            .with_context(|| format!("No matching version found for {}@{}", name, version_req_str))?;

        debug!(
            "Selected version {} for {}@{}",
            best_version, name, version_req
        );

        let version_info = &package_info.versions[&best_version];

        // Create package
        let package = Package {
            name: name.to_string(),
            version: best_version,
            dependencies: version_info.dependencies.clone(),
            dev_dependencies: version_info.dev_dependencies.clone(),
        };
        
        // Cache the result
        let _ = self.package_cache.insert(key, package.clone());

        Ok(package)
    }

    #[allow(dead_code)]
    pub async fn resolve_dependencies(&self, root_pkg: &Package) -> Result<DependencyTree> {
        let mut dependencies = HashMap::new();
        let dep_entries: Vec<_> = root_pkg.dependencies.iter().collect();
        
        // Use a work-stealing queue for dynamic workload distribution
        let work_queue = Arc::new(SegQueue::new());
        
        // Initialize the queue with dependencies
        for (name, version) in dep_entries {
            work_queue.push((name.clone(), version.clone()));
        }
        
        // Process queue in batches for better throughput
        while !work_queue.is_empty() {
            // Collect a batch of work items
            let mut batch = Vec::new();
            for _ in 0..self.batch_size {
                if let Some((name, version)) = work_queue.pop() {
                    batch.push((name, version));
                } else {
                    break;
                }
            }
            
            if batch.is_empty() {
                break;
            }
            
            // Create a clone of the work queue for the async task
            let work_queue_clone = Arc::clone(&work_queue);
            
            // Process batch concurrently
            let mut stream = stream::iter(batch)
                .map(|(dep_name, dep_version)| {
                    let resolver = self.clone();
                    let queue = Arc::clone(&work_queue_clone);
                    
                    async move {
                        match resolver.resolve_package(&dep_name, &dep_version).await {
                            Ok(pkg) => {
                                // Add nested dependencies to work queue
                                for (nested_name, nested_version) in &pkg.dependencies {
                                    let key = format!("{}@{}", nested_name, nested_version);
                                    let mut visited = resolver.visited.lock().unwrap();
                                    if !visited.contains(&key) {
                                        queue.push((nested_name.clone(), nested_version.clone()));
                                        visited.insert(key);
                                    }
                                }
                                Some((format!("{}@{}", dep_name, dep_version), pkg))
                            },
                            Err(e) => {
                                debug!("Failed to resolve {}@{}: {}", dep_name, dep_version, e);
                                None
                            }
                        }
                    }
                })
                .buffer_unordered(self.concurrency);
                
            while let Some(result) = stream.next().await {
                if let Some((key, pkg)) = result {
                    dependencies.insert(key, pkg);
                }
            }
        }

        Ok(DependencyTree {
            root: root_pkg.clone(),
            dependencies,
        })
    }

    #[allow(dead_code)]
    pub async fn install_tree(&self, tree: &DependencyTree, install_path: &Path) -> Result<()> {
        let start = Instant::now();
        
        // Create node_modules directory if it doesn't exist
        let node_modules = install_path.join("node_modules");
        fs::create_dir_all(&node_modules).await?;

        // Convert dependencies to a vector for parallel processing
        let packages: Vec<_> = tree.dependencies.values().collect();
        let total = packages.len();
        
        debug!("Installing {} packages with concurrency {}", total, self.concurrency);
        
        // Process packages in optimized batches
        for chunk in packages.chunks(self.batch_size) {
            // Install packages concurrently
            let results = stream::iter(chunk)
                .map(|pkg| {
                    let registry = self.registry.clone();
                    let node_modules = node_modules.clone();
                    let pkg = (*pkg).clone();
                    
                    async move {
                        let pkg_dir = node_modules.join(&pkg.name);
                        
                        // Create directory structure in parallel
                        let pkg_dir_clone = pkg_dir.clone();
                        tokio::task::spawn_blocking(move || {
                            std::fs::create_dir_all(&pkg_dir_clone).unwrap();
                        }).await?;

                        debug!(
                            "Installing {} {} to {}",
                            pkg.name,
                            pkg.version,
                            pkg_dir.display()
                        );

                        // In a real implementation, download and extract the package
                        // Get version info to access tarball URL
                        let package_info = registry.get_package_info(&pkg.name).await?;
                        if let Some(version_info) = package_info.versions.get(&pkg.version) {
                            // Download package tarball
                            let tarball_url = &version_info.dist.tarball;
                            let tarball_path = pkg_dir.join("package.tgz");
                            registry.download_package(tarball_url, &tarball_path).await?;
                            
                            // Extract tarball with cloned paths to avoid move issues
                            let tarball_path_clone = tarball_path.clone();
                            let pkg_dir_clone = pkg_dir.clone();
                            let extract_result = tokio::task::spawn_blocking(move || {
                                registry.extract_tarball(&tarball_path_clone, &pkg_dir_clone)
                            }).await?;
                            
                            if let Err(e) = extract_result {
                                return Err(e);
                            }
                            
                            // Remove tarball after extraction
                            fs::remove_file(tarball_path).await?;
                        }

                        // Create a minimal package.json
                        let pkg_json = serde_json::json!({
                            "name": pkg.name,
                            "version": pkg.version,
                            "dependencies": pkg.dependencies,
                        });

                        fs::write(
                            pkg_dir.join("package.json"),
                            serde_json::to_string_pretty(&pkg_json)?,
                        ).await?;
                        
                        Ok(pkg.name.clone())
                    }
                })
                .buffer_unordered(self.concurrency)
                .collect::<Vec<_>>()
                .await;
                
            // Process results
            for result in results {
                match result {
                    Ok(name) => {
                        debug!("Successfully installed {}", name);
                    },
                    Err(e) => {
                        debug!("Task error: {}", e);
                    }
                }
            }
        }
        
        info!("Installed {} packages in {:?}", total, start.elapsed());
        
        Ok(())
    }

    // Utility method to resolve and install a list of packages with zero-copy optimization
    #[allow(dead_code)]
    pub async fn resolve_and_install(&self, 
        packages: &[(String, String)], 
        install_path: &Path,
        is_dev: bool
    ) -> Result<Vec<Package>> {
        let start = Instant::now();
        
        // Create a root package
        let mut root = Package {
            name: "root".to_string(),
            version: "0.0.0".to_string(),
            dependencies: HashMap::with_capacity(packages.len()),
            dev_dependencies: HashMap::with_capacity(if is_dev { packages.len() } else { 0 }),
        };
        
        // Add packages to the appropriate section
        for (name, version) in packages {
            if is_dev {
                root.dev_dependencies.insert(name.clone(), version.clone());
            } else {
                root.dependencies.insert(name.clone(), version.clone());
            }
        }
        
        // Resolve dependencies
        info!("Resolving dependencies for {} packages", packages.len());
        let tree = self.resolve_dependencies(&root).await?;
        debug!("Resolved {} dependencies in {:?}", tree.dependencies.len(), start.elapsed());
        
        // Install dependencies
        let install_start = Instant::now();
        info!("Installing {} packages", tree.dependencies.len());
        self.install_tree(&tree, install_path).await?;
        debug!("Installation completed in {:?}", install_start.elapsed());
        
        // Return the list of resolved packages
        Ok(tree.dependencies.values().cloned().collect())
    }
}

// Helper methods that could be used by commands
#[allow(dead_code)]
pub async fn read_package_json(path: &Path) -> Result<Package> {
    let content = fs::read_to_string(path).await?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let dependencies = json
        .get("dependencies")
        .and_then(|deps| deps.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let dev_dependencies = json
        .get("devDependencies")
        .and_then(|deps| deps.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    Ok(Package {
        name,
        version,
        dependencies,
        dev_dependencies,
    })
}

#[allow(dead_code)]
pub async fn update_package_json(
    path: &Path,
    dependencies: &HashMap<String, String>,
    dev: bool,
) -> Result<()> {
    let content = fs::read_to_string(path).await?;
    let mut json: serde_json::Value = serde_json::from_str(&content)?;

    let deps_field = if dev {
        "devDependencies"
    } else {
        "dependencies"
    };

    // Create a new object for dependencies if it doesn't exist
    if !json.as_object_mut().unwrap().contains_key(deps_field) {
        json.as_object_mut().unwrap().insert(
            deps_field.to_string(),
            serde_json::Value::Object(serde_json::Map::new()),
        );
    }

    // Get the dependencies object
    let deps_obj = json
        .as_object_mut()
        .unwrap()
        .get_mut(deps_field)
        .and_then(|v| v.as_object_mut())
        .unwrap();

    // Update dependencies
    for (name, version) in dependencies {
        deps_obj.insert(name.clone(), serde_json::Value::String(version.clone()));
    }

    fs::write(path, serde_json::to_string_pretty(&json)?).await?;

    Ok(())
}
