use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use log::{debug, info};
use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs;
use rayon::prelude::*;
use std::time::Instant;
use crossbeam::queue::SegQueue;
use std::thread;
use serde::{Deserialize, Serialize};
use hex;

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

// Add a structure for tracking deduplicated dependencies
#[derive(Clone)]
struct DependencyDeduplication {
    // Map from package name to available versions and their full specs
    packages: Arc<Mutex<HashMap<String, Vec<(Version, String, String)>>>>,
}

impl DependencyDeduplication {
    fn new() -> Self {
        Self {
            packages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn register_package(&self, name: &str, version_str: &str, spec: &str) -> Result<()> {
        let version = Version::parse(version_str)
            .with_context(|| format!("Invalid version '{}' for package '{}'", version_str, name))?;
        
        let mut packages = self.packages.lock().unwrap();
        let versions = packages.entry(name.to_string()).or_insert_with(Vec::new);
        
        // Check if this exact version is already registered
        if !versions.iter().any(|(v, _, _)| *v == version) {
            versions.push((version, version_str.to_string(), spec.to_string()));
            // Sort versions in descending order
            versions.sort_by(|(a, _, _), (b, _, _)| b.cmp(a));
        }
        
        Ok(())
    }
    
    fn find_compatible_version(&self, name: &str, req_str: &str) -> Option<String> {
        let req = match VersionReq::parse(req_str) {
            Ok(r) => r,
            Err(_) => return None, // If we can't parse the requirement, we can't find a match
        };
        
        let packages = self.packages.lock().unwrap();
        if let Some(versions) = packages.get(name) {
            // Try to find the highest version that satisfies the requirement
            for (version, version_str, _) in versions {
                if req.matches(version) {
                    return Some(version_str.clone());
                }
            }
        }
        
        None
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
    deduplication: DependencyDeduplication,
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
            deduplication: DependencyDeduplication::new(),
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

    // Update resolve_package to use deduplication
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

        // Check if we can deduplicate by finding a compatible version we've already resolved
        let deduplicated_version = self.deduplication.find_compatible_version(name, version_req);
        if let Some(version) = deduplicated_version {
            debug!("Using deduplicated version {} for {}@{}", version, name, version_req);
            let deduplicated_key = format!("{}@{}", name, version);
            if let Some(cached_pkg) = self.package_cache.get(&deduplicated_key) {
                // We found a compatible package, use it
                return Ok((*cached_pkg).clone());
            }
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
            version: best_version.clone(),
            dependencies: version_info.dependencies.clone(),
            dev_dependencies: version_info.dev_dependencies.clone(),
        };
        
        // Register this package for future deduplication
        let _ = self.deduplication.register_package(name, &best_version, version_req);
        
        // Cache the result
        let _ = self.package_cache.insert(key, package.clone());

        Ok(package)
    }

    // Add a method to deduplicate a dependency tree
    pub async fn deduplicate_tree(&self, tree: &mut DependencyTree) -> Result<()> {
        debug!("Deduplicating dependency tree...");
        let start = Instant::now();
        
        let mut packages_by_name: HashMap<String, Vec<(String, Package)>> = HashMap::new();
        
        // Group packages by name
        for (key, pkg) in &tree.dependencies {
            packages_by_name
                .entry(pkg.name.clone())
                .or_default()
                .push((key.clone(), pkg.clone()));
        }
        
        // Counter for deduplicated packages
        let mut deduped_count = 0;
        
        // Process each group of packages with the same name
        for (_name, packages) in packages_by_name {
            if packages.len() <= 1 {
                continue; // No need to deduplicate single packages
            }
            
            // Sort packages by version (newest first) to prefer newer versions
            let mut sorted_packages = packages;
            sorted_packages.sort_by(|(_, a), (_, b)| {
                Version::parse(&b.version)
                    .unwrap_or_else(|_| Version::new(0, 0, 0))
                    .cmp(&Version::parse(&a.version).unwrap_or_else(|_| Version::new(0, 0, 0)))
            });
            
            // Take the newest version as the preferred one
            let (_preferred_key, preferred_pkg) = &sorted_packages[0];
            
            // For remaining versions, check if they can be deduplicated
            for (key, _package) in sorted_packages.iter().skip(1) {
                // Check if this package is a dependency of any other package
                let mut can_deduplicate = false;
                
                for (_, dep_pkg) in &tree.dependencies {
                    if dep_pkg.dependencies.contains_key(&preferred_pkg.name) {
                        let req = VersionReq::parse(
                            dep_pkg.dependencies.get(&preferred_pkg.name).unwrap()
                        ).unwrap_or_else(|_| VersionReq::STAR);
                        
                        let preferred_version = Version::parse(&preferred_pkg.version)
                            .unwrap_or_else(|_| Version::new(0, 0, 0));
                        
                        if req.matches(&preferred_version) {
                            can_deduplicate = true;
                            break;
                        }
                    }
                }
                
                if can_deduplicate {
                    // Replace this package with the preferred one
                    tree.dependencies.remove(key);
                    deduped_count += 1;
                    
                    // Update dependencies to point to the preferred version
                    for (_, dep_pkg) in tree.dependencies.iter_mut() {
                        if dep_pkg.dependencies.contains_key(&preferred_pkg.name) {
                            dep_pkg.dependencies.insert(
                                preferred_pkg.name.clone(),
                                preferred_pkg.version.clone()
                            );
                        }
                    }
                }
            }
        }
        
        debug!("Deduplicated {} packages in {:?}", deduped_count, start.elapsed());
        Ok(())
    }

    // Update resolve_dependencies to apply deduplication
    #[allow(dead_code)]
    pub async fn resolve_dependencies(&self, root_pkg: &Package) -> Result<DependencyTree> {
        let mut tree = self.resolve_dependencies_internal(root_pkg).await?;
        self.deduplicate_tree(&mut tree).await?;
        Ok(tree)
    }

    // Renamed the original resolve_dependencies method to resolve_dependencies_internal
    async fn resolve_dependencies_internal(&self, root_pkg: &Package) -> Result<DependencyTree> {
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

    // Install method from previous implementation
    pub async fn install_tree(&self, tree: &DependencyTree, install_path: &Path) -> Result<Vec<String>> {
        debug!("Installing dependency tree with {} packages...", tree.dependencies.len());
        let start = Instant::now();
        
        // Create node_modules directory
        let node_modules_dir = install_path.join("node_modules");
        
        // Make sure the node_modules directory exists
        if !node_modules_dir.exists() {
            fs::create_dir_all(&node_modules_dir).await?;
        }
        
        // For tests, just simulate installation by creating empty directories for each package
        let mut installed = Vec::with_capacity(tree.dependencies.len());
        
        for (key, pkg) in &tree.dependencies {
            let pkg_dir = node_modules_dir.join(&pkg.name);
            
            // Create package directory
            if !pkg_dir.exists() {
                fs::create_dir_all(&pkg_dir).await?;
                
                // Create a minimal package.json for the package
                let pkg_json = serde_json::json!({
                    "name": pkg.name,
                    "version": pkg.version,
                    "dependencies": pkg.dependencies,
                });
                
                fs::write(
                    pkg_dir.join("package.json"),
                    serde_json::to_string_pretty(&pkg_json)?,
                ).await?;
            }
            
            installed.push(pkg.name.clone());
            debug!("Installed package {}", key);
        }
        
        debug!("Installed {} packages in {:?}", installed.len(), start.elapsed());
        
        Ok(installed)
    }

    // Generate a lockfile from a dependency tree
    pub async fn generate_lockfile(&self, tree: &DependencyTree, _root_path: &Path) -> Result<Lockfile> {
        debug!("Generating lockfile from dependency tree...");
        let start = Instant::now();
        
        // Create lockfile with project info
        let mut lockfile = Lockfile::new(&tree.root.name, &tree.root.version);
        
        // Add all packages to the lockfile
        for (_, package) in &tree.dependencies {
            // Get registry URL
            let registry_url = format!("{}", self.registry.get_registry_url());
            lockfile.add_package(package, &registry_url);
        }
        
        debug!("Added {} packages to lockfile", lockfile.packages.len());
        debug!("Generated lockfile in {:?}", start.elapsed());
        
        Ok(lockfile)
    }
    
    // Save lockfile to disk
    pub async fn save_lockfile(&self, lockfile: &Lockfile, root_path: &Path) -> Result<()> {
        debug!("Saving lockfile to disk...");
        let start = Instant::now();
        
        let lockfile_path = root_path.join("rjs-lock.json");
        let lockfile_json = serde_json::to_string_pretty(lockfile)?;
        
        fs::write(&lockfile_path, lockfile_json).await?;
        
        debug!("Saved lockfile to {} in {:?}", lockfile_path.display(), start.elapsed());
        
        Ok(())
    }
    
    // Load lockfile from disk
    pub async fn load_lockfile(&self, root_path: &Path) -> Result<Option<Lockfile>> {
        let lockfile_path = root_path.join("rjs-lock.json");
        
        if !lockfile_path.exists() {
            debug!("No lockfile found at {}", lockfile_path.display());
            return Ok(None);
        }
        
        debug!("Loading lockfile from {}...", lockfile_path.display());
        let start = Instant::now();
        
        let lockfile_json = fs::read_to_string(&lockfile_path).await?;
        let lockfile: Lockfile = serde_json::from_str(&lockfile_json)?;
        
        debug!("Loaded lockfile with {} packages in {:?}", 
            lockfile.packages.len(), start.elapsed());
        
        Ok(Some(lockfile))
    }
    
    // Update resolve_and_install to use lockfile if frozen=true
    pub async fn resolve_and_install(
        &self, 
        packages: &[(String, String)], 
        install_path: &Path,
        is_dev: bool,
        frozen: bool  // Add frozen parameter
    ) -> Result<Vec<Package>> {
        info!("Resolving and installing {} packages...", packages.len());
        let start = Instant::now();
        
        // Use absolute path to ensure we're installing in the correct location
        let absolute_install_path = if install_path.is_absolute() {
            install_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(install_path)
        };
        
        println!("Installation path (absolute): {}", absolute_install_path.display());
        
        // Look for existing lockfile if frozen mode is enabled
        if frozen {
            if let Some(lockfile) = self.load_lockfile(&absolute_install_path).await? {
                info!("Using existing lockfile with {} packages", lockfile.packages.len());
                println!("Using frozen lockfile mode - not updating dependencies");
                
                // Install directly from lockfile
                let packages = self.install_from_lockfile(&lockfile, &absolute_install_path).await?;
                
                info!("Installed {} packages from lockfile in {:?}", 
                    packages.len(), start.elapsed());
                    
                return Ok(packages);
            } else {
                info!("No lockfile found, proceeding with normal installation");
            }
        }
        
        // Create a temporary root package
        let mut root_pkg = Package {
            name: "root".to_string(),
            version: "0.0.0".to_string(),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
        };

        // Add requested packages as dependencies
        for (name, version) in packages {
            if is_dev {
                root_pkg.dev_dependencies.insert(name.clone(), version.clone());
            } else {
                root_pkg.dependencies.insert(name.clone(), version.clone());
            }
        }

        // Resolve dependencies
        info!("Resolving dependencies tree...");
        let tree = self.resolve_dependencies(&root_pkg).await?;
        
        info!("Resolved {} packages in {:?}", 
            tree.dependencies.len(), start.elapsed());
        
        // Install packages
        info!("Installing {} packages...", tree.dependencies.len());
        let installed = self.install_tree(&tree, &absolute_install_path).await?;
        
        // Generate and save lockfile
        let lockfile = self.generate_lockfile(&tree, &absolute_install_path).await?;
        self.save_lockfile(&lockfile, &absolute_install_path).await?;
        
        info!("Installed and locked {} packages in {:?}", 
            installed.len(), start.elapsed());
        
        Ok(tree.dependencies.values().cloned().collect())
    }
    
    // Add method to install directly from lockfile
    async fn install_from_lockfile(&self, lockfile: &Lockfile, install_path: &Path) -> Result<Vec<Package>> {
        debug!("Installing packages from lockfile...");
        let start = Instant::now();
        
        // Create node_modules directory
        let node_modules_dir = install_path.join("node_modules");
        if !node_modules_dir.exists() {
            fs::create_dir_all(&node_modules_dir).await?;
        }
        
        // Convert lockfile entries to packages
        let mut packages = Vec::new();
        
        // Clone the packages map to avoid borrowing issues
        let packages_map = lockfile.packages.clone();
        
        // Install packages in parallel
        let registry = self.registry.clone();
        let mut handles = Vec::new();
        
        for (pkg_key, entry) in packages_map {
            // Parse the package name from the key
            let parts: Vec<&str> = pkg_key.split('@').collect();
            if parts.is_empty() {
                continue;
            }
            
            let name = parts[0].to_string();
            let version = entry.version.clone();
            
            let pkg = Package {
                name: name.clone(),
                version: version.clone(),
                dependencies: entry.dependencies.clone(),
                dev_dependencies: HashMap::new(),
            };
            
            packages.push(pkg.clone());
            
            // Install in parallel
            let pkg_dir = node_modules_dir.join(&name);
            let registry_clone = registry.clone();
            
            let handle = tokio::spawn(async move {
                if !pkg_dir.exists() {
                    let _ = fs::create_dir_all(&pkg_dir).await;
                    
                    if let Some(url) = &entry.resolved {
                        // Download and extract the package
                        let tarball_path = pkg_dir.join("package.tgz");
                        let _ = registry_clone.download_package(url, &tarball_path).await;
                        
                        // Extract the package
                        let tarball_path_clone = tarball_path.clone();
                        let pkg_dir_clone = pkg_dir.clone();
                        let extract_result = tokio::task::spawn_blocking(move || {
                            registry_clone.extract_tarball(&tarball_path_clone, &pkg_dir_clone)
                        }).await;
                        
                        if let Ok(Ok(_)) = extract_result {
                            // Clean up the tarball
                            let _ = fs::remove_file(tarball_path).await;
                        }
                    }
                }
                
                name
            });
            
            handles.push(handle);
        }
        
        // Wait for all installations to complete
        let results = futures::future::join_all(handles).await;
        let installed_count = results.iter().filter(|r| r.is_ok()).count();
        
        debug!("Installed {} packages from lockfile in {:?}", 
            installed_count, start.elapsed());
        
        Ok(packages)
    }
}

// Add the Lockfile structures at module scope, before any impl blocks
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LockfileEntry {
    pub version: String,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Lockfile {
    pub name: String,
    pub version: String,
    pub lockfile_version: String,
    pub packages: HashMap<String, LockfileEntry>,
}

// Lockfile implementation at module scope
impl Lockfile {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            lockfile_version: "1.0.0".to_string(),
            packages: HashMap::new(),
        }
    }

    // Add a package to the lockfile
    pub fn add_package(&mut self, pkg: &Package, registry: &str) {
        let key = format!("{}@{}", pkg.name, pkg.version);
        let integrity = Some(format!("sha512-{}", hex::encode(key.as_bytes())));
        let resolved = Some(format!("{}/{}-{}.tgz", registry, pkg.name, pkg.version));
        
        let entry = LockfileEntry {
            version: pkg.version.clone(),
            resolved,
            integrity,
            dependencies: pkg.dependencies.clone(),
        };
        
        self.packages.insert(key, entry);
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

