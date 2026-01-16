use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use crate::http::{Response, StatusCode};
use crate::application::handler::error_page_handler::error_response;
use crate::config::Server;

const MAX_STATIC_BYTES: u64 = 8 * 1024 * 1024;

fn safe_join(root: &Path, req_path: &str) -> Option<PathBuf> {
    let clean = req_path.trim_start_matches('/');
    let mut out = PathBuf::new();
    for comp in Path::new(clean).components() {
        match comp {
            std::path::Component::Normal(c) => out.push(c),
            std::path::Component::CurDir => {}
            _ => return None, // reject .. and absolute
        }
    }
    Some(root.join(out))
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "application/javascript",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub fn serve_static(server: &Server, root: &Path, path: &str, index: &[String]) -> Response {
    let mut full = match safe_join(root, path) {
        Some(p) => p,
        None => return error_response(StatusCode::NotFound, server, root),
    };
    if full.is_dir() {
        if let Some(idx) = index.first() {
            full = full.join(idx);
        }
    }

    let meta = match fs::metadata(&full) {
        Ok(m) => m,
        Err(e) => {
            return match e.kind() {
                io::ErrorKind::NotFound => error_response(StatusCode::NotFound, server, root),
                io::ErrorKind::PermissionDenied => error_response(StatusCode::Forbidden, server, root),
                _ => error_response(StatusCode::InternalServerError, server, root),
            }
        }
    };
    if !meta.is_file() {
        return error_response(StatusCode::NotFound, server, root);
    }
    if meta.len() > MAX_STATIC_BYTES {
        return error_response(StatusCode::PayloadTooLarge, server, root);
    }

    let mut f = match File::open(&full) {
        Ok(f) => f,
        Err(e) => {
            return match e.kind() {
                io::ErrorKind::NotFound => error_response(StatusCode::NotFound, server, root),
                io::ErrorKind::PermissionDenied => error_response(StatusCode::Forbidden, server, root),
                _ => error_response(StatusCode::InternalServerError, server, root),
            }
        }
    };

    let mut bytes = Vec::with_capacity(meta.len() as usize);
    if let Err(_) = f.read_to_end(&mut bytes) {
        return error_response(StatusCode::InternalServerError, server, root);
    }

    let mut resp = Response::new(StatusCode::Ok);
    resp.body = bytes;
    resp.headers.insert("Content-Type".into(), mime_for(&full).into());
    resp
}