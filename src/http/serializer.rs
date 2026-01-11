use crate::http::{response::Response, status::StatusCode};

pub fn serialize_response(resp: &Response) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 + resp.body.len());
    let status = resp.status.as_u16();
    let reason = resp.status.reason();
    out.extend_from_slice(format!("HTTP/1.1 {} {}\r\n", status, reason).as_bytes());

    // ensure Content-Length
    if !resp.headers.contains_key("Content-Length") {
        out.extend_from_slice(format!("Content-Length: {}\r\n", resp.body.len()).as_bytes());
    }
    // default Connection
    if !resp.headers.contains_key("Connection") {
        out.extend_from_slice(b"Connection: close\r\n");
    }
    for (k, v) in resp.headers.iter() {
        out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(&resp.body);
    out
}