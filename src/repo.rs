use crate::config::{Config, RepoConfig};
use crate::platforms::ReleasePlatform;
use std::path::Path;

pub struct RepoManager;

impl RepoManager {
    /// Adds a repository and downloads its latest release.
    pub async fn add(config: &mut Config, repo: &str, path: Option<String>, release_type: Option<String>) -> Result<(), String> {
        println!("Adding repository: {}", repo);

        let download_path_str = match path {
            Some(p) => p,
            None => config.default_download_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| "No download path provided and no default path configured. Use 'grm config default_download_path <path>'".to_string())?,
        };
        let download_path = std::path::PathBuf::from(&download_path_str);

        let platform = crate::platforms::github::GithubPlatform::new();
        let version = Self::download_latest(&platform, repo, &download_path, release_type.as_deref()).await?;

        config.repositories.push(crate::config::RepoConfig {
            name: repo.to_string(),
            path: download_path_str,
            version,
        });

        Ok(())
    }

    /// Upgrades a specific repository or all repositories.
    pub async fn upgrade(config: &mut Config, repo: Option<String>) -> Result<(), String> {
        let platform = crate::platforms::github::GithubPlatform::new();

        match repo {
            Some(r) => {
                let repo_config = config
                    .repositories
                    .iter_mut()
                    .find(|rc| rc.name == r)
                    .ok_or_else(|| format!("Repository {} not found in config", r))?;

                Self::upgrade_repo(&platform, repo_config).await?;
            }
            None => {
                println!("Upgrading all repositories...");
                for repo_config in config.repositories.iter_mut() {
                    Self::upgrade_repo(&platform, repo_config).await?;
                }
                println!("All repositories processed.");
            }
        }
        Ok(())
    }

    async fn download_latest(
        platform: &dyn ReleasePlatform,
        repo: &str,
        path: &std::path::Path,
        release_type: Option<&str>,
    ) -> Result<String, String> {
        let release_info = platform
            .get_latest_release(repo, release_type)
            .await
            .map_err(|e| format!("Failed to get latest release for {}: {}", repo, e))?;

        platform
            .download_release(&release_info.download_url, path)
            .await
            .map_err(|e| format!("Failed to download release for {}: {}", repo, e))?;

        Ok(release_info.version)
    }

    async fn upgrade_repo(
        platform: &dyn ReleasePlatform,
        repo_config: &mut crate::config::RepoConfig,
    ) -> Result<(), String> {
        println!("Checking for updates for {}...", repo_config.name);
        let release_info = platform
            .get_latest_release(&repo_config.name, None)
            .await
            .map_err(|e| {
                format!(
                    "Failed to get latest release for {}: {}",
                    repo_config.name, e
                )
            })?;

        if release_info.version != repo_config.version {
            println!(
                "New version found: {} -> {}. Upgrading...",
                repo_config.version, release_info.version
            );

            let download_path = std::path::PathBuf::from(&repo_config.path);
            platform
                .download_release(&release_info.download_url, &download_path)
                .await
                .map_err(|e| {
                    format!("Failed to download update for {}: {}", repo_config.name, e)
                })?;

            repo_config.version = release_info.version;
            println!(
                "Successfully upgraded {} to {}",
                repo_config.name, repo_config.version
            );
        } else {
            println!(
                "{} is already up to date (version {})",
                repo_config.name, repo_config.version
            );
        }
        Ok(())
    }
}
