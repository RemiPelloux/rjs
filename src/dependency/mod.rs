use anyhow::{Context, Result};
use log::debug;
use semver::VersionReq;
use std::collections::{HashMap, HashSet};
use std::path::Path;

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

#[allow(dead_code)]
pub struct DependencyResolver {
    registry: NpmRegistry,
    visited: HashSet<String>,
}

impl DependencyResolver {
    #[allow(dead_code)]
    pub fn new(registry: NpmRegistry) -> Self {
        Self {
            registry,
            visited: HashSet::new(),
        }
    }

    #[allow(dead_code)]
    pub async fn resolve_package(&mut self, name: &str, version_req: &str) -> Result<Package> {
        let key = format!("{}@{}", name, version_req);

        if self.visited.contains(&key) {
            debug!("Already visited {}", key);
            // Return a dummy package to avoid circular dependencies for now
            return Ok(Package {
                name: name.to_string(),
                version: "0.0.0".to_string(),
                dependencies: HashMap::new(),
                dev_dependencies: HashMap::new(),
            });
        }

        self.visited.insert(key.clone());

        // Fetch package info from registry
        let package_info = self.registry.get_package_info(name).await?;

        // Find the best matching version
        let version_req = VersionReq::parse(version_req).unwrap_or(VersionReq::STAR);

        let best_version = package_info
            .versions
            .keys()
            .filter_map(|v| match semver::Version::parse(v) {
                Ok(parsed) => Some((v, parsed)),
                Err(_) => None,
            })
            .filter(|(_, parsed)| version_req.matches(parsed))
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(v, _)| v.clone())
            .with_context(|| format!("No matching version found for {}@{}", name, version_req))?;

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

        Ok(package)
    }

    #[allow(dead_code)]
    pub async fn resolve_dependencies(&mut self, root_pkg: &Package) -> Result<DependencyTree> {
        let mut dependencies = HashMap::new();

        // Resolve all dependencies recursively
        for (dep_name, dep_version) in &root_pkg.dependencies {
            let dep_pkg = self.resolve_package(dep_name, dep_version).await?;
            dependencies.insert(format!("{}@{}", dep_name, dep_version), dep_pkg);
        }

        Ok(DependencyTree {
            root: root_pkg.clone(),
            dependencies,
        })
    }

    #[allow(dead_code)]
    pub async fn install_tree(&self, tree: &DependencyTree, install_path: &Path) -> Result<()> {
        // Create node_modules directory if it doesn't exist
        let node_modules = install_path.join("node_modules");
        if !node_modules.exists() {
            std::fs::create_dir_all(&node_modules)?;
        }

        // Install all dependencies
        for pkg in tree.dependencies.values() {
            let pkg_dir = node_modules.join(&pkg.name);
            if !pkg_dir.exists() {
                std::fs::create_dir_all(&pkg_dir)?;
            }

            // In a real implementation, we would download and extract the package here
            debug!(
                "Installing {} {} to {}",
                pkg.name,
                pkg.version,
                pkg_dir.display()
            );

            // Create a minimal package.json
            let pkg_json = serde_json::json!({
                "name": pkg.name,
                "version": pkg.version,
                "dependencies": pkg.dependencies,
            });

            std::fs::write(
                pkg_dir.join("package.json"),
                serde_json::to_string_pretty(&pkg_json)?,
            )?;
        }

        Ok(())
    }
}

// Helper methods that could be used by commands
#[allow(dead_code)]
pub fn read_package_json(path: &Path) -> Result<Package> {
    let content = std::fs::read_to_string(path)?;
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
pub fn update_package_json(
    path: &Path,
    dependencies: &HashMap<String, String>,
    dev: bool,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
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

    std::fs::write(path, serde_json::to_string_pretty(&json)?)?;

    Ok(())
}
