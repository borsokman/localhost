#[path = "../core/mod.rs"]
mod core;
#[path = "../application/mod.rs"]
mod application;
#[path = "../http/mod.rs"]
mod http;
#[path = "../config/mod.rs"]
mod config;

use std::collections::{HashMap, HashSet};
use std::io;
use std::net::SocketAddr;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::Duration;

use application::handler::{error_page_handler::error_response, static_file::serve_static, cgi::{start_cgi, parse_cgi_response}, upload::handle_upload, delete::handle_delete};
use application::server::manager::ServerManager;
use config::load_config;
use core::event::EventLoop;
use core::net::connection::{Connection, ConnState};
use core::net::socket::{accept_nonblocking, create_listening_socket};
use http::parser::{parse_request, ParseResult};
use http::serializer::serialize_response;
use http::{Response, StatusCode};

fn main() -> Result<(), String> {
    let cfg = load_config(std::path::Path::new("config.conf"))?;
    let event_loop = EventLoop::new()?;
    let mut mgr = ServerManager::new();
    let mut listen_map: HashMap<i32, SocketAddr> = HashMap::new();
    let mut listen_fds: Vec<core::net::fd::Fd> = Vec::new();
    let mut seen = HashSet::new();

    for srv in &cfg.servers {
        for addr in &srv.listen {
            for addr in &srv.listen {
                for name in &srv.server_names {
                    let key = (addr, name);
                    if !seen.insert(key.clone()) {
                       eprintln!("Config error: duplicate listen address {} with server_name '{}'", addr, name);
                       std::process::exit(1);
                    }
                }
            }
            if !listen_map.values().any(|a| a == addr) {
                eprintln!("Listening on {}", addr);
                let fd = create_listening_socket(*addr)?;
                let fd_raw = fd.0;
                event_loop.poller().register_read(fd_raw)?;
                listen_map.insert(fd_raw, *addr);
                listen_fds.push(fd);
            }
        }
    }

    loop {
        event_loop.tick(64, Some(1000), |ev| {
            // Accept new connections on any listener
            if let Some(&local_addr) = listen_map.get(&ev.fd) {
                if ev.readable {
                    loop {
                        match accept_nonblocking(ev.fd) {
                            Ok(Some(fd)) => {
                                let fd_raw = fd.as_raw_fd();
                                let srv = cfg.find_server(local_addr, None);
                                let timeout = Duration::from_secs(srv.keep_alive_timeout.unwrap_or(75));
                                mgr.insert(fd_raw, Connection::new(fd, local_addr, timeout));
                                let _ = event_loop.poller().register_read(fd_raw);
                            }
                            Ok(None) => break,
                            Err(e) => {
                                eprintln!("accept error: {e}");
                                break;
                            }
                        }
                    }
                }
            } else {
                // Determine connection FD
                let conn_fd_opt = if mgr.conns.contains_key(&ev.fd) {
                    Some(ev.fd)
                } else {
                    mgr.pipe_map.get(&ev.fd).cloned()
                };

                if let Some(conn_fd) = conn_fd_opt {
                    let mut should_close = false;
                    {
                        // Disjoint borrow: conn from mgr.conns
                        let conn = mgr.conns.get_mut(&conn_fd).unwrap();
                        conn.touch();

                        // Check for connection errors or EOF first - close immediately
                        if ev.error || (ev.eof && ev.fd == conn_fd) {
                            conn.state = ConnState::Closing;
                        }

                        // If state is already Closing, handle it immediately
                        if matches!(conn.state, ConnState::Closing) {
                            let _ = event_loop.poller().deregister(conn_fd);
                            unsafe { libc::close(conn_fd) };
                            should_close = true;
                        } else {
                            match &mut conn.state {
                            ConnState::Reading => {
                                if ev.fd == conn_fd && ev.readable {
                                    let mut buf = [0u8; 4096];
                                    loop {
                                        let n = unsafe { libc::read(conn_fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                                        if n > 0 {
                                            let n = n as usize;
                                            conn.read_buf.extend_from_slice(&buf[..n]);
                                            
                                            // We don't know the exact limit yet until we parse the Host header,
                                            // so use a reasonable global max for initial parsing.
                                            match parse_request(&conn.read_buf, 100 * 1024 * 1024) {
                                                ParseResult::Incomplete => {},
                                                ParseResult::Error(err) => {
                                                    let status = if err == "body too large" { StatusCode::PayloadTooLarge } else { StatusCode::BadRequest };
                                                    // Use default server for this port for error response
                                                    let srv = cfg.find_server(conn.local_addr, None);
                                                    let root = srv.root.as_deref().unwrap_or(Path::new("www"));
                                                    let resp = error_response(status, srv, root);
                                                    let mut bytes = serialize_response(&resp, false, conn.timeout);
                                                    conn.write_buf.append(&mut bytes);
                                                    conn.state = ConnState::Writing;
                                                    let _ = event_loop.poller().register_write(conn_fd);
                                                    break;
                                                }
                                                ParseResult::Complete(req, used) => {
                                                    conn.read_buf.drain(0..used);
                                                    conn.keep_alive = req.keep_alive;
                                                    
                                                    let host_header = req.headers.get("Host").map(|s| s.as_str());
                                                    let srv = cfg.find_server(conn.local_addr, host_header);
                                                    let loc = srv.find_location(&req.path);
                                                    
                                                    // 1. Check body limit
                                                    let limit = loc.and_then(|l| l.body_limit).or(srv.client_max_body_size).unwrap_or(20 * 1024 * 1024);
                                                    if req.body.len() as u64 > limit {
                                                        let root = loc.and_then(|l| l.root.as_deref()).or(srv.root.as_deref()).unwrap_or(Path::new("www"));
                                                        let resp = error_response(StatusCode::PayloadTooLarge, srv, root);
                                                        let mut bytes = serialize_response(&resp, conn.keep_alive, conn.timeout);
                                                        conn.write_buf.append(&mut bytes);
                                                        conn.state = ConnState::Writing;
                                                        let _ = event_loop.poller().register_write(conn_fd);
                                                        break;
                                                    }

                                                    // 2. Check methods
                                                    if let Some(l) = loc {
                                                        if let Some(allowed) = &l.methods {
                                                            if !allowed.contains(&req.method.into()) {
                                                                let root = l.root.as_deref().or(srv.root.as_deref()).unwrap_or(Path::new("www"));
                                                                let resp = error_response(StatusCode::MethodNotAllowed, srv, root);
                                                                let mut bytes = serialize_response(&resp, conn.keep_alive, conn.timeout);
                                                                conn.write_buf.append(&mut bytes);
                                                                conn.state = ConnState::Writing;
                                                                let _ = event_loop.poller().register_write(conn_fd);
                                                                break;
                                                            }
                                                        }
                                                    }

                                                    // 3. Handle redirect
                                                    if let Some(l) = loc {
                                                        if let Some(redir) = &l.redirect {
                                                            let mut resp = Response::new(StatusCode::MovedPermanently);
                                                            resp.headers.insert("Location".into(), redir.clone());
                                                            let mut bytes = serialize_response(&resp, conn.keep_alive, conn.timeout);
                                                            conn.write_buf.append(&mut bytes);
                                                            conn.state = ConnState::Writing;
                                                            let _ = event_loop.poller().register_write(conn_fd);
                                                            break;
                                                        }
                                                    }

                                                    let loc_root = loc.and_then(|l| l.root.as_deref());
                                                    let root = loc_root.or(srv.root.as_deref()).unwrap_or(Path::new("www"));
                                                    let path_no_q = req.path.split('?').next().unwrap_or("");
                                                    
                                                    // 4. Handle CGI
                                                    if let Some(cgi_config) = loc.and_then(|l| l.cgi.as_ref()) {
                                                        match start_cgi(srv, root, &req, cgi_config) {
                                                            Ok(cgi_proc) => {
                                                                let _ = event_loop.poller().register_read(cgi_proc.output);
                                                                mgr.pipe_map.insert(cgi_proc.output, conn_fd);
                                                                if let Some(input) = cgi_proc.input {
                                                                    let _ = event_loop.poller().register_write(input);
                                                                    mgr.pipe_map.insert(input, conn_fd);
                                                                }
                                                                conn.state = ConnState::Cgi {
                                                                    pid: cgi_proc.pid,
                                                                    input: cgi_proc.input,
                                                                    output: cgi_proc.output,
                                                                    data: Vec::new(),
                                                                };
                                                                break;
                                                            },
                                                            Err(resp) => {
                                                                let mut bytes = serialize_response(&resp, conn.keep_alive, conn.timeout);
                                                                conn.write_buf.append(&mut bytes);
                                                                conn.state = ConnState::Writing;
                                                                let _ = event_loop.poller().register_write(conn_fd);
                                                                break;
                                                            }
                                                        }
                                                    } else {
                                                        // 5. Handle Static / Upload
                                                        let resp = if path_no_q == "/upload" {
                                                            handle_upload(srv, root, &req)  
                                                        } else if req.method == http::method::Method::Delete {
                                                            let location_prefix = loc.map(|l| l.path.as_str()).unwrap_or("");
                                                            application::handler::delete::handle_delete(srv, root, &req, location_prefix)
                                                        } else {
                                                            let mut indices = srv.index.clone();
                                                            if let Some(l) = loc {
                                                                if let Some(df) = &l.default_file {
                                                                    indices.insert(0, df.clone());
                                                                }
                                                            }
                                                            if indices.is_empty() {
                                                                indices.push("index.html".into());
                                                            }
                                                            let autoindex = loc.and_then(|l| l.autoindex).unwrap_or(false);
                                                            let location_prefix = loc.map(|l| l.path.as_str()).unwrap_or("");
                                                            // Strip prefix only if location root differs from server root
                                                            let strip_prefix = match (loc_root, srv.root.as_deref()) {
                                                                (Some(lr), Some(sr)) => lr != sr,
                                                                (Some(_), None) => true,
                                                                _ => false,
                                                            };
                                                            serve_static(srv, root, &req.path, location_prefix, strip_prefix, &indices, autoindex)
                                                        };
                                                        let mut bytes = serialize_response(&resp, conn.keep_alive, conn.timeout);
                                                        conn.write_buf.append(&mut bytes);
                                                        conn.state = ConnState::Writing;
                                                        let _ = event_loop.poller().register_write(conn_fd);
                                                        break;
                                                    }
                                                }
                                            }
                                        } else if n == 0 {
                                            // EOF - client closed connection
                                            conn.state = ConnState::Closing;
                                            break;
                                        } else {
                                            // Read error - close connection
                                            let err = io::Error::last_os_error();
                                            if err.raw_os_error() != Some(libc::EAGAIN) && err.raw_os_error() != Some(libc::EWOULDBLOCK) {
                                                conn.state = ConnState::Closing;
                                            }
                                            break;
                                        }
                                    }
                                }
                            },
                            ConnState::Cgi { pid, input, output, data } => {
                                let pid_cp = *pid;
                                let output_cp = *output;
                                let input_cp = *input;

                                if ev.fd == output_cp && ev.readable {
                                    let mut buf = [0u8; 4096];
                                    loop {
                                        let n = unsafe { libc::read(output_cp, buf.as_mut_ptr() as *mut _, buf.len()) };
                                        if n > 0 {
                                            data.extend_from_slice(&buf[..n as usize]);
                                        } else if n == 0 {
                                            // EOF
                                            let resp = parse_cgi_response(data);
                                            
                                            // Cleanup
                                            unsafe { libc::close(output_cp); }
                                            let _ = event_loop.poller().deregister(output_cp);
                                            mgr.pipe_map.remove(&output_cp);
                                            if let Some(in_fd) = input_cp {
                                                unsafe { libc::close(in_fd); }
                                                let _ = event_loop.poller().deregister(in_fd);
                                                mgr.pipe_map.remove(&in_fd);
                                            }
                                            unsafe { libc::waitpid(pid_cp, std::ptr::null_mut(), libc::WNOHANG); }

                                            let mut bytes = serialize_response(&resp, conn.keep_alive, conn.timeout);
                                            conn.write_buf.append(&mut bytes);
                                            conn.state = ConnState::Writing;
                                            let _ = event_loop.poller().register_write(conn_fd);
                                            break;
                                        } else {
                                            break;
                                        }
                                    }
                                } else if let Some(in_fd) = input_cp {
                                    if ev.fd == in_fd && ev.writable {
                                        // Assume request body handling needed here, but for now closing
                                        unsafe { libc::close(in_fd); }
                                        let _ = event_loop.poller().deregister(in_fd);
                                        mgr.pipe_map.remove(&in_fd);
                                        if let ConnState::Cgi { input, .. } = &mut conn.state {
                                            *input = None;
                                        }
                                    }
                                }
                            },
                            ConnState::Writing => {
                                 if ev.fd == conn_fd && ev.writable && !conn.write_buf.is_empty() {
                                    let n = unsafe {
                                        libc::write(
                                            conn_fd,
                                            conn.write_buf.as_ptr() as *const _,
                                            conn.write_buf.len(),
                                        )
                                    };
                                    if n > 0 {
                                        let n = n as usize;
                                        conn.write_buf.drain(0..n);
                                        if conn.write_buf.is_empty() {
                                            let _ = event_loop.poller().disable_write(conn_fd);
                                            if conn.keep_alive {
                                                conn.state = ConnState::Reading;
                                            } else {
                                                conn.state = ConnState::Closing;
                                            }
                                        }
                                    } else if n < 0 {
                                        // Write error - close connection
                                        let err = io::Error::last_os_error();
                                        if err.raw_os_error() != Some(libc::EAGAIN) && err.raw_os_error() != Some(libc::EWOULDBLOCK) {
                                            conn.state = ConnState::Closing;
                                        }
                                    }
                                }
                            },
                            ConnState::Closing => {
                                 let _ = event_loop.poller().deregister(conn_fd);
                                 // Explicitly close the socket to free the port immediately
                                 // The Fd's Drop will also try to close it, but closing an already-closed fd is safe
                                 unsafe { libc::close(conn_fd) };
                                 should_close = true;
                            }
                        }
                        } // end of else/match
                    } // end of conn borrow

                    if should_close {
                        mgr.remove(conn_fd);
                    }
                }
            }
        })?;

        for fd in mgr.sweep_timeouts() {
            let _ = event_loop.poller().deregister(fd);
            // Explicitly close the socket before removing from manager
            unsafe { libc::close(fd) };
            mgr.remove(fd);
        }
    }
}