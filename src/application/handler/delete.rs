use std::fs;
use std::path::Path;

use crate::http::request::Request;
use crate::http::response::Response;
use crate::http::status::StatusCode;
use crate::config::Server;

pub fn handle_delete(_server: &Server, root: &Path, req: &Request, location_prefix: &str) -> Response {
    let rel_path = req.path.strip_prefix(location_prefix).unwrap_or("").trim_start_matches('/');
    let full_path = root.join(rel_path);
    match fs::remove_file(&full_path) {
        Ok(_) => Response::new(StatusCode::Ok),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Response::new(StatusCode::NotFound),
        Err(ref e) if e.kind() == std::io::ErrorKind::PermissionDenied => Response::new(StatusCode::Forbidden),
        Err(_) => Response::new(StatusCode::InternalServerError),
    }
}