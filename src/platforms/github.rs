use crate::platforms::{ReleaseInfo, ReleasePlatform};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    browser_download_url: String,
    name: String,
}

pub struct GithubPlatform {
    client: Client,
}

impl GithubPlatform {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ReleasePlatform for GithubPlatform {
    async fn get_latest_release(
        &self,
        repo: &str,
        release_type: Option<&str>,
    ) -> Result<ReleaseInfo, String> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "grm-cli")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if response.status().is_success() {
            let release: GithubRelease = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse release JSON: {}", e))?;

            if let Some(rt) = release_type {
                let asset = release
                    .assets
                    .iter()
                    .find(|a| a.name.contains(rt))
                    .ok_or_else(|| {
                        format!(
                            "No asset matching keyword '{}' found in the latest release",
                            rt
                        )
                    })?;

                return Ok(ReleaseInfo {
                    version: release.tag_name,
                    download_url: asset.browser_download_url.clone(),
                    filename: asset.name.clone(),
                });
            } else {
                if release.assets.len() == 1 {
                    let asset = &release.assets[0];
                    return Ok(ReleaseInfo {
                        version: release.tag_name,
                        download_url: asset.browser_download_url.clone(),
                        filename: asset.name.clone(),
                    });
                } else if release.assets.is_empty() {
                    return Err("No assets found in the latest release".to_string());
                } else {
                    return Err(format!(
                        "Multiple assets found in the latest release ({}). Please specify a release type using --release-type <keyword>.",
                        release.assets.len()
                    ));
                }
            }
        } else {
            Err(format!("GitHub API returned error: {}", response.status()))
        }
    }

    async fn download_release(&self, url: &str, destination: &Path) -> Result<(), String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;

        let content = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read download content: {}", e))?;

        std::fs::write(destination, content)
            .map_err(|e| format!("Failed to write file to disk: {}", e))?;

        Ok(())
    }
}
