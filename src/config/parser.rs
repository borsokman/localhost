use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use super::ast::*;

#[derive(Debug, Clone)]
enum Token {
    Ident(String),
    StringLit(String),
    Number(u64),
    LBrace,
    RBrace,
    Semi,
}

pub fn parse_config(input: &str, base_dir: &Path) -> Result<Config, String> {
    let tokens = tokenize(input)?;
    let mut p = Parser { tokens, pos: 0, base_dir };
    let cfg = p.parse_config()?;

    // Validation
    if cfg.servers.is_empty() {
        return Err("No servers defined".into());
    }
    for (i, s) in cfg.servers.iter().enumerate() {
        if s.listen.is_empty() {
            return Err(format!("Server #{i} missing listen directive"));
        }
    }

    Ok(cfg)
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            '{' => { chars.next(); tokens.push(Token::LBrace); }
            '}' => { chars.next(); tokens.push(Token::RBrace); }
            ';' => { chars.next(); tokens.push(Token::Semi); }
            '#' => { while let Some(ch) = chars.next() { if ch == '\n' { break; } } }
            '"' => {
                chars.next();
                let mut s = String::new();
                let mut terminated = false;
                while let Some(ch) = chars.next() {
                    if ch == '"' {
                        terminated = true;
                        break;
                    }
                    s.push(ch);
                }
                if !terminated {
                    return Err("Unterminated string literal".into());
                }
                tokens.push(Token::StringLit(s));
            }
            c if c.is_ascii_whitespace() => { chars.next(); }
            c if c.is_ascii_digit() => {
                let mut s = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_whitespace() || ch == '{' || ch == '}' || ch == ';' { 
                        break; 
                    }
                    s.push(ch);
                    chars.next();
              }
              if s.chars().all(|ch| ch.is_ascii_digit()) {
                let n = s.parse::<u64>().map_err(|e| e.to_string())?;
                tokens.push(Token::Number(n));
              } else {
                tokens.push(Token::Ident(s));
            }
        }
            _ => {
                let mut s = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_whitespace() || ch == '{' || ch == '}' || ch == ';' {
                        break;
                    }
                    s.push(ch);
                    chars.next();
                }
                tokens.push(Token::Ident(s));
            }
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    base_dir: &'a Path,
}

impl<'a> Parser<'a> {
    fn parse_config(&mut self) -> Result<Config, String> {
        let mut servers = Vec::new();
        while !self.is_end() {
            match self.peek() {
                Some(Token::Ident(s)) if s == "server" => {
                    self.next();
                    self.expect(Token::LBrace)?;
                    servers.push(self.parse_server()?);
                }
                Some(tok) => return Err(format!("Unexpected token at top-level: {:?}", tok)),
                None => break,
            }
        }
        Ok(Config { servers })
    }

    fn parse_server(&mut self) -> Result<Server, String> {
        let mut listen = Vec::new();
        let mut server_names = Vec::new();
        let mut root = None;
        let mut index = Vec::new();
        let mut errors = Vec::new();
        let mut locations = Vec::new();
        let mut client_max_body_size = None;
        let mut keep_alive_timeout = None;

        loop {
            match self.peek() {
                Some(Token::RBrace) => { self.next(); break; }
                Some(Token::Ident(s)) if s == "listen" => {
                    self.next();
                    let addr = self.parse_listen_value()?;
                    listen.push(addr);
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "server_name" => {
                    self.next();
                    loop {
                        match self.peek() {
                            Some(Token::Semi) => break,
                            Some(Token::Ident(name)) => { server_names.push(name.clone()); self.next(); }
                            Some(Token::StringLit(name)) => { server_names.push(name.clone()); self.next(); }
                            other => return Err(format!("Unexpected in server_name: {:?}", other)),
                        }
                    }
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "root" => {
                    self.next();
                    let path = self.parse_path()?;
                    root = Some(path);
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "index" => {
                    self.next();
                    loop {
                        match self.peek() {
                            Some(Token::Semi) => break,
                            Some(Token::Ident(v)) => { index.push(v.clone()); self.next(); }
                            Some(Token::StringLit(v)) => { index.push(v.clone()); self.next(); }
                            other => return Err(format!("Unexpected in index: {:?}", other)),
                        }
                    }
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "error_page" => {
                    self.next();
                    let code = self.expect_number_u16()?;
                    let path = self.expect_stringish()?;
                    errors.push(ErrorPage { code, path });
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "location" => {
                    self.next();
                    let path = self.expect_stringish()?;
                    self.expect(Token::LBrace)?;
                    let mut loc = self.parse_location(path)?;
                    if loc.root.is_none() {
                        loc.root = root.clone(); // inherit root
                    }
                    locations.push(loc);
                }
                Some(Token::Ident(s)) if s == "client_max_body_size" => {
                    self.next();
                    client_max_body_size = Some(self.expect_number_u64()?);
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "keep_alive_timeout" => {
                    self.next();
                    keep_alive_timeout = Some(self.expect_number_u64()?);
                    self.expect(Token::Semi)?;
                }
                Some(tok) => return Err(format!("Unknown directive in server: {:?}", tok)),
                None => return Err("Unexpected EOF in server block".into()),
            }
        }

        Ok(Server {
            listen,
            server_names,
            root,
            index,
            errors,
            locations,
            client_max_body_size,
            keep_alive_timeout,
        })
    }

    fn parse_location(&mut self, path: String) -> Result<Location, String> {
        let mut root = None;
        let mut methods = None;
        let mut redirect = None;
        let mut autoindex = None;
        let mut default_file = None;
        let mut cgi = None;
        let mut body_limit = None;

        loop {
            match self.peek() {
                Some(Token::RBrace) => { self.next(); break; }
                Some(Token::Ident(s)) if s == "root" => {
                    self.next();
                    root = Some(self.parse_path()?);
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "methods" => {
                    self.next();
                    let mut ms = Vec::new();
                    loop {
                        match self.peek() {
                            Some(Token::Semi) => break,
                            Some(Token::Ident(m)) => { ms.push(self.parse_method(m)?); self.next(); }
                            other => return Err(format!("Unexpected in methods: {:?}", other)),
                        }
                    }
                    methods = Some(ms);
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "redirect" => {
                    self.next();
                    redirect = Some(self.expect_stringish()?);
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "autoindex" => {
                    self.next();
                    let v = self.expect_ident()?.to_lowercase();
                    autoindex = match v.as_str() {
                        "on" => Some(true),
                        "off" => Some(false),
                        _ => return Err("autoindex expects on|off".into()),
                    };
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "default_file" => {
                    self.next();
                    default_file = Some(self.expect_stringish()?);
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "cgi" => {
                    self.next();
                    let ext = self.expect_stringish()?;
                    let interpreter = PathBuf::from(self.expect_stringish()?);
                    cgi = Some(Cgi { extension: ext, interpreter });
                    self.expect(Token::Semi)?;
                }
                Some(Token::Ident(s)) if s == "body_limit" => {
                    self.next();
                    body_limit = Some(self.expect_number_u64()?);
                    self.expect(Token::Semi)?;
                }
                Some(tok) => return Err(format!("Unknown directive in location: {:?}", tok)),
                None => return Err("Unexpected EOF in location block".into()),
            }
        }

        Ok(Location {
            path,
            root,
            methods,
            redirect,
            autoindex,
            default_file,
            cgi,
            body_limit,
        })
    }

    fn parse_listen_value(&mut self) -> Result<SocketAddr, String> {
        match self.next() {
            Some(Token::Ident(s)) | Some(Token::StringLit(s)) => self.parse_socket_addr(&s),
            Some(Token::Number(n)) => {
                let addr = format!("0.0.0.0:{}", n);
                self.parse_socket_addr(&addr)
            }
            other => Err(format!("Expected listen address, got {:?}", other)),
        }
    }

    fn parse_socket_addr(&self, s: &str) -> Result<SocketAddr, String> {
        // Try full parse (handles IPv6 like [::1]:8080 or ::1:8080)
        if let Ok(a) = s.parse::<SocketAddr>() {
            return Ok(a);
        }
        // If only a port was given, default to 0.0.0.0
        if let Ok(port) = s.parse::<u16>() {
            return format!("0.0.0.0:{port}")
                .parse::<SocketAddr>()
                .map_err(|e| e.to_string());
        }
        Err(format!("Invalid listen address: {s}"))
    }

    fn parse_path(&mut self) -> Result<PathBuf, String> {
        let p = self.expect_stringish()?;
        let pb = PathBuf::from(&p);
        if pb.is_absolute() {
            Ok(pb)
        } else {
            Ok(self.base_dir.join(pb))
        }
    }

    fn parse_method(&self, s: &str) -> Result<HttpMethod, String> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "DELETE" => Ok(HttpMethod::Delete),
            _ => Err(format!("Unsupported method {}", s)),
        }
    }

    // token helpers
    fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos) }
    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() { self.pos += 1; }
        t
    }
    fn expect(&mut self, want: Token) -> Result<(), String> {
        let got = self.next().ok_or_else(|| "Unexpected EOF".to_string())?;
        if std::mem::discriminant(&got) == std::mem::discriminant(&want) {
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", want, got))
        }
    }
    fn expect_ident(&mut self) -> Result<String, String> {
        match self.next() {
            Some(Token::Ident(s)) => Ok(s),
            other => Err(format!("Expected identifier, got {:?}", other)),
        }
    }
    fn expect_stringish(&mut self) -> Result<String, String> {
        match self.next() {
            Some(Token::Ident(s)) => Ok(s),
            Some(Token::StringLit(s)) => Ok(s),
            other => Err(format!("Expected string, got {:?}", other)),
        }
    }
    fn expect_number_u64(&mut self) -> Result<u64, String> {
        match self.next() {
            Some(Token::Number(n)) => Ok(n),
            other => Err(format!("Expected number, got {:?}", other)),
        }
    }
    fn expect_number_u16(&mut self) -> Result<u16, String> {
        let n = self.expect_number_u64()?;
        u16::try_from(n).map_err(|_| "Number out of range for u16".into())
    }
    fn is_end(&self) -> bool { self.pos >= self.tokens.len() }
}