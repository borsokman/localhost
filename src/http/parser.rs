use crate::http::{method::Method, request::Request};
use std::str;

pub enum ParseResult {
    Incomplete,
    Complete(Request, usize), // request, bytes_consumed
    Error(String),
}

pub fn parse_request(buf: &[u8], body_limit: usize) -> ParseResult {
    // find header end
    let headers_end = match twoway::find_bytes(buf, b"\r\n\r\n") {
        Some(idx) => idx + 4,
        None => return ParseResult::Incomplete,
    };
    let head = &buf[..headers_end];
    let head_str = match str::from_utf8(head) {
        Ok(s) => s,
        Err(_) => return ParseResult::Error("invalid utf8 in headers".into()),
    };
    let mut lines = head_str.split("\r\n").filter(|l| !l.is_empty());
    let req_line = lines.next().ok_or("empty request line").unwrap();
    let mut parts = req_line.split_whitespace();
    let method_str = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("").to_string();
    let version = parts.next().unwrap_or("").to_string();
    let method = match Method::parse(method_str) {
        Some(m) => m,
        None => return ParseResult::Error("unsupported method".into()),
    };
    if !version.starts_with("HTTP/1.") {
        return ParseResult::Error("unsupported version".into());
    }

    let mut headers = std::collections::HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let content_length = headers
        .get("Content-Length")
        .and_then(|v| v.parse::<usize>().ok());
    let keep_alive = match headers.get("Connection").map(|s| s.to_ascii_lowercase()) {
        Some(ref v) if v == "close" => false,
        _ => true,
    };

    let body_len = content_length.unwrap_or(0);
    if body_len > body_limit {
        return ParseResult::Error("body too large".into());
    }
    let total_needed = headers_end + body_len;
    if buf.len() < total_needed {
        return ParseResult::Incomplete;
    }
    let body = buf[headers_end..total_needed].to_vec();

    ParseResult::Complete(
        Request {
            method,
            path,
            version,
            headers,
            body,
            content_length,
            keep_alive,
        },
        total_needed,
    )
}