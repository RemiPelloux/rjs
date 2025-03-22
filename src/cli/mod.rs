use anyhow::Result;
use clap::Subcommand;

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
            Command::Init(opts) => commands::init::execute(opts).await,
            Command::Install(opts) => commands::install::execute(opts).await,
            Command::List(opts) => commands::list::execute(opts).await,
        }
    }
}
