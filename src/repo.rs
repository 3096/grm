use crate::config::Config;
use crate::platforms::ReleasePlatform;

pub struct RepoManager;

impl RepoManager {
    /// Adds a repository and downloads its latest release.
    pub async fn add(
        config: &mut Config,
        repo: &str,
        path: Option<String>,
        release_type: Option<String>,
    ) -> Result<(), String> {
        println!("Adding repository: {}", repo);

        let download_path_str = match path {
            Some(p) => p,
            None => config.default_download_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| "No download path provided and no default path configured. Use 'grm config default_download_path <path>'".to_string())?,
        };

        let platform = crate::platforms::github::GithubPlatform::new();
        let release_info = platform
            .get_latest_release(repo, release_type.as_deref())
            .await
            .map_err(|e| format!("Failed to get latest release for {}: {}", repo, e))?;

        println!(
            "Downloading latest release {} to {}...",
            release_info.version, download_path_str
        );

        // Use a temporary directory for the download and extraction process
        let temp_dir =
            tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let archive_path = temp_dir.path().join("release_archive");

        if let Err(e) = platform
            .download_release(&release_info.download_url, &archive_path)
            .await
        {
            return Err(format!("Failed to download release: {}", e));
        }

        // Extract the archive
        let extract_dir = temp_dir.path().join("extracted");
        std::fs::create_dir_all(&extract_dir)
            .map_err(|e| format!("Failed to create extract dir: {}", e))?;

        crate::archive::ArchiveManager::extract(&archive_path, &extract_dir).map_err(|e| e)?;

        // Finalize the extraction to the destination
        let final_destination = std::path::PathBuf::from(&download_path_str);
        crate::archive::ArchiveManager::finalize_extraction(&extract_dir, &final_destination, repo)
            .map_err(|e| e)?;

        config.repositories.push(crate::config::RepoConfig {
            name: repo.to_string(),
            path: download_path_str,
            version: release_info.version,
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

            let temp_dir =
                tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
            let archive_path = temp_dir.path().join("release_archive");

            if let Err(e) = platform
                .download_release(&release_info.download_url, &archive_path)
                .await
            {
                return Err(format!("Failed to download release: {}", e));
            }

            let extract_dir = temp_dir.path().join("extracted");
            std::fs::create_dir_all(&extract_dir)
                .map_err(|e| format!("Failed to create extract dir: {}", e))?;

            crate::archive::ArchiveManager::extract(&archive_path, &extract_dir).map_err(|e| e)?;

            let final_destination = std::path::PathBuf::from(&repo_config.path);
            crate::archive::ArchiveManager::finalize_extraction(
                &extract_dir,
                &final_destination,
                &repo_config.name,
            )
            .map_err(|e| e)?;

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
