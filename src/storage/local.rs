use std::path::{Path, PathBuf};

use actix_files::NamedFile;
use anyhow;
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
            anyhow::anyhow!(
                "Failed to read metadata for file {}: {}",
                self.ent_path_in_provider(path_in_provider, ent),
                e
            )
        })?;
        let modified = metadata.modified().map_err(|e| {
            anyhow::anyhow!(
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

impl LocalStorageProvider {
    fn abs_path(&self, path_in_provider: &str) -> anyhow::Result<PathBuf> {
        let full_path = format!("{}/{}", self.root_path, path_in_provider);
        let full_path = PathBuf::from(&full_path).canonicalize().map_err(|e| {
            anyhow!(
                "Failed to canonicalize path {}, err: {}",
                path_in_provider,
                e
            )
        })?;
        Ok(full_path)
    }
}

#[async_trait]
impl StorageProvider for LocalStorageProvider {
    fn is_local(&self) -> bool {
        true
    }

    async fn stream_file(&self, path_in_provider: &str) -> anyhow::Result<Option<NamedFile>> {
        let file_path = self.abs_path(path_in_provider)?;
        match NamedFile::open_async(file_path).await {
            Ok(file) => Ok(Some(file)),
            Err(e) => {
                log::debug!("Failed to open file {}: {}", path_in_provider, e);
                Ok(None)
            }
        }
    }

    fn path_in_provider(&self, full_path: &str) -> Option<String> {
        if full_path.starts_with(&self.req_path_prefix) {
            Some(full_path[self.req_path_prefix.len()..].to_string())
        } else {
            None
        }
    }

    async fn get_download_url(&self, _full_path: &str) -> anyhow::Result<Option<String>> {
        // should not impl for local storage
        Ok(None)
    }

    async fn list_directory(
        &self,
        path_in_provider: &str,
    ) -> anyhow::Result<Option<Vec<super::StorageEntry>>> {
        let mut entries = Vec::new();
        let full_path = self.abs_path(path_in_provider).map_err(|e| {
            e.context(format!(
                "list_directory: Failed to resolve path '{}'",
                path_in_provider
            ))
        })?;
        if !full_path.is_dir() {
            log::debug!("Path {} is not a directory", full_path.display());
            return Ok(None);
        }

        if !full_path.exists() {
            log::debug!("Path {} does not exist", full_path.display());
            return Ok(None);
        }
        log::debug!(
            "Listing directory {}/{}, full_path={}",
            self.root_path,
            path_in_provider,
            full_path.display()
        );
        let mut dir_entries = fs::read_dir(full_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read directory {}: {}", path_in_provider, e))?;

        while let Some(entry) = dir_entries
            .next_entry()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read directory entry: {}", e))?
        {
            match self.process_entry(path_in_provider, &entry).await {
                Ok(ent) => entries.push(ent),
                Err(e) => {
                    log::warn!("Failed to process entry {:?}: {}", entry.path(), e);
                }
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));

        log::trace!(
            "Directory {}/{} listing complete: {:?}",
            self.root_path,
            path_in_provider,
            entries
        );
        Ok(Some(entries))
    }
}
