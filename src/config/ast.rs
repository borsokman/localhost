use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub servers: Vec<Server>,
}

#[derive(Debug, Clone)]
pub struct Server {
    pub listen: Vec<SocketAddr>,
    pub server_names: Vec<String>,
    pub root: Option<PathBuf>,
    pub index: Vec<String>,
    pub error_pages: Vec<ErrorPage>,
    pub client_max_body_size: Option<u64>,
    pub locations: Vec<Location>,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub path: String,
    pub root: Option<PathBuf>,
    pub methods: Option<Vec<HttpMethod>>,
    pub redirect: Option<String>,
    pub autoindex: Option<bool>,
    pub default_file: Option<String>,
    pub cgi: Option<Cgi>,
    pub body_limit: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Delete,
}

#[derive(Debug, Clone)]
pub struct ErrorPage {
    pub code: u16,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Cgi {
    pub extension: String,
    pub interpreter: PathBuf,
}