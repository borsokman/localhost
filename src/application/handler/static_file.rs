use std::fs;
use std::path::Path;
use crate::http::{Response, StatusCode};

pub fn serve_static(root: &Path, path: &str, index: &[String]) -> Response {
    let mut resp = Response::new(StatusCode::Ok);
    let clean = path.trim_start_matches('/');
    let mut full = root.join(clean);
    if full.is_dir() {
        if let Some(idx) = index.first() {
            full = full.join(idx);
        }
    }
    match fs::read(&full) {
        Ok(bytes) => {
            resp.body = bytes;
            resp.headers.insert("Content-Type".into(), "application/octet-stream".into());
        }
        Err(_) => {
            resp = Response::new(StatusCode::NotFound);
            resp.body = b"Not Found".to_vec();
        }
    }
    resp
}