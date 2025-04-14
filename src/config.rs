use serde::Deserialize;
use std::{collections::HashSet, path::Path};
use tokio::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub storage: StorageConfig,
    pub download_rules: DownloadRules,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum StorageBackend {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "nginx")]
    Nginx,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub local: Option<LocalStorageConfig>,
    pub nginx: Option<NginxStorageConfig>,
}

#[derive(Debug, Deserialize)]
pub struct NginxStorageConfig {
    pub base_url: String,
    pub public_url: String, // 用于对外返回的url
}

#[derive(Debug, Deserialize)]
pub struct LocalStorageConfig {
    pub root_path: String,
}

#[derive(Debug, Deserialize)]
pub struct DownloadRules {
    pub extensions: HashSet<String>,
}

pub async fn load_config(path: &str) -> anyhow::Result<Config> {
    let config_str = fs::read_to_string(path).await?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

pub fn has_matching_extension(path: &str, extensions: &HashSet<String>) -> bool {
    let path = Path::new(path);
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_string();
        log::debug!(
            "Checking extension for {:?} against {:?}, extension: {:?}",
            path,
            extensions,
            ext_str
        );
        let matches = extensions.contains(&ext_str);
        if matches {
            log::debug!("Extension match found for {:?}", path);
        }
        matches
    } else {
        false
    }
}
