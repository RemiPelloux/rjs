use anyhow::Result;
use clap::Args;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle, ProgressState};
use log::{info, warn};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time;
use futures::future;
use std::fmt::Write;

use crate::dependency::{self, DependencyResolver};
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
    
    /// Number of concurrent operations (default: number of CPU cores * 4)
    #[arg(short = 'j', long)]
    concurrency: Option<usize>,
    
    /// Batch size for processing packages (10-100, default: 50)
    #[arg(short = 'b', long)]
    batch_size: Option<usize>,
    
    /// Skip progress display for faster non-interactive installs
    #[arg(long)]
    no_progress: bool,
}

pub async fn execute(opts: InstallOptions) -> Result<()> {
    let start_time = Instant::now();
    
    // Check if package.json exists
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");

    if !package_json_path.exists() {
        warn!("No package.json found. Run 'rjs init' first or specify packages to install.");
        println!("No package.json found. Run 'rjs init' first or specify packages to install.");
        if opts.packages.is_empty() {
            return Ok(());
        }
    }

    // Create registry and dependency resolver with concurrency
    let registry = NpmRegistry::new();
    let mut resolver = DependencyResolver::new(registry);
    
    // Set custom concurrency if provided
    if let Some(concurrency) = opts.concurrency {
        info!("Using custom concurrency level: {}", concurrency);
        resolver = resolver.with_concurrency(concurrency);
    }
    
    // Set custom batch size if provided
    if let Some(batch_size) = opts.batch_size {
        info!("Using custom batch size: {}", batch_size);
        resolver = resolver.with_batch_size(batch_size);
    }

    if opts.packages.is_empty() {
        info!("Installing dependencies from package.json");
        println!("{} Installing dependencies from package.json", style("📦").bold().cyan());
        return install_from_package_json(&cwd, &resolver, opts.frozen, opts.no_progress).await;
    }

    // Install specified packages
    info!("Installing specified packages: {:?}", opts.packages);
    println!("{} Installing packages: {}", 
        style("📦").bold().cyan(),
        opts.packages.iter().map(|p| style(p).bold().to_string()).collect::<Vec<_>>().join(", ")
    );

    // Display frozen mode message if enabled
    if opts.frozen {
        println!("  {} Using {} mode - exact versions from lockfile", 
            style("•").yellow(),
            style("frozen").bold()
        );
    }

    // Set up progress bars if enabled
    let progress_enabled = !opts.no_progress && atty::is(atty::Stream::Stdout);
    let multi_progress = MultiProgress::new();
    
    // High-performance progress bar style
    let spinner_style = ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {prefix:.bold.dim}: {msg}")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("█▓▒░  ");
    
    let progress_bars: Vec<_> = if progress_enabled {
        opts.packages
            .iter()
            .map(|pkg| {
                let pb = multi_progress.add(ProgressBar::new(100));
                pb.set_style(spinner_style.clone());
                pb.set_prefix(pkg.clone());
                pb.set_message("Resolving...");
                pb.enable_steady_tick(Duration::from_millis(80));
                pb
            })
            .collect()
    } else {
        vec![]
    };
    
    // Convert packages to name/version pairs with "latest" as default version
    let packages_to_install: Vec<(String, String)> = opts
        .packages
        .iter()
        .map(|pkg| {
            let parts: Vec<&str> = pkg.split('@').collect();
            if parts.len() > 1 && !parts[0].is_empty() {
                // Format is name@version
                (parts[0].to_string(), parts[1..].join("@"))
            } else {
                // Just a name or scoped package without version
                (pkg.clone(), "latest".to_string())
            }
        })
        .collect();
    
    // Create a background task to update progress bars
    let progress_task = if progress_enabled {
        let total_packages = packages_to_install.len();
        let progress_bars_clone = progress_bars.clone();
        
        tokio::spawn(async move {
            for i in 0..total_packages {
                let pb = &progress_bars_clone[i];
                
                // Simulate phases of installation
                for (phase, pct) in &[
                    ("Resolving metadata...", 10),
                    ("Resolving dependencies...", 30),
                    ("Downloading packages...", 60),
                    ("Installing...", 80),
                    ("Finalizing...", 95),
                ] {
                    pb.set_message(*phase);
                    pb.set_position(*pct);
                    time::sleep(Duration::from_millis(300)).await;
                }
            }
        })
    } else {
        tokio::spawn(async {})
    };
    
    // Actually install packages
    let install_result = resolver
        .resolve_and_install(&packages_to_install, &cwd, opts.save_dev, opts.frozen)
        .await;
    
    // Complete progress bars if enabled
    if progress_enabled {
        for pb in &progress_bars {
            pb.finish_with_message(format!("{} Done", style("✓").green()));
            pb.set_position(100);
        }
        
        // Wait for the progress task to complete
        let _ = progress_task.await;
    }
    
    match install_result {
        Ok(installed_packages) => {
            // Update package.json if needed
            if !opts.no_save && package_json_path.exists() {
                // Create a map of installed packages with their versions
                let mut dependencies = std::collections::HashMap::new();
                for package in installed_packages {
                    dependencies.insert(package.name, package.version);
                }
                
                // Update package.json
                dependency::update_package_json(&package_json_path, &dependencies, opts.save_dev).await?;
                info!("Updated package.json");
                println!("{} Updated package.json", style("✓").green());
            }
            
            let elapsed = start_time.elapsed();
            info!("Installed {} packages in {:?}", packages_to_install.len(), elapsed);
            println!(
                "{} Installed {} packages in {:.2}s", 
                style("✅").green(), 
                style(packages_to_install.len()).bold(),
                elapsed.as_secs_f64()
            );
        },
        Err(e) => {
            println!("{} Failed to install packages: {}", style("✗").red(), e);
            return Err(e);
        }
    }
    
    Ok(())
}

async fn install_from_package_json(
    cwd: &Path, 
    resolver: &DependencyResolver, 
    frozen: bool,
    no_progress: bool
) -> Result<()> {
    let start_time = Instant::now();
    let package_json_path = cwd.join("package.json");
    let package = dependency::read_package_json(&package_json_path).await?;
    
    let dependencies = &package.dependencies;
    let dev_dependencies = &package.dev_dependencies;

    let total_deps = dependencies.len() + dev_dependencies.len();

    if total_deps == 0 {
        info!("No dependencies found in package.json");
        println!("{} No dependencies found in package.json", style("ℹ").blue());
        return Ok(());
    }

    info!("Found {} dependencies in package.json", total_deps);
    println!("{} Found {} dependencies in package.json", 
        style("ℹ").blue(),
        style(total_deps).bold()
    );

    // Set up progress if enabled
    let progress_enabled = !no_progress && atty::is(atty::Stream::Stdout);
    let progress_bar = if progress_enabled {
        let pb = ProgressBar::new(total_deps as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg} {eta}"
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "({:.1}s)", state.eta().as_secs_f64()).unwrap())
            .progress_chars("█▓▒░  ")
        );
        pb.set_message("Resolving dependencies...");
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };
    
    // Convert dependencies to the format expected by resolver
    let regular_deps: Vec<(String, String)> = dependencies
        .iter()
        .map(|(name, version)| (name.clone(), version.clone()))
        .collect();
        
    let dev_deps: Vec<(String, String)> = dev_dependencies
        .iter()
        .map(|(name, version)| (name.clone(), version.clone()))
        .collect();
    
    // Show dependency counts
    if !regular_deps.is_empty() {
        println!("  {} {} regular dependencies", 
            style("•").cyan(),
            style(regular_deps.len()).bold()
        );
    }
    
    if !dev_deps.is_empty() {
        println!("  {} {} development dependencies", 
            style("•").magenta(),
            style(dev_deps.len()).bold()
        );
    }
    
    // Display frozen mode message if enabled
    if frozen {
        println!("  {} Using {} mode - exact versions from lockfile", 
            style("•").yellow(),
            style("frozen").bold()
        );
    }
    
    // Update progress message
    if progress_enabled {
        progress_bar.set_message("Installing dependencies...");
    }
    
    // Install both types of dependencies concurrently
    let (regular_result, dev_result) = future::join(
        resolver.resolve_and_install(&regular_deps, cwd, false, frozen),
        resolver.resolve_and_install(&dev_deps, cwd, true, frozen)
    ).await;
    
    // Check results
    match (regular_result, dev_result) {
        (Ok(_), Ok(_)) => {
            // Complete the progress bar
            if progress_enabled {
                progress_bar.finish_with_message("All dependencies installed successfully!");
            }
            
            let elapsed = start_time.elapsed();
            println!("{} All dependencies installed successfully in {:.2}s!", 
                style("✅").green(),
                elapsed.as_secs_f64()
            );
            Ok(())
        },
        (Err(e), _) | (_, Err(e)) => {
            if progress_enabled {
                progress_bar.abandon_with_message(format!("Failed to install: {}", e));
            }
            println!("{} Failed to install dependencies: {}", style("✗").red(), e);
            Err(e)
        }
    }
}
