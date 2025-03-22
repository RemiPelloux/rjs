use anyhow::Result;
use clap::Subcommand;
use console::style;
use log::info;

pub mod commands;

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new package.json file
    Init(commands::init::InitOptions),

    /// Install packages
    Install(commands::install::InstallOptions),

    /// List installed packages
    List(commands::list::ListOptions),
}

impl Command {
    pub async fn execute(self) -> Result<()> {
        match self {
            Command::Init(opts) => {
                println!("{}", style("RJS - Initialize a new project").bold().green());
                info!("Initializing new project");
                commands::init::execute(opts).await
            },
            Command::Install(opts) => commands::install::execute(opts).await,
            Command::List(opts) => commands::list::execute(opts).await,
        }
    }
}
