use std::time::SystemTime;

use async_trait::async_trait;
use reqwest::Url;
use scraper::{Html, Selector};
use url::Url as UrlParser;

use super::{StorageEntry, StorageProvider};

pub struct NginxStorageProvider {
    base_url: String,
    req_path_prefix: String,
    public_url: String, // 用于对外返回的url_base
}

impl NginxStorageProvider {
    pub fn new(
        mut base_url: String,
        req_path_prefix: String,
        mut public_url: String,
    ) -> anyhow::Result<Self> {
        // 验证base_url格式
        let url = UrlParser::parse(&base_url)
            .map_err(|e| anyhow::anyhow!("Invalid nginx base_url: {}", e))?;

        // 验证协议是http或https
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(anyhow::anyhow!(
                "nginx base_url must use http or https protocol"
            ));
        }

        if !base_url.is_empty() && !base_url.ends_with('/') {
            base_url.push('/');
        }
        if !public_url.is_empty() && !public_url.ends_with('/') {
            public_url.push('/');
        }

        Ok(Self {
            base_url,
            req_path_prefix,
            public_url,
        })
    }

    async fn fetch_autoindex(&self, path: &str) -> anyhow::Result<String> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let resp = reqwest::get(&url).await.map_err(|e| {
            if e.is_connect() {
                anyhow::anyhow!("Failed to connect to nginx: {}", e)
            } else {
                anyhow::anyhow!(e)
            }
        })?;

        if resp.status().is_success() {
            resp.text().await.map_err(Into::into)
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(anyhow::anyhow!("nginx returned 404"))
        } else {
            Err(anyhow::anyhow!("nginx returned {}", resp.status()))
        }
    }

    fn parse_entries(&self, html: &str, path: &str) -> anyhow::Result<Vec<StorageEntry>> {
        let document = Html::parse_document(html);
        let selector = Selector::parse("a").unwrap();
        let mut entries = Vec::new();

        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if href == "../" {
                    continue;
                }

                let name = element.text().collect::<String>();
                let url = format!("{}/{}", path.trim_end_matches('/'), href);
                let entry = StorageEntry {
                    name,
                    url,
                    modified: SystemTime::now(), // nginx autoindex doesn't provide modified time
                    size: None,                  // nginx autoindex doesn't provide file size
                };
                entries.push(entry);
            }
        }
        log::debug!("Parsed nginx autoindex entries: {:?}", entries);
        Ok(entries)
    }
}

#[async_trait]
impl StorageProvider for NginxStorageProvider {
    async fn list_directory(
        &self,
        path_in_provider: &str,
    ) -> anyhow::Result<Option<Vec<StorageEntry>>> {
        match self.fetch_autoindex(path_in_provider).await {
            Ok(html) => match self.parse_entries(&html, path_in_provider) {
                Ok(entries) => Ok(Some(entries)),
                Err(e) => {
                    log::error!("Failed to parse nginx autoindex: {}", e);
                    Ok(None)
                }
            },
            Err(e) => {
                if e.to_string().contains("Failed to connect to nginx") {
                    log::error!("Nginx connection failed: {}", e);
                    Err(anyhow::anyhow!("Internal server error").context(e))
                } else if e.to_string().contains("nginx returned 404") {
                    log::debug!("Nginx returned 404: {}", e);
                    Ok(None)
                } else {
                    log::error!("Failed to fetch nginx autoindex: {}", e);
                    Ok(None)
                }
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

    async fn get_download_url(&self, full_path: &str) -> anyhow::Result<Option<String>> {
        log::debug!("Getting download URL for {}", full_path);
        let path_in_provider = match self.path_in_provider(full_path) {
            Some(path) => path,
            None => return Ok(None),
        };

        // 使用public_url构建对外URL
        let url = Url::parse(&self.public_url)
            .map_err(|e| anyhow::anyhow!("Invalid URL: {}", e))?
            .join(&path_in_provider.strip_prefix("/").unwrap_or_default())
            .map_err(|e| anyhow::anyhow!("Failed to join URLs: {}", e))?;

        log::debug!(
            "Download URL for {} is {} (public_host: {})",
            full_path,
            url,
            self.public_url
        );

        Ok(Some(url.to_string()))
    }

    fn is_local(&self) -> bool {
        false
    }
}
