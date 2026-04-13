use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub repositories: Vec<RepoConfig>,
    pub default_download_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepoConfig {
    pub author: String,
    pub repo: String,
    pub path: String,
    pub version: String,
}

impl Config {
    pub fn load() -> Self {
        Self::load_from_path(&Self::get_config_path())
    }

    pub fn load_from_path(path: &std::path::Path) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !Self::check_write_permission(path) {
                    panic!(
                        "Config file not found and no writable directory found in the path hierarchy for {:?}. You may need to check your permissions.",
                        path
                    );
                }
                return Config::default();
            }
            Err(e) => panic!("Failed to read config file at {:?}: {}", path, e),
        };

        toml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse config file at {:?}: {}", path, e))
    }

    fn check_write_permission(path: &std::path::Path) -> bool {
        let mut current = path.parent();
        while let Some(p) = current {
            if p.exists() {
                let test_file = p.join(".grm_write_test");
                if std::fs::File::create(&test_file).is_ok() {
                    let _ = std::fs::remove_file(&test_file);
                    return true;
                }
            }
            current = p.parent();
        }
        false
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        self.save_to_path(&Self::get_config_path())
    }

    pub fn save_to_path(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }

    fn get_config_path() -> PathBuf {
        ProjectDirs::from("com", "the3096", "grm")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    pub fn set_value(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "default_download_path" => {
                self.default_download_path = Some(PathBuf::from(value));
                Ok(())
            }
            _ => Err(format!("Unknown config key: {}", key)),
        }
    }

    pub fn get_value(&self, key: &str) -> Option<String> {
        match key {
            "default_download_path" => self
                .default_download_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            _ => None,
        }
    }

    pub fn unset_value(&mut self, key: &str) -> Result<(), String> {
        match key {
            "default_download_path" => {
                self.default_download_path = None;
                Ok(())
            }
            _ => Err(format!("Unknown config key: {}", key)),
        }
    }

    pub fn list_values(&self) -> Vec<(String, String)> {
        let mut values = Vec::new();
        if let Some(path) = &self.default_download_path {
            values.push((
                "default_download_path".to_string(),
                path.to_string_lossy().into_owned(),
            ));
        }
        values
    }

    pub fn handle_config_command(
        &mut self,
        key: Option<String>,
        value: Option<String>,
        list: bool,
        unset: bool,
    ) {
        if list {
            let values = self.list_values();
            for (k, v) in values {
                println!("{} = {}", k, v);
            }
        } else if unset {
            if let Some(k) = key {
                match self.unset_value(&k) {
                    Ok(_) => {
                        self.save().expect("Failed to save config");
                        println!("Unset {} successfully", k);
                    }
                    Err(e) => println!("Error: {}", e),
                }
            } else {
                println!("Error: Key required for unset");
            }
        } else if let Some(k) = key {
            if let Some(v) = value {
                match self.set_value(&k, &v) {
                    Ok(_) => {
                        self.save().expect("Failed to save config");
                        println!("Set {} = {}", k, v);
                    }
                    Err(e) => println!("Error: {}", e),
                }
            } else {
                let val = self.get_value(&k);
                match val {
                    Some(v) => println!("{}", v),
                    None => println!("Key {} not set", k),
                }
            }
        } else {
            println!("Error: Config key required");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_save_load() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.toml");

        let config = Config {
            repositories: vec![RepoConfig {
                author: "owner".to_string(),
                repo: "repo".to_string(),
                path: "/tmp/repo".to_string(),
                version: "v1.0.0".to_string(),
            }],
            default_download_path: None,
        };

        config
            .save_to_path(&file_path)
            .expect("Failed to save config");
        let loaded_config = Config::load_from_path(&file_path);

        assert_eq!(config.repositories.len(), 1);
        assert_eq!(loaded_config.repositories.len(), 1);
        assert_eq!(loaded_config.repositories[0].repo, "repo");
    }

    #[test]
    fn test_config_default() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("non_existent.toml");
        let config = Config::load_from_path(&file_path);
        assert_eq!(config.repositories.len(), 0);
    }

    #[test]
    fn test_config_value_management() {
        let mut config = Config::default();

        // Test set
        assert!(
            config
                .set_value("default_download_path", "/tmp/downloads")
                .is_ok()
        );
        assert_eq!(
            config.get_value("default_download_path"),
            Some("/tmp/downloads".to_string())
        );

        // Test unknown key
        assert!(config.set_value("unknown", "val").is_err());
        assert_eq!(config.get_value("unknown"), None);

        // Test unset
        assert!(config.unset_value("default_download_path").is_ok());
        assert_eq!(config.get_value("default_download_path"), None);

        // Test unset unknown
        assert!(config.unset_value("unknown").is_err());
    }

    #[test]
    fn test_config_list_values() {
        let mut config = Config::default();
        config
            .set_value("default_download_path", "/tmp/downloads")
            .unwrap();

        let values = config.list_values();
        assert_eq!(values.len(), 1);
        assert_eq!(
            values[0],
            (
                "default_download_path".to_string(),
                "/tmp/downloads".to_string()
            )
        );
    }

    #[test]
    fn test_handle_config_command_updates_state() {
        let mut config = Config::default();

        // Test set via handle
        config.handle_config_command(
            Some("default_download_path".to_string()),
            Some("/tmp/downloads".to_string()),
            false,
            false,
        );
        assert_eq!(
            config.get_value("default_download_path"),
            Some("/tmp/downloads".to_string())
        );

        // Test unset via handle
        config.handle_config_command(Some("default_download_path".to_string()), None, false, true);
        assert_eq!(config.get_value("default_download_path"), None);
    }
}
