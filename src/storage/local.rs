use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;

use super::{StorageEntry, StorageProvider};

pub struct LocalStorageProvider {
    root_path: String,
    req_path_prefix: String,
}

impl LocalStorageProvider {
    pub fn new(root_path: String, req_path_prefix: String) -> Self {
        let abs_root_path = Path::new(&root_path)
            .canonicalize()
            .expect("Failed to canonicalize root path");
        Self {
            root_path: abs_root_path.to_string_lossy().to_string(),
            req_path_prefix,
        }
    }

    fn ent_path_in_provider(&self, path_in_provider: &str, ent: &tokio::fs::DirEntry) -> String {
        format!("{}/{}", path_in_provider, ent.file_name().to_string_lossy())
    }

    async fn process_entry(
        &self,
        path_in_provider: &str,
        ent: &tokio::fs::DirEntry,
    ) -> anyhow::Result<super::StorageEntry> {
        let file_name = ent.file_name().to_string_lossy().to_string();
        let metadata = ent.metadata().await.map_err(|e| {
            anyhow!(
                "Failed to read metadata for file {}: {}",
                self.ent_path_in_provider(path_in_provider, ent),
                e
            )
        })?;
        let modified = metadata.modified().map_err(|e| {
            anyhow!(
                "Failed to get modified time for file {}: {}",
                self.ent_path_in_provider(path_in_provider, ent),
                e
            )
        })?;

        let size = if metadata.is_file() {
            Some(metadata.len() as usize)
        } else {
            None
        };

        let entry = StorageEntry {
            name: file_name,
            url: self.ent_path_in_provider(path_in_provider, ent),
            modified,
            size,
        };

        Ok(entry)
    }
}

#[async_trait]
impl StorageProvider for LocalStorageProvider {
    fn path_in_provider(&self, full_path: &str) -> Option<String> {
        if full_path.starts_with(&self.req_path_prefix) {
            Some(full_path[self.req_path_prefix.len()..].to_string())
        } else {
            None
        }
    }

    async fn list_directory(&self, path_in_provider: &str) -> Option<Vec<super::StorageEntry>> {
        let mut entries = Vec::new();
        let full_path = format!("{}/{}", self.root_path, path_in_provider);
        let full_path = PathBuf::from(&full_path).canonicalize();
        if full_path.is_err() {
            // 路径解析失败
            return None;
        }
        let full_path = full_path.unwrap();
        if !full_path.is_dir() {
            log::debug!("Path {} is not a directory", full_path.display());
            return None;
        }

        if !full_path.exists() {
            log::debug!("Path {} does not exist", full_path.display());
            return None;
        }
        log::debug!(
            "Listing directory {}/{}, full_path={}",
            self.root_path,
            path_in_provider,
            full_path.display()
        );
        let dir_entries = fs::read_dir(full_path).await.inspect_err(|e| {
            log::error!("Failed to read directory {}: {:?}", path_in_provider, e);
        });
        if dir_entries.is_err() {
            return None;
        }

        if let Ok(mut dir_entries) = dir_entries {
            while let Ok(Some(entry)) = dir_entries.next_entry().await {
                let ent = self.process_entry(path_in_provider, &entry).await;
                if ent.is_err() {
                    log::info!(
                        "Failed to process entry: {:?}, err: {:?}",
                        entry.path(),
                        ent
                    );
                    continue;
                }
                let ent = ent.unwrap();
                entries.push(ent);
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));

        log::trace!(
            "Directory {}/{} listing complete: {:?}",
            self.root_path,
            path_in_provider,
            entries
        );
        Some(entries)
    }
}
