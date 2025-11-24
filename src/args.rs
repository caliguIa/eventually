use clap::{Parser, Subcommand};
use std::io::Result;

use crate::launchd::{Service, ID};

#[derive(Parser)]
#[command(name = "eventually")]
#[command(about = "macOS menu bar calendar app", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Manage launchd service
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
}

#[derive(Subcommand)]
pub enum ServiceAction {
    /// Install the launchd service
    Install,
    /// Uninstall the launchd service
    Uninstall,
    /// Start the service
    Start,
    /// Stop the service
    Stop,
    /// Restart the service
    Restart,
}

impl ServiceAction {
    pub fn execute(self) -> Result<()> {
        let service = Service::try_new(ID)?;
        
        match self {
            Self::Install => service.install(),
            Self::Uninstall => service.uninstall(),
            Self::Start => service.start(),
            Self::Stop => service.stop(),
            Self::Restart => service.restart(),
        }
    }
}

impl Cli {
    pub fn parse_and_execute() -> Option<Result<()>> {
        let cli = Self::parse();
        
        match cli.command {
            Some(Command::Service { action }) => Some(action.execute()),
            None => None,
        }
    }
}
