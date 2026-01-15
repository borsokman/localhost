use std::ffi::CString;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::config::Server;
use crate::http::{method::Method, request::Request, response::Response, status::StatusCode};

pub fn serve_cgi(_server: &Server, root: &Path, req: &Request) -> Response {
    // Resolve script path
    let (path_no_q, query) = split_path_query(&req.path);
    let script = match resolve_script(root, path_no_q) {
        Some(p) => p,
        None => return Response::new(StatusCode::NotFound),
    };

    // Ensure file exists and is .py
    if script.extension().and_then(|e| e.to_str()) != Some("py") || !script.is_file() {
        return Response::new(StatusCode::NotFound);
    }

    // Create pipes: stdin for child, stdout from child
    let mut in_pipe: [RawFd; 2] = [0; 2];
    let mut out_pipe: [RawFd; 2] = [0; 2];
    unsafe {
        if libc::pipe(in_pipe.as_mut_ptr()) != 0 {
            return Response::new(StatusCode::InternalServerError);
        }
        if libc::pipe(out_pipe.as_mut_ptr()) != 0 {
            libc::close(in_pipe[0]);
            libc::close(in_pipe[1]);
            return Response::new(StatusCode::InternalServerError);
        }
    }

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        cleanup_pipes(in_pipe, out_pipe);
        return Response::new(StatusCode::InternalServerError);
    }

    if pid == 0 {
        // Child
        unsafe {
            // Redirect stdin/stdout
            libc::dup2(in_pipe[0], libc::STDIN_FILENO);
            libc::dup2(out_pipe[1], libc::STDOUT_FILENO);
            cleanup_pipes(in_pipe, out_pipe);

            // chdir to script dir
            if let Some(dir) = script.parent() {
                let _ = libc::chdir(path_cstr(dir).as_ptr());
            }

            // Build argv (null-terminated)
            let argv_cstr = vec![
                CString::new("python3").unwrap(),
                path_cstr(&script),
            ];
            let mut argv: Vec<*const i8> = argv_cstr.iter().map(|s| s.as_ptr()).collect();
            argv.push(std::ptr::null());

            // Build env (null-terminated)
            let env_cstr = build_env(req, &script, query);
            let mut envp: Vec<*const i8> = env_cstr.iter().map(|s| s.as_ptr()).collect();
            envp.push(std::ptr::null());

            // execve
            libc::execve(
                cstr("/usr/bin/env").as_ptr(),
                argv.as_ptr(),
                envp.as_ptr(),
            );
            libc::_exit(127);
        }
    }

    // Parent
    unsafe {
        // Close child-side ends
        libc::close(in_pipe[0]);
        libc::close(out_pipe[1]);
    }

    // Write body to child stdin
    if matches!(req.method, Method::Post | Method::Delete) && !req.body.is_empty() {
        let mut written = 0;
        while written < req.body.len() {
            let n = unsafe {
                libc::write(
                    in_pipe[1],
                    req.body[written..].as_ptr() as *const _,
                    (req.body.len() - written) as libc::size_t,
                )
            };
            if n <= 0 {
                break;
            }
            written += n as usize;
        }
    }
    unsafe { libc::close(in_pipe[1]); }

    // Read child stdout
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(out_pipe[0], buf.as_mut_ptr() as *mut _, buf.len()) };
        if n > 0 {
            out.extend_from_slice(&buf[..n as usize]);
        } else {
            break;
        }
    }
    unsafe { libc::close(out_pipe[0]); }

    // Wait for child
    unsafe {
        let mut status: libc::c_int = 0;
        libc::waitpid(pid, &mut status, 0);
    }

    // Parse CGI response (headers \r\n\r\n body)
    let (headers, body) = split_headers_body(&out);
    let mut resp_headers = Response::new(StatusCode::Ok);
    // Status header
    let mut status = StatusCode::Ok;
    for line in headers {
        if let Some(rest) = line.strip_prefix("Status:") {
            let trimmed = rest.trim();
            if let Some((code_str, _msg)) = trimmed.split_once(' ') {
                if let Ok(code) = code_str.parse::<u16>() {
                    status = map_status(code);
                }
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            resp_headers.headers.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    
    let mut resp = Response::new(status);
    resp.headers = resp_headers.headers;
    resp.body = body.to_vec();
    resp
}

fn resolve_script(root: &Path, req_path: &str) -> Option<PathBuf> {
    let clean = req_path.trim_start_matches('/');
    let path = Path::new(clean);
    if path.starts_with("cgi-bin") {
        // Some(root.join(path));
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

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn path_cstr(p: &Path) -> CString {
    CString::new(p.as_os_str().as_bytes()).unwrap()
}

fn build_env(req: &Request, script: &Path, query: &str) -> Vec<CString> {
    let mut env = Vec::new();
    env.push(cstr(&format!("REQUEST_METHOD={}", method_to_str(&req.method))));
    env.push(cstr(&format!("QUERY_STRING={}", query)));
    env.push(cstr("SERVER_PROTOCOL=HTTP/1.1"));
    env.push(cstr("GATEWAY_INTERFACE=CGI/1.1"));
    env.push(cstr(&format!(
        "CONTENT_LENGTH={}",
        req.content_length.unwrap_or(0)
    )));
    if let Some(ct) = req.headers.get("Content-Type") {
        env.push(cstr(&format!("CONTENT_TYPE={}", ct)));
    }
    let full = script.canonicalize().unwrap_or_else(|_| script.to_path_buf());
    env.push(cstr(&format!("PATH_INFO={}", full.display())));
    env
}

fn split_headers_body(buf: &[u8]) -> (Vec<String>, &[u8]) {
    if let Some(idx) = twoway::find_bytes(buf, b"\r\n\r\n").or_else(|| twoway::find_bytes(buf, b"\n\n")) {
        let sep_len = if buf.get(idx + 1) == Some(&b'\n') && buf.get(idx) == Some(&b'\r') { 4 } else { 2 };
        let head = &buf[..idx];
        let body = &buf[idx + sep_len..];
        let headers = head
            .split(|&b| b == b'\n')
            .filter_map(|line| std::str::from_utf8(line).ok())
            .map(|s| s.trim_end_matches('\r').to_string())
            .collect();
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

fn method_to_str(m: &Method) -> &'static str {
    match m {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Delete => "DELETE",
    }
}

fn split_path_query(path: &str) -> (&str, &str) {
    if let Some((p, q)) = path.split_once('?') {
    (p, q)
    } else {
        (path, "")
    }
}