use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use log::info;

mod cli;
mod dependency;
mod registry;
mod utils;

#[derive(Parser)]
#[command(
    name = "rjs",
    about = "A modern, fast, and secure npm alternative",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: cli::Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Execute the command
    info!("RJS - Rust JavaScript Package Manager");
    cli.command.execute().await?;
    
    Ok(())
}
