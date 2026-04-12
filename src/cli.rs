use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "grm")]
#[command(about = "GitHub Release Manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a repository and download its latest release
    Add {
        /// The GitHub repository (e.g., "owner/repo")
        repo: String,
        /// The location to download the release to
        #[arg(short, long)]
        path: Option<String>,
        /// Keyword to filter the release asset (e.g., "windows", "linux")
        #[arg(short, long)]
        release_type: Option<String>,
    },
    /// Upgrade a repository's release, or all if none specified
    Upgrade {
        /// The GitHub repository to upgrade (optional)
        repo: Option<String>,
    },
    /// Manage grm configuration
    Config {
        /// The config key to get or set (e.g., "default_download_path")
        key: Option<String>,
        /// The value to set for the config key
        value: Option<String>,
        /// List all configuration settings
        #[arg(short, long)]
        list: bool,
        /// Unset a configuration setting
        #[arg(short, long)]
        unset: bool,
    },
}
