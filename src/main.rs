use self::error::HttpError;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer};
use storage::select_provider;

use std::path::PathBuf;

#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate lazy_static;

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
