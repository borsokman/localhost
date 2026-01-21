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

pub fn serve_static(server: &Server, root: &Path, path: &str, location_prefix: &str, index: &[String], autoindex: bool) -> Response {
    // Strip the location prefix from the request path
    let rel_path = path.strip_prefix(location_prefix).unwrap_or("");
    let full_path = match safe_join(root, rel_path) {
        Some(p) => p,
        None => return error_response(StatusCode::NotFound, server, root),
    };

    let mut target = full_path.clone();
    if target.is_dir() {
        for idx in index {
            let candidate = target.join(idx);
            if candidate.is_file() {
                target = candidate;
                break;
            }
        }
    }

    if target.is_dir() {
        if autoindex {
            return serve_autoindex(server, root, path, &target);
        } else {
            return error_response(StatusCode::Forbidden, server, root);
        }
    }

    let meta = match fs::metadata(&target) {
        Ok(m) => m,
        Err(e) => {
            return match e.kind() {
                io::ErrorKind::NotFound => error_response(StatusCode::NotFound, server, root),
                io::ErrorKind::PermissionDenied => error_response(StatusCode::Forbidden, server, root),
                _ => error_response(StatusCode::InternalServerError, server, root),
            }
        }
    };
    
    if meta.len() > MAX_STATIC_BYTES {
        return error_response(StatusCode::PayloadTooLarge, server, root);
    }

    let mut f = match File::open(&target) {
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
    resp.headers.insert("Content-Type".into(), mime_for(&target).into());
    resp
}

fn serve_autoindex(server: &Server, root: &Path, req_path: &str, dir_path: &Path) -> Response {
    let entries = match fs::read_dir(dir_path) {
        Ok(e) => e,
        Err(_) => return error_response(StatusCode::InternalServerError, server, root),
    };

    let mut html = format!("<html><head><title>Index of {req_path}</title></head><body><h1>Index of {req_path}</h1><hr><ul>");
    for entry in entries {
        if let Ok(entry) = entry {
            let name = entry.file_name().to_string_lossy().into_owned();
            let slash = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { "/" } else { "" };
            html.push_str(&format!("<li><a href=\"{}/{}{}\">{}{}</a></li>", req_path.trim_end_matches('/'), name, slash, name, slash));
        }
    }
    html.push_str("</ul><hr></body></html>");

    let mut resp = Response::new(StatusCode::Ok);
    resp.body = html.into_bytes();
    resp.headers.insert("Content-Type".into(), "text/html; charset=utf-8".into());
    resp
}