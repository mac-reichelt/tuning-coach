use axum::{
    body::Body,
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../overlay/"]
struct OverlayAssets;

pub async fn index() -> impl IntoResponse {
    serve_asset("index.html")
}

pub async fn src_asset(Path(path): Path<String>) -> impl IntoResponse {
    if !is_safe_relative_path(&path) {
        return StatusCode::NOT_FOUND.into_response();
    }
    serve_asset(&format!("src/{path}"))
}

pub async fn styles_asset(Path(path): Path<String>) -> impl IntoResponse {
    if !is_safe_relative_path(&path) {
        return StatusCode::NOT_FOUND.into_response();
    }
    serve_asset(&format!("styles/{path}"))
}

fn serve_asset(path: &str) -> Response {
    let Some(asset) = OverlayAssets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = content_type_for(path);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(asset.data.into_owned()))
        .expect("overlay response should be valid")
}

fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn is_safe_relative_path(path: &str) -> bool {
    !path.contains("..") && !path.contains('\\') && !path.starts_with('/')
}
