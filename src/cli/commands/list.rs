use anyhow::{Context, Result};
use clap::Args;
use console::style;
use log::info;
use std::collections::BTreeMap;

#[derive(Args)]
pub struct ListOptions {
    /// Display only top-level dependencies
    #[arg(short, long)]
    depth: Option<usize>,
    
    /// Show only dev dependencies
    #[arg(long)]
    dev: bool,
    
    /// Show only production dependencies
    #[arg(long)]
    production: bool,
    
    /// Show only outdated packages
    #[arg(long)]
    outdated: bool,
}

pub async fn execute(opts: ListOptions) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");
    
    // Check if package.json exists
    if !package_json_path.exists() {
        info!("No package.json found.");
        return Ok(());
    }
    
    // Read and parse package.json
    let package_json_content = std::fs::read_to_string(&package_json_path)
        .with_context(|| format!("Failed to read {}", package_json_path.display()))?;
    
    let package_json: serde_json::Value = serde_json::from_str(&package_json_content)
        .with_context(|| "Failed to parse package.json")?;
    
    // Get the package name
    let package_name = package_json.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    // Get the dependencies
    let dependencies = if !opts.dev {
        package_json
            .get("dependencies")
            .and_then(|deps| deps.as_object())
            .map(|obj| obj.iter().collect::<BTreeMap<_, _>>())
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    
    // Get the devDependencies
    let dev_dependencies = if !opts.production {
        package_json
            .get("devDependencies")
            .and_then(|deps| deps.as_object())
            .map(|obj| obj.iter().collect::<BTreeMap<_, _>>())
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    
    if dependencies.is_empty() && dev_dependencies.is_empty() {
        info!("No dependencies found.");
        return Ok(());
    }
    
    // Print the package info
    println!("{} {}", style(package_name).bold(), style("dependencies").dim());
    
    // Print the dependencies
    if !dependencies.is_empty() {
        println!("\n{}:", style("dependencies").green().bold());
        for (name, version) in &dependencies {
            println!("  {} {}", name, style(version.as_str().unwrap_or("")).dim());
        }
    }
    
    // Print the devDependencies
    if !dev_dependencies.is_empty() {
        println!("\n{}:", style("devDependencies").magenta().bold());
        for (name, version) in &dev_dependencies {
            println!("  {} {}", name, style(version.as_str().unwrap_or("")).dim());
        }
    }
    
    // Print summary
    let total_deps = dependencies.len() + dev_dependencies.len();
    println!("\n{} {} dependencies, {} dev dependencies",
        style("✓").green(),
        dependencies.len(),
        dev_dependencies.len());
    
    Ok(())
} 