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

#[get("/pub{path:.*}")]
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

    // 构造完整路径
    let full_path = PathBuf::from(format!("{}/{}", base_path, req_path));
    log::debug!("Full path: {:?}", full_path);

    // 检查路径是否包含 `..`，防止跳出 `/pub` 目录
    if full_path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return HttpError::forbidden("访问被拒绝", "路径中包含'..'，可能试图访问上级目录")
            .to_http_response();
    }

    let path_str = match full_path.to_str() {
        Some(s) => s,
        None => {
            return HttpError::bad_request("无效路径编码", "请求路径包含无效字符")
                .to_http_response()
        }
    };

    // 检查是否匹配下载规则
    let config = CONFIG.get().expect("Config not initialized");
    if has_matching_extension(path_str, &config.download_rules.extensions) {
        match select_provider(path_str) {
            Some((provider, path_in_provider)) => {
                if provider.is_local() {
                    log::debug!("Local storage provider selected, attempting to stream file (path in provider: {:?})", path_in_provider);
                    if let Some(file) = provider.stream_file(&path_in_provider).await {
                        return named_file_to_response(
                            file,
                            req.headers().get("range").and_then(|h| h.to_str().ok()),
                            &req,
                        )
                        .await
                        .unwrap_or_else(|_| {
                            HttpError::internal_error("服务器错误", "文件处理失败")
                                .to_http_response()
                        });
                    }
                } else if let Some(download_url) = provider.get_download_url(path_str).await {
                    return HttpResponse::Found()
                        .append_header((header::LOCATION, download_url))
                        .finish();
                }
                return HttpError::not_found("文件不存在", "请求的下载文件不存在")
                    .to_http_response();
            }
            None => {
                return HttpError::not_found("路径不存在", "请求的资源不存在").to_http_response()
            }
        }
    }

    let entries = match select_provider(path_str) {
        Some((provider, path_in_provider)) => provider.list_directory(&path_in_provider).await,
        None => return HttpError::not_found("路径不存在", "请求的资源不存在").to_http_response(),
    };

    if entries.is_none() {
        return HttpError::not_found("目录不存在", "请求的目录不存在").to_http_response();
    }

    let entries = entries.unwrap();

    match render::render_list(BASE_PATH, full_path.to_str().unwrap(), entries) {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => {
            log::error!("渲染目录失败: {}", e);
            HttpError::internal_error("服务器错误", "渲染目录时发生内部错误").to_http_response()
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
        "application/octet-stream".parse().unwrap(),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", name)
            .parse()
            .unwrap(),
    );

    // 保持原有的内容长度和断点续传支持
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        metadata.len().to_string().parse().unwrap(),
    );
    if let Some(range) = range_header {
        response
            .headers_mut()
            .insert(header::RANGE, range.to_string().parse().unwrap());
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
