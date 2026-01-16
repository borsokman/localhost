use std::ffi::CString;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::config::Server;
use crate::http::{method::Method, request::Request, response::Response, status::StatusCode};

pub struct CgiProcess {
    pub pid: i32,
    pub input: Option<RawFd>,
    pub output: RawFd,
}

pub fn start_cgi(_server: &Server, root: &Path, req: &Request) -> Result<CgiProcess, Response> {
    let (path_no_q, query) = split_path_query(&req.path);
    let script = match resolve_script(root, path_no_q) {
        Some(p) => p,
        None => return Err(Response::new(StatusCode::NotFound)),
    };
    if script.extension().and_then(|e| e.to_str()) != Some("py") || !script.is_file() {
        return Err(Response::new(StatusCode::NotFound));
    }

    let mut in_pipe: [RawFd; 2] = [0; 2];
    let mut out_pipe: [RawFd; 2] = [0; 2];
    unsafe {
        if libc::pipe(in_pipe.as_mut_ptr()) != 0 {
            return Err(Response::new(StatusCode::InternalServerError));
        }
        if libc::pipe(out_pipe.as_mut_ptr()) != 0 {
            cleanup_pipes(in_pipe, out_pipe);
            return Err(Response::new(StatusCode::InternalServerError));
        }
        set_nonblock(in_pipe[1]);
        set_nonblock(out_pipe[0]);
    }

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        cleanup_pipes(in_pipe, out_pipe);
        return Err(Response::new(StatusCode::InternalServerError));
    }

    if pid == 0 {
        unsafe {
            libc::dup2(in_pipe[0], libc::STDIN_FILENO);
            libc::dup2(out_pipe[1], libc::STDOUT_FILENO);
            cleanup_pipes(in_pipe, out_pipe);
            if let Some(dir) = script.parent() {
                let _ = libc::chdir(path_cstr(dir).as_ptr());
            }
            let argv_cstr = vec![safe_cstr("python3"), path_cstr(&script)];
            let mut argv: Vec<*const i8> = argv_cstr.iter().map(|s| s.as_ptr()).collect();
            argv.push(std::ptr::null());

            let env_cstr = build_env(req, &script, query);
            let mut envp: Vec<*const i8> = env_cstr.iter().map(|s| s.as_ptr()).collect();
            envp.push(std::ptr::null());

            libc::execve(safe_cstr("/usr/bin/env").as_ptr(), argv.as_ptr(), envp.as_ptr());
            libc::_exit(127);
        }
    }

    // Parent
    unsafe {
        libc::close(in_pipe[0]);
        libc::close(out_pipe[1]);
    }

    let input = if matches!(req.method, Method::Post | Method::Delete) && !req.body.is_empty() {
        Some(in_pipe[1])
    } else {
        unsafe { libc::close(in_pipe[1]); }
        None
    };

    Ok(CgiProcess {
        pid,
        input,
        output: out_pipe[0],
    })
}

pub fn parse_cgi_response(out: &[u8]) -> Response {
    let (headers, body) = split_headers_body(out);
    let mut status = StatusCode::Ok;
    let mut resp = Response::new(StatusCode::Ok);
    for line in headers {
        if let Some(rest) = line.strip_prefix("Status:") {
            if let Some((code_str, _msg)) = rest.trim().split_once(' ') {
                if let Ok(code) = code_str.parse::<u16>() {
                    status = map_status(code);
                }
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            resp.headers.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    resp.status = status;
    resp.body = body.to_vec();
    resp
}

fn resolve_script(root: &Path, req_path: &str) -> Option<PathBuf> {
    let clean = req_path.trim_start_matches('/');
    let path = Path::new(clean);
    if path.starts_with("cgi-bin") {
        root.join(path).canonicalize().ok()
    } else {
        None
    }
}

fn cleanup_pipes(a: [RawFd; 2], b: [RawFd; 2]) {
    unsafe {
        libc::close(a[0]);
        libc::close(a[1]);
        libc::close(b[0]);
        libc::close(b[1]);
    }
}

fn safe_cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_else(|_| CString::new("").unwrap())
}

fn path_cstr(p: &Path) -> CString {
    CString::new(p.as_os_str().as_bytes().iter().copied().filter(|&b| b != 0).collect::<Vec<u8>>())
        .unwrap_or_else(|_| CString::new("").unwrap())
}

fn build_env(req: &Request, script: &Path, query: &str) -> Vec<CString> {
    let mut env = Vec::new();
    env.push(safe_cstr(&format!("REQUEST_METHOD={}", method_to_str(&req.method))));
    env.push(safe_cstr(&format!("QUERY_STRING={}", query)));
    env.push(safe_cstr("SERVER_PROTOCOL=HTTP/1.1"));
    env.push(safe_cstr("GATEWAY_INTERFACE=CGI/1.1"));
    env.push(safe_cstr(&format!("CONTENT_LENGTH={}", req.content_length.unwrap_or(0))));
    if let Some(ct) = req.headers.get("Content-Type") {
        env.push(safe_cstr(&format!("CONTENT_TYPE={}", ct)));
    }
    let full = script.canonicalize().unwrap_or_else(|_| script.to_path_buf());
    env.push(safe_cstr(&format!("PATH_INFO={}", full.display())));
    env
}

fn set_nonblock(fd: RawFd) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
}

fn split_path_query(path: &str) -> (&str, &str) {
    if let Some(idx) = path.find('?') {
        (&path[..idx], &path[idx + 1..])
    } else {
        (path, "")
    }
}

fn split_headers_body(buf: &[u8]) -> (Vec<String>, &[u8]) {
    let mut boundary = None;
    for i in 0..buf.len().saturating_sub(3) {
        if &buf[i..i + 4] == b"\r\n\r\n" {
            boundary = Some((i, 4));
            break;
        }
    }
    if boundary.is_none() {
        for i in 0..buf.len().saturating_sub(1) {
            if &buf[i..i + 2] == b"\n\n" {
                boundary = Some((i, 2));
                break;
            }
        }
    }

    if let Some((idx, sep_len)) = boundary {
        let headers_bytes = &buf[..idx];
        let headers = headers_bytes
            .split(|b| *b == b'\n')
            .filter_map(|line| {
                let line = if line.ends_with(b"\r") { &line[..line.len() - 1] } else { line };
                if line.is_empty() { None } else { Some(String::from_utf8_lossy(line).to_string()) }
            })
            .collect();
        let body = &buf[idx + sep_len..];
        (headers, body)
    } else {
        (Vec::new(), buf)
    }
}

fn map_status(code: u16) -> StatusCode {
    match code {
        200 => StatusCode::Ok,
        400 => StatusCode::BadRequest,
        403 => StatusCode::Forbidden,
        404 => StatusCode::NotFound,
        405 => StatusCode::MethodNotAllowed,
        413 => StatusCode::PayloadTooLarge,
        500 => StatusCode::InternalServerError,
        _ => StatusCode::InternalServerError,
    }
}

fn method_to_str(method: &Method) -> &'static str {
    match method {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Delete => "DELETE",
    }
}