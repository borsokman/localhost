use std::fs;
use std::path::{Path, PathBuf};
use crate::http::{Response, StatusCode};
use crate::config::Server; // Import your config types

pub fn error_response(status: StatusCode, server: &Server, root: &Path) -> Response {
    let code = status.as_u16();

    // Look for a custom error page in config
    let custom_path = server.errors.iter()
        .find(|e| e.code == code)
        .map(|e| e.path.clone());

    let file = if let Some(custom) = custom_path {
        // If custom path is absolute, use as is; else, join with root
        let custom_path = PathBuf::from(&custom);
        if custom_path.is_absolute() {
            custom_path
        } else {
            root.join(custom_path)
        }
    } else {
        root.join("errors").join(format!("{}.html", code))
    };

    let mut resp = Response::new(status);

    match fs::read(&file) {
        Ok(bytes) => {
            resp.body = bytes;
            resp.headers.insert("Content-Type".into(), "text/html; charset=utf-8".into());
        }
        Err(_) => {
            resp.body = format!(
                "<html><head><title>{code} {}</title></head>\
                 <body><h1>{code} {}</h1><p>{}</p></body></html>",
                status.reason(),
                status.reason(),
                default_message(code)
            ).into_bytes();
            resp.headers.insert("Content-Type".into(), "text/html; charset=utf-8".into());
        }
    }
    resp
}

fn default_message(code: u16) -> &'static str {
    match code {
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Unknown Error",
    }
}