use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../frontend/build/"]
#[exclude = "*.gitkeep"]
struct Asset;

pub async fn static_handler(Path(path): Path<String>) -> Response {
    serve_file(&path)
}

pub async fn index_handler() -> Response {
    serve_file("index.html")
}

fn serve_file(path: &str) -> Response {
    match Asset::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime)],
                file.data.into_owned(),
            )
                .into_response()
        }
        None => match Asset::get("index.html") {
            Some(file) => {
                Html(String::from_utf8_lossy(&file.data).to_string()).into_response()
            }
            None => (StatusCode::NOT_FOUND, "Not found").into_response(),
        },
    }
}
