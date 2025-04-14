use std::{sync::Arc, time::SystemTime};

use crate::{config::StorageBackend, BASE_PATH};
use actix_files::NamedFile;
use async_trait::async_trait;

pub mod local;
pub mod nginx;

lazy_static! {
    static ref STORAGE_PROVIDER: Arc<dyn StorageProvider> = {
        let config = crate::CONFIG.get().expect("Config not initialized");
        match config.storage.backend {
            StorageBackend::Nginx => {
                let nginx_config = config
                    .storage
                    .nginx
                    .as_ref()
                    .expect("Nginx storage config not found");
                Arc::new(
                    nginx::NginxStorageProvider::new(
                        nginx_config.base_url.clone(),
                        BASE_PATH.to_string(),
                        nginx_config.public_url.clone(),
                    )
                    .expect("Failed to create Nginx storage provider"),
                )
            }
            StorageBackend::Local => Arc::new(local::LocalStorageProvider::new(
                config.storage.local.as_ref().unwrap().root_path.clone(),
                BASE_PATH.to_string(),
            )),
        }
    };
}

#[async_trait]
pub trait StorageProvider: Sync + Send {
    async fn list_directory(
        &self,
        path_in_provider: &str,
    ) -> anyhow::Result<Option<Vec<StorageEntry>>>;
    /// 根据完整的请求路径，返回在存储提供者中的路径
    fn path_in_provider(&self, full_path: &str) -> Option<String>;
    /// 获取文件的下载URL（适用于特定后缀的文件）
    async fn get_download_url(&self, full_path: &str) -> anyhow::Result<Option<String>>;

    /// 是否是本地存储
    fn is_local(&self) -> bool {
        false
    }

    /// 流式返回文件内容
    #[allow(unused)]
    async fn stream_file(&self, path_in_provider: &str) -> anyhow::Result<Option<NamedFile>> {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct StorageEntry {
    pub name: String,
    pub url: String,
    pub modified: SystemTime,
    pub size: Option<usize>,
}

pub fn select_provider(full_path: &str) -> Option<(Arc<dyn StorageProvider>, String)> {
    let provider = STORAGE_PROVIDER.clone();
    let path_in_provider = provider.path_in_provider(full_path)?;
    Some((provider, path_in_provider))
}
