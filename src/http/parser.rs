use crate::http::{method::Method, request::Request};
use std::str;

pub enum ParseResult {
    Incomplete,
    Complete(Request, usize),
    Error(String),
}

pub fn parse_request(buf: &[u8], body_limit: usize) -> ParseResult {
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
    let req_line = match lines.next() {
        Some(line) => line,
        None => return ParseResult::Error("empty request line".into()),
    };
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

    let te_chunked = headers
        .get("Transfer-Encoding")
        .map(|v| v.eq_ignore_ascii_case("chunked"))
        .unwrap_or(false);
    let content_length = if te_chunked {
        None
    } else {
        headers.get("Content-Length").and_then(|v| v.parse::<usize>().ok())
    };
    let keep_alive = match headers.get("Connection").map(|s| s.to_ascii_lowercase()) {
        Some(ref v) if v == "close" => false,
        _ => true,
    };

    if te_chunked {
        // parse chunked body
        let mut idx = headers_end;
        let mut body = Vec::new();
        loop {
            // read chunk size line
            let rest = &buf[idx..];
            let Some(line_end) = twoway::find_bytes(rest, b"\r\n") else { return ParseResult::Incomplete };
            let size_line = &rest[..line_end];
            let size_str = match str::from_utf8(size_line) {
                Ok(s) => s,
                Err(_) => return ParseResult::Error("invalid chunk size".into()),
            };
            let chunk_size = match usize::from_str_radix(size_str.trim(), 16) {
                Ok(n) => n,
                Err(_) => return ParseResult::Error("invalid chunk size".into()),
            };
            idx += line_end + 2; // skip size line + CRLF
            if chunk_size == 0 {
                // consume trailing CRLF after last chunk (and optional trailers)
                if buf.len() < idx + 2 {
                    return ParseResult::Incomplete;
                }
                // skip possible trailers until blank line
                let trailers = &buf[idx..];
                if let Some(tend) = twoway::find_bytes(trailers, b"\r\n\r\n") {
                    idx += tend + 4;
                } else {
                    idx += 2; // no trailers, just CRLF
                }
                return ParseResult::Complete(
                    Request {
                        method,
                        path,
                        headers,
                        body,
                        content_length: None,
                        keep_alive,
                    },
                    idx,
                );
            } else {
                if buf.len() < idx + chunk_size + 2 {
                    return ParseResult::Incomplete;
                }
                body.extend_from_slice(&buf[idx..idx + chunk_size]);
                if body.len() > body_limit {
                    return ParseResult::Error("body too large".into());
                }
                idx += chunk_size + 2; // skip chunk + CRLF
            }
        }
    } else {
        let body_len = content_length.unwrap_or(0);
        if body_len > body_limit {
            return ParseResult::Error("body too large".into());
        }
        let total_needed = headers_end + body_len;
        if buf.len() < total_needed {
            return ParseResult::Incomplete;
        }
        let body = buf[headers_end..total_needed].to_vec();
        return ParseResult::Complete(
            Request {
                method,
                path,
                headers,
                body,
                content_length,
                keep_alive,
            },
            total_needed,
        );
    }
}