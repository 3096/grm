use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait ReleasePlatform {
    async fn get_latest_release(
        &self,
        repo: &str,
        release_type: Option<&str>,
    ) -> Result<ReleaseInfo, String>;
    async fn download_release(&self, url: &str, destination: &Path) -> Result<(), String>;
}

pub struct ReleaseInfo {
    pub version: String,
    pub download_url: String,
    pub filename: String,
}

pub mod github;
