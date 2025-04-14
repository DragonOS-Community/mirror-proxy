use self::error::HttpError;
use crate::config::{has_matching_extension, Config};
use actix_files::NamedFile;
use actix_web::{get, http::header, web, App, HttpRequest, HttpResponse, HttpServer};
use anyhow::Context;
use storage::select_provider;

use std::path::PathBuf;
use std::sync::OnceLock;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate anyhow;

static CONFIG: OnceLock<Config> = OnceLock::new();

mod config;
mod error;
mod render;
mod storage;

const BASE_PATH: &str = "/pub";

async fn handle_download_request(
    path_str: &str,
    req: &HttpRequest,
) -> Result<HttpResponse, HttpError> {
    let config = CONFIG.get().expect("Config not initialized");
    if !has_matching_extension(path_str, &config.download_rules.extensions) {
        return Err(HttpError::not_found("路径不存在", "请求的资源不存在"));
    }

    match select_provider(path_str) {
        Some((provider, path_in_provider)) => {
            if provider.is_local() {
                log::debug!("Local storage provider selected, attempting to stream file (path in provider: {:?})", path_in_provider);
                match provider.stream_file(&path_in_provider).await {
                    Ok(Some(file)) => {
                        return named_file_to_response(
                            file,
                            req.headers().get("range").and_then(|h| h.to_str().ok()),
                            req,
                        )
                        .await
                        .map_err(|_| HttpError::internal_error("服务器错误", "文件处理失败"));
                    }
                    Ok(None) => {
                        return Err(HttpError::not_found("文件不存在", "请求的下载文件不存在"));
                    }
                    Err(e) => {
                        log::error!("文件流处理失败 - 路径: {}, 错误: {}", path_in_provider, e);
                        return Err(HttpError::internal_error("服务器错误", "文件处理失败"));
                    }
                }
            } else {
                match provider.get_download_url(path_str).await {
                    Ok(Some(download_url)) => {
                        return Ok(HttpResponse::Found()
                            .append_header((header::LOCATION, download_url))
                            .finish());
                    }
                    Ok(None) => {
                        return Err(HttpError::not_found("文件不存在", "请求的下载文件不存在"));
                    }
                    Err(e) => {
                        log::error!("Failed to get download URL: {}", e);
                        return Err(HttpError::internal_error("服务器错误", "获取下载链接失败"));
                    }
                }
            }
        }
        None => Err(HttpError::not_found("路径不存在", "请求的资源不存在")),
    }
}

async fn handle_directory_listing(
    path_str: &str,
    full_path: &PathBuf,
) -> Result<HttpResponse, HttpError> {
    let entries = match select_provider(path_str) {
        Some((provider, path_in_provider)) => {
            match provider.list_directory(&path_in_provider).await {
                Ok(Some(entries)) => entries,
                Ok(None) => return Err(HttpError::not_found("目录不存在", "请求的目录不存在")),
                Err(e) => {
                    if e.to_string().contains("Failed to connect to nginx") {
                        log::error!("Nginx connection failed: {}", e);
                        return Err(HttpError::internal_error(
                            "服务器错误",
                            "无法连接到存储服务",
                        ));
                    }
                    log::error!("Failed to list directory: {}", e);
                    return Err(HttpError::internal_error("服务器错误", "获取目录列表失败"));
                }
            }
        }
        None => return Err(HttpError::not_found("路径不存在", "请求的资源不存在")),
    };

    render::render_list(BASE_PATH, full_path.to_str().unwrap(), entries)
        .map(|html| HttpResponse::Ok().content_type("text/html").body(html))
        .map_err(|e| {
            log::error!("渲染目录失败: {}", e);
            HttpError::internal_error("服务器错误", "渲染目录时发生内部错误")
        })
}

fn validate_path(full_path: &PathBuf) -> Result<&str, HttpError> {
    if full_path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        log::warn!("检测到非法路径访问尝试: {:?}", full_path);
        return Err(HttpError::forbidden("访问被拒绝", "请求的路径无效"));
    }

    full_path.to_str().ok_or_else(|| {
        log::warn!("检测到无效路径编码: {:?}", full_path);
        HttpError::bad_request("无效请求", "请求路径无效")
    })
}

#[get("/pub/{path:.*}")]
async fn autoindex(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let base_path = BASE_PATH.to_string();
    log::debug!("Base path: {:?}", base_path);
    log::debug!("Request path: {:?}", path);
    let mut req_path = path.into_inner();

    if req_path.is_empty() {
        req_path = "/".to_string();
    }

    if req_path.starts_with("/") {
        req_path = req_path.trim_start_matches('/').to_string();
    }

    let full_path = PathBuf::from(format!("{}/{}", base_path, req_path));
    log::debug!("Full path: {:?}", full_path);

    let path_str = match validate_path(&full_path) {
        Ok(s) => s,
        Err(e) => return e.to_http_response(),
    };

    let config = CONFIG.get().expect("Config not initialized");
    if has_matching_extension(path_str, &config.download_rules.extensions) {
        match handle_download_request(path_str, &req).await {
            Ok(resp) => resp,
            Err(e) => e.to_http_response(),
        }
    } else {
        match handle_directory_listing(path_str, &full_path).await {
            Ok(resp) => resp,
            Err(e) => e.to_http_response(),
        }
    }
}

async fn named_file_to_response(
    file: NamedFile,
    range_header: Option<&str>,
    req: &HttpRequest,
) -> anyhow::Result<HttpResponse> {
    let metadata = file.file().metadata().context("无法获取文件元数据")?;
    let name = file
        .path()
        .file_name()
        .ok_or(anyhow!("文件名无效"))?
        .to_string_lossy()
        .into_owned();
    let mut response = file.into_response(req);

    // 设置强制下载的headers
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/octet-stream"
            .parse()
            .context("无效的CONTENT_TYPE header值")?,
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", name)
            .parse()
            .context("无效的CONTENT_DISPOSITION header值")?,
    );

    // 保持原有的内容长度和断点续传支持
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        metadata
            .len()
            .to_string()
            .parse()
            .context("无效的CONTENT_LENGTH header值")?,
    );
    if let Some(range) = range_header {
        response.headers_mut().insert(
            header::RANGE,
            range.to_string().parse().context("无效的RANGE header值")?,
        );
    }

    Ok(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = config::load_config("config.toml")
        .await
        .expect("Failed to load config.toml");
    CONFIG.set(config).expect("CONFIG already initialized");

    let mut builder = env_logger::Builder::from_default_env();
    if std::env::var_os("RUST_LOG").is_none() {
        builder.filter_level(log::LevelFilter::Info);
    }
    builder.init();
    HttpServer::new(|| {
        App::new()
            .service(autoindex)
            .default_service(web::route().to(|| async {
                HttpError::not_found("页面不存在", "您访问的页面不存在，请检查URL是否正确")
                    .to_http_response()
            }))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
