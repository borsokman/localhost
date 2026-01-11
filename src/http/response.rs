use super::headers::Headers;
use super::status::StatusCode;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: StatusCode,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self { status, headers: Headers::new(), body: Vec::new() }
    }
}