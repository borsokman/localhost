use crate::http::response::Response;

pub fn serialize_response(resp: &Response, keep_alive: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 + resp.body.len());
    out.extend_from_slice(
        format!(
            "HTTP/1.1 {} {}\r\n",
            resp.status.as_u16(),
            resp.status.reason()
        )
        .as_bytes(),
    );

    if !resp.headers.contains_key("Content-Length") {
        out.extend_from_slice(format!("Content-Length: {}\r\n", resp.body.len()).as_bytes());
    }
    if !resp.headers.contains_key("Connection") {
        if keep_alive {
            out.extend_from_slice(b"Connection: keep-alive\r\n");
        } else {
            out.extend_from_slice(b"Connection: close\r\n");
        }
    }

    for (k, v) in resp.headers.iter() {
        out.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(&resp.body);
    out
}