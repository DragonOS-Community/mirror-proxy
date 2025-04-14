use std::{path::Path, sync::Arc, time::SystemTime};

use async_trait::async_trait;

use crate::BASE_PATH;

pub mod local;

lazy_static! {
    static ref STORAGE_PROVIDER: Arc<dyn StorageProvider> = Arc::new(
        local::LocalStorageProvider::new("./".into(), BASE_PATH.to_string())
    );
}

#[async_trait]
pub trait StorageProvider: Sync + Send {
    async fn list_directory(&self, path_in_provider: &str) -> Option<Vec<StorageEntry>>;
    /// 根据完整的请求路径，返回在存储提供者中的路径
    fn path_in_provider(&self, full_path: &str) -> Option<String>;
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
