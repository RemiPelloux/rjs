use anyhow::{Context, Result};
use clap::Args;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, warn};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;

use crate::dependency::DependencyResolver;
use crate::registry::NpmRegistry;

#[derive(Args)]
pub struct InstallOptions {
    /// Packages to install
    packages: Vec<String>,

    /// Save as a development dependency
    #[arg(short = 'D', long)]
    save_dev: bool,

    /// Install dependencies from lockfile without updating
    #[arg(short, long)]
    frozen: bool,

    /// Don't save to dependencies
    #[arg(long)]
    no_save: bool,
}

pub async fn execute(opts: InstallOptions) -> Result<()> {
    // Check if package.json exists
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");

    if !package_json_path.exists() {
        warn!("No package.json found. Run 'rjs init' first or specify packages to install.");
        if opts.packages.is_empty() {
            return Ok(());
        }
    }

    if opts.packages.is_empty() {
        info!("Installing dependencies from package.json");
        return install_from_package_json(&cwd, opts.frozen).await;
    }

    // Install specified packages
    info!("Installing specified packages: {:?}", opts.packages);

    let registry = NpmRegistry::new();
    let _resolver = DependencyResolver::new(registry);

    // Set up progress bars
    let multi_progress = MultiProgress::new();
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap();

    let progress_bars: Vec<_> = opts
        .packages
        .iter()
        .map(|pkg| {
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_prefix(format!("[{}]", pkg));
            pb.set_message("Resolving...");
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        })
        .collect();

    // Simulating installation for now
    for (i, _package) in opts.packages.iter().enumerate() {
        let pb = &progress_bars[i];

        // Simulate dependency resolution
        pb.set_message("Resolving dependencies...");
        time::sleep(Duration::from_millis(800)).await;

        // Simulate download
        pb.set_message("Downloading...");
        time::sleep(Duration::from_millis(1200)).await;

        // Simulate extraction
        pb.set_message("Extracting...");
        time::sleep(Duration::from_millis(500)).await;

        // Finish
        pb.finish_with_message(format!("{} Installed", style("✓").green()));
    }

    // Update package.json if needed
    if !opts.no_save {
        // Logic to update package.json
        info!("Updated package.json");
    }

    info!("Installed {} packages", opts.packages.len());
    Ok(())
}

async fn install_from_package_json(cwd: &PathBuf, _frozen: bool) -> Result<()> {
    let package_json_path = cwd.join("package.json");
    let package_json_content = std::fs::read_to_string(&package_json_path)
        .with_context(|| format!("Failed to read {}", package_json_path.display()))?;

    let package_json: serde_json::Value = serde_json::from_str(&package_json_content)
        .with_context(|| "Failed to parse package.json")?;

    let dependencies = package_json
        .get("dependencies")
        .and_then(|deps| deps.as_object())
        .map(|obj| obj.iter().collect::<BTreeMap<_, _>>())
        .unwrap_or_default();

    let dev_dependencies = package_json
        .get("devDependencies")
        .and_then(|deps| deps.as_object())
        .map(|obj| obj.iter().collect::<BTreeMap<_, _>>())
        .unwrap_or_default();

    let total_deps = dependencies.len() + dev_dependencies.len();

    if total_deps == 0 {
        info!("No dependencies found in package.json");
        return Ok(());
    }

    info!("Found {} dependencies in package.json", total_deps);

    // Set up progress
    let progress_bar = ProgressBar::new(total_deps as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{prefix:.bold.dim} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );
    progress_bar.set_prefix("[installing]");

    // Simulate installation
    for (name, _) in dependencies.iter() {
        progress_bar.set_message(format!("Installing {}...", name));
        time::sleep(Duration::from_millis(300)).await;
        progress_bar.inc(1);
    }

    for (name, _) in dev_dependencies.iter() {
        progress_bar.set_message(format!("Installing {}...", name));
        time::sleep(Duration::from_millis(300)).await;
        progress_bar.inc(1);
    }

    progress_bar.finish_with_message("All dependencies installed successfully!");

    Ok(())
}
