use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub servers: Vec<Server>,
}

impl Config {
    pub fn find_server(&self, addr: SocketAddr, host_header: Option<&str>) -> &Server {
        let host = host_header.and_then(|h| h.split(':').next()).unwrap_or("");
        
        // 1. Try to find a server that matches both listen address and server_name
        for srv in &self.servers {
            if srv.listen.contains(&addr) && srv.server_names.iter().any(|n| n == host) {
                return srv;
            }
        }

        // 2. Fallback to the first server that matches the listen address
        for srv in &self.servers {
            if srv.listen.contains(&addr) {
                return srv;
            }
        }

        // 3. Absolute fallback (should not happen if listeners are correctly set up)
        &self.servers[0]
    }
}

#[derive(Debug, Clone)]
pub struct Server {
    pub listen: Vec<SocketAddr>,
    pub server_names: Vec<String>,
    pub root: Option<PathBuf>,
    pub index: Vec<String>,
    pub errors: Vec<ErrorPage>,
    pub client_max_body_size: Option<u64>,
    pub locations: Vec<Location>,
}

impl Server {
    pub fn find_location(&self, path: &str) -> Option<&Location> {
        let mut best_match: Option<&Location> = None;
        for loc in &self.locations {
            if path.starts_with(&loc.path) {
                if let Some(best) = best_match {
                    if loc.path.len() > best.path.len() {
                        best_match = Some(loc);
                    }
                } else {
                    best_match = Some(loc);
                }
            }
        }
        best_match
    }
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

impl From<crate::http::Method> for HttpMethod {
    fn from(m: crate::http::Method) -> Self {
        match m {
            crate::http::Method::Get => HttpMethod::Get,
            crate::http::Method::Post => HttpMethod::Post,
            crate::http::Method::Delete => HttpMethod::Delete,
        }
    }
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