use anyhow::{Context, Result};
use clap::Args;
use dialoguer::{Confirm, Input};
use log::{info, debug};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitOptions {
    /// Skip prompts and use defaults
    #[arg(short, long)]
    yes: bool,
}

#[derive(Serialize, Deserialize)]
struct PackageJson {
    name: String,
    version: String,
    description: String,
    main: String,
    scripts: Scripts,
    author: String,
    license: String,
    dependencies: serde_json::Value,
    devDependencies: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct Scripts {
    test: String,
}

pub async fn execute(opts: InitOptions) -> Result<()> {
    info!("Initializing new package.json");
    
    let cwd = std::env::current_dir()?;
    let package_path = cwd.join("package.json");
    
    if package_path.exists() && !opts.yes {
        let overwrite = Confirm::new()
            .with_prompt("package.json already exists. Overwrite?")
            .default(false)
            .interact()?;
            
        if !overwrite {
            info!("Aborted");
            return Ok(());
        }
    }
    
    let folder_name = cwd
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("my-package");
        
    let package_json = if opts.yes {
        create_default_package_json(folder_name.to_string())
    } else {
        create_interactive_package_json(folder_name.to_string())?
    };
    
    let json_content = serde_json::to_string_pretty(&package_json)?;
    fs::write(&package_path, json_content)
        .with_context(|| format!("Failed to write to {}", package_path.display()))?;
        
    info!("Created package.json");
    
    Ok(())
}

fn create_default_package_json(name: String) -> PackageJson {
    PackageJson {
        name,
        version: "1.0.0".to_string(),
        description: "".to_string(),
        main: "index.js".to_string(),
        scripts: Scripts {
            test: "echo \"Error: no test specified\" && exit 1".to_string(),
        },
        author: "".to_string(),
        license: "ISC".to_string(),
        dependencies: serde_json::json!({}),
        devDependencies: serde_json::json!({}),
    }
}

fn create_interactive_package_json(default_name: String) -> Result<PackageJson> {
    let name: String = Input::new()
        .with_prompt("package name")
        .default(default_name)
        .interact_text()?;
        
    let version: String = Input::new()
        .with_prompt("version")
        .default("1.0.0".to_string())
        .interact_text()?;
        
    let description: String = Input::new()
        .with_prompt("description")
        .allow_empty(true)
        .interact_text()?;
        
    let main: String = Input::new()
        .with_prompt("entry point")
        .default("index.js".to_string())
        .interact_text()?;
        
    let test_cmd: String = Input::new()
        .with_prompt("test command")
        .default("echo \"Error: no test specified\" && exit 1".to_string())
        .interact_text()?;
        
    let author: String = Input::new()
        .with_prompt("author")
        .allow_empty(true)
        .interact_text()?;
        
    let license: String = Input::new()
        .with_prompt("license")
        .default("ISC".to_string())
        .interact_text()?;
        
    Ok(PackageJson {
        name,
        version,
        description,
        main,
        scripts: Scripts { test: test_cmd },
        author,
        license,
        dependencies: serde_json::json!({}),
        devDependencies: serde_json::json!({}),
    })
}
