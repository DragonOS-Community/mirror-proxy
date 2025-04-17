use std::time::SystemTime;

use askama::Template;

use crate::storage::StorageEntry;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub title: String,
    pub status_code: String,
    pub message: String,
    pub error_code: String,
    pub description: String,
    pub home_url: String,
}

#[derive(Template)]
#[template(path = "autoindex.html")]
struct AutoIndexTemplate {
    path: String,
    entries: Vec<IndexDirEntry>,
}

pub struct IndexDirEntry {
    pub name: String,
    pub url: String,
    pub modified: String,
    pub size: String,
}

impl IndexDirEntry {
    fn parent_entry() -> IndexDirEntry {
        Self {
            name: "../".to_string(),
            url: "../".to_string(),
            modified: "".to_string(),
            size: "".to_string(),
        }
    }
}

impl From<StorageEntry> for IndexDirEntry {
    fn from(entry: StorageEntry) -> Self {
        Self {
            name: entry.name,
            url: entry.url,
            modified: format_time(entry.modified),
            size: format_size(entry.size),
        }
    }
}

fn format_time(time: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn format_size(size: Option<usize>) -> String {
    if let Some(size) = size {
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        }
    } else {
        "-".to_owned()
    }
}

pub fn render_list(
    base_path: &str,
    req_path: &str,
    src_entries: Vec<StorageEntry>,
) -> anyhow::Result<String> {
    let mut entries = Vec::new();
    entries.push(IndexDirEntry::parent_entry());
    src_entries.into_iter().for_each(|e| {
        let mut et: IndexDirEntry = e.into();
        et.url = format!("{}/{}", base_path, et.url);
        entries.push(et);
    });

    let template = AutoIndexTemplate {
        path: req_path.to_string(),
        entries,
    };

    template.render().map_err(|e| anyhow::anyhow!(e))
}
