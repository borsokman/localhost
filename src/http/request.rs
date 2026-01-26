use super::headers::Headers;
use super::method::Method;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: Headers,
    pub body: Vec<u8>,
    pub content_length: Option<usize>,
    pub keep_alive: bool,
}