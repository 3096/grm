mod archive;
mod cli;
mod config;
mod platforms;
mod repo;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use repo::RepoManager;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mut config = Config::load();

    match cli.command {
        Commands::Add {
            repo,
            path,
            release_type,
        } => {
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
                config.save().expect("Failed to save config");
                println!("Upgrade process completed.");
            }
        }
        Commands::List {} => {
            if config.repositories.is_empty() {
                println!("No repositories managed by grm.");
            } else {
                println!("{:<20} {:<20} {:<10}", "Repository", "Path", "Version");
                println!("{:-<50}", "");
                for repo in &config.repositories {
                    println!(
                        "{:<20} {:<20} {:<10}",
                        format!("{}/{}", repo.author, repo.repo),
                        repo.path,
                        repo.version
                    );
                }
            }
        }
        Commands::Config {
            key,
            value,
            list,
            open,
            unset,
        } => {
            config.handle_config_command(key, value, list, open, unset);
        }
    }
}
