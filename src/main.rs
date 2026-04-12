mod cli;
mod config;
mod repo;
mod platforms;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use repo::RepoManager;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mut config = Config::load();

    match cli.command {
        Commands::Add { repo, path, release_type } => {
            if let Err(e) = RepoManager::add(&mut config, &repo, path, release_type).await {
                eprintln!("Error: {}", e);
            } else {
                config.save().expect("Failed to save config");
                println!("Successfully added {}", repo);
            }
        }
        Commands::Upgrade { repo } => {
            if let Err(e) = RepoManager::upgrade(&mut config, repo).await {
                eprintln!("Error: {}", e);
            } else {
                println!("Upgrade process completed.");
            }
        }
        Commands::Config {
            key,
            value,
            list,
            unset,
        } => {
            config.handle_config_command(key, value, list, unset);
        }
    }
}
