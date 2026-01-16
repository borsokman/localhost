#[path = "../core/mod.rs"]
mod core;
#[path = "../application/mod.rs"]
mod application;
#[path = "../http/mod.rs"]
mod http;
#[path = "../config/mod.rs"]
mod config;

use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::time::Duration;

use application::handler::{error_page_handler::error_response, static_file::serve_static, cgi::{start_cgi, parse_cgi_response}, upload::handle_upload};
use application::server::manager::ServerManager;
use config::load_config;
use core::event::EventLoop;
use core::net::connection::{Connection, ConnState};
use core::net::socket::{accept_nonblocking, create_listening_socket};
use http::parser::{parse_request, ParseResult};
use http::serializer::serialize_response;
use http::StatusCode;

fn main() -> Result<(), String> {
    let cfg = load_config(std::path::Path::new("config.conf"))?;
    let event_loop = EventLoop::new()?;
    let mut mgr = ServerManager::new(Duration::from_secs(15));
    let mut listen_map: HashMap<i32, usize> = HashMap::new();
    let mut listen_fds: Vec<core::net::fd::Fd> = Vec::new();

    for (i, srv) in cfg.servers.iter().enumerate() {
        for addr in &srv.listen {
            eprintln!("Listening on {}", addr);
            let fd = create_listening_socket(*addr)?;
            let fd_raw = fd.0;
            event_loop.poller().register_read(fd_raw)?;
            listen_map.insert(fd_raw, i);
            listen_fds.push(fd);
        }
    }

    loop {
        event_loop.tick(64, Some(1000), |ev| {
            // Accept new connections on any listener
            if let Some(&srv_idx) = listen_map.get(&ev.fd) {
                if ev.readable {
                    loop {
                        match accept_nonblocking(ev.fd) {
                            Ok(Some(fd)) => {
                                let fd_raw = fd.as_raw_fd();
                                mgr.insert(fd_raw, Connection::new(fd, srv_idx));
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

                        match &mut conn.state {
                            ConnState::Reading => {
                                if ev.fd == conn_fd && ev.readable {
                                    let mut buf = [0u8; 4096];
                                    loop {
                                        let n = unsafe { libc::read(conn_fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                                        if n > 0 {
                                            let n = n as usize;
                                            conn.read_buf.extend_from_slice(&buf[..n]);
                                            match parse_request(&conn.read_buf, 20 * 1024 * 1024) {
                                                ParseResult::Incomplete => {},
                                                ParseResult::Error(err) => {
                                                    let status = if err == "body too large" { StatusCode::PayloadTooLarge } else { StatusCode::BadRequest };
                                                    let srv = &cfg.servers[conn.server_idx];
                                                    let root: PathBuf = srv.root.clone().unwrap_or_else(|| PathBuf::from("www"));
                                                    let resp = error_response(status, srv, &root);
                                                    let mut bytes = serialize_response(&resp, false);
                                                    conn.write_buf.append(&mut bytes);
                                                    conn.state = ConnState::Writing;
                                                    let _ = event_loop.poller().register_write(conn_fd);
                                                    break;
                                                }
                                                ParseResult::Complete(req, used) => {
                                                    conn.read_buf.drain(0..used);
                                                    conn.keep_alive = req.keep_alive;
                                                    let srv = &cfg.servers[conn.server_idx];
                                                    let root: PathBuf = srv.root.clone().unwrap_or_else(|| PathBuf::from("www"));
                                                    let path_no_q = req.path.split('?').next().unwrap_or("");
                                                    let is_cgi = path_no_q.starts_with("/cgi-bin/") && path_no_q.ends_with(".py");
                                                    
                                                    if is_cgi && matches!(req.method, http::method::Method::Get | http::method::Method::Post | http::method::Method::Delete) {
                                                        match start_cgi(srv, &root, &req) {
                                                            Ok(cgi_proc) => {
                                                                // Register pipes
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
                                                                let mut bytes = serialize_response(&resp, conn.keep_alive);
                                                                conn.write_buf.append(&mut bytes);
                                                                conn.state = ConnState::Writing;
                                                                let _ = event_loop.poller().register_write(conn_fd);
                                                                break;
                                                            }
                                                        }
                                                    } else {
                                                        let resp = if path_no_q == "/upload" {
                                                            handle_upload(srv, &root, &req)  
                                                        } else if !matches!(req.method, http::method::Method::Get) {
                                                            error_response(StatusCode::MethodNotAllowed, srv, &root)
                                                        } else {
                                                            serve_static(srv, &root, &req.path, &["index.html".into()])
                                                        };
                                                        let mut bytes = serialize_response(&resp, conn.keep_alive);
                                                        conn.write_buf.append(&mut bytes);
                                                        conn.state = ConnState::Writing;
                                                        let _ = event_loop.poller().register_write(conn_fd);
                                                        break;
                                                    }
                                                }
                                            }
                                        } else if n == 0 {
                                            conn.state = ConnState::Closing;
                                            break;
                                        } else {
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

                                            let mut bytes = serialize_response(&resp, conn.keep_alive);
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
                                    }
                                    if conn.write_buf.is_empty() {
                                        let _ = event_loop.poller().disable_write(conn_fd);
                                        if conn.keep_alive {
                                            conn.state = ConnState::Reading;
                                        } else {
                                            conn.state = ConnState::Closing;
                                        }
                                    }
                                }
                            },
                            ConnState::Closing => {
                                 let _ = event_loop.poller().deregister(conn_fd);
                                 should_close = true;
                            }
                        }
                    } // end of conn borrow

                    if should_close {
                        mgr.remove(conn_fd);
                    }
                }
            }
        })?;

        for fd in mgr.sweep_timeouts() {
            let _ = event_loop.poller().deregister(fd);
            mgr.remove(fd);
        }
    }
}