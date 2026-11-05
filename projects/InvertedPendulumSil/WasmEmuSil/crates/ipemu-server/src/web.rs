//! SPA の静的配信。`/api` 以外のパスは index.html にフォールバックする (クライアントルーティング)。

use crate::state::AppState;
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use std::path::{Component, Path, PathBuf};

pub async fn serve(axum::extract::State(st): axum::extract::State<AppState>, uri: Uri) -> Response {
    let Some(dir) = st.config.web_dir.as_deref() else {
        return (
            StatusCode::NOT_FOUND,
            "SPA は配信されていません (IPEMU_WEB_DIR を設定してください)",
        )
            .into_response();
    };
    let rel = uri.path().trim_start_matches('/');
    let candidate = safe_join(dir, rel);
    let (path, immutable) = match candidate {
        Some(p) if p.is_file() => {
            // Vite の成果物はハッシュ付きファイル名なので assets/ 配下は immutable にできる
            let imm = rel.starts_with("assets/");
            (p, imm)
        }
        _ => (dir.join("index.html"), false),
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            let mut resp = Response::new(axum::body::Body::from(bytes));
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref()).unwrap(),
            );
            resp.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static(if immutable {
                    "public, max-age=31536000, immutable"
                } else {
                    "no-cache"
                }),
            );
            resp
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

/// `..` や絶対パスを含む相対パスは拒否する。
fn safe_join(root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    let mut out = root.to_path_buf();
    for c in Path::new(rel).components() {
        match c {
            Component::Normal(s) => out.push(s),
            _ => return None,
        }
    }
    Some(out)
}
