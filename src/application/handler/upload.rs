use std::fs;
use std::path::Path;

use crate::config::Server;
use crate::http::{response::Response, status::StatusCode};
use crate::http::request::Request;

pub fn handle_upload(server: &Server, root: &Path, req: &Request) -> Response {
    println!("Upload hit: method={:?}, len={}", req.method, req.body.len());
    // Enforce method
    if req.method != crate::http::method::Method::Post {
        return Response::new(StatusCode::MethodNotAllowed);
    }

    // Enforce size (client_max_body_size)
    if let Some(max) = server.client_max_body_size {
        if req.body.len() as u64 > max {
            return Response::new(StatusCode::PayloadTooLarge);
        }
    }

    // Get boundary
    let ct = match req.headers.get("Content-Type") {
        Some(v) => v,
        None => return Response::new(StatusCode::BadRequest),
    };
    let boundary = match ct.split("boundary=").nth(1) {
        Some(b) => format!("--{}", b.trim()),
        None => return Response::new(StatusCode::BadRequest),
    };

    // Parse first part
    let (filename, data) = match parse_multipart(&boundary, &req.body) {
        Some(v) => v,
        None => return Response::new(StatusCode::BadRequest),
    };

    // Ensure uploads dir
    let upload_dir = root.join("uploads");
    if let Err(_) = fs::create_dir_all(&upload_dir) {
        return Response::new(StatusCode::InternalServerError);
    }

    // Sanitize filename
    let fname = filename
        .rsplit('/')
        .next()
        .unwrap_or("upload.bin")
        .rsplit('\\')
        .next()
        .unwrap_or("upload.bin");
    let path = upload_dir.join(fname);

    if let Err(_) = fs::write(&path, data) {
        return Response::new(StatusCode::InternalServerError);
    }

    let mut resp = Response::new(StatusCode::SeeOther);
    resp.headers.insert("Location".into(), "/upload.html".into());
    resp
}

fn parse_multipart(boundary: &str, body: &[u8]) -> Option<(String, Vec<u8>)> {
    let delimiter = format!("\r\n{}", boundary);
    let delimiter_bytes = delimiter.as_bytes();
    let boundary_bytes = boundary.as_bytes();

    // Find start of first boundary
    let start_offset = match twoway::find_bytes(body, boundary_bytes) {
        Some(idx) => idx + boundary_bytes.len(),
        None => return None,
    };

    if body.get(start_offset..start_offset + 2) != Some(b"\r\n") {
        return None;
    }

    let mut current_pos = start_offset + 2;

    loop {
        let end_of_part = match twoway::find_bytes(&body[current_pos..], delimiter_bytes) {
            Some(idx) => current_pos + idx,
            None => return None,
        };

        let part_data = &body[current_pos..end_of_part];

        // Split headers/body
        if let Some(header_end_rel) = twoway::find_bytes(part_data, b"\r\n\r\n") {
            let headers_bytes = &part_data[..header_end_rel];
            let content_bytes = &part_data[header_end_rel + 4..];

            // Parse headers
            if let Ok(headers_str) = std::str::from_utf8(headers_bytes) {
                let mut filename = None;
                for line in headers_str.lines() {
                    if let Some(rest) = line.strip_prefix("Content-Disposition:") {
                        if let Some(fn_part) = rest.split("filename=").nth(1) {
                            let fn_val = fn_part.trim().trim_matches('"');
                            if !fn_val.is_empty() {
                                filename = Some(fn_val.to_string());
                            }
                        }
                    }
                }
                if let Some(fname) = filename {
                    return Some((fname, content_bytes.to_vec()));
                }
            }
        }

        current_pos = end_of_part + delimiter_bytes.len();

        // Check for end
        if body.get(current_pos..current_pos + 2) == Some(b"--") {
            break;
        }
        if body.get(current_pos..current_pos + 2) == Some(b"\r\n") {
            current_pos += 2;
        } else {
            break;
        }
    }
    None
}