#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method { Get, Post, Delete }

impl Method {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Method::Get),
            "POST" => Some(Method::Post),
            "DELETE" => Some(Method::Delete),
            _ => None,
        }
    }
}