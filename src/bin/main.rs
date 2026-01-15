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

use application::handler::{error_page_handler::error_response, static_file::serve_static, cgi::serve_cgi, upload::handle_upload};
use application::server::manager::ServerManager;
use config::load_config;
use core::event::EventLoop;
use core::net::connection::Connection;
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
            } else if let Some(conn) = mgr.get_mut(ev.fd) {
                conn.touch();

                // Readable path
                if ev.readable {
                    let mut buf = [0u8; 4096];
                    loop {
                        let n = unsafe { libc::read(ev.fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                        if n > 0 {
                            let n = n as usize;
                            conn.read_buf.extend_from_slice(&buf[..n]);
                            match parse_request(&conn.read_buf, 20 * 1024 * 1024) {
                                ParseResult::Incomplete => {},
                                ParseResult::Error(err) => {
                                    let status = if err == "body too large" {
                                        StatusCode::PayloadTooLarge
                                    } else {
                                        StatusCode::BadRequest
                                    };
                                    let srv = &cfg.servers[conn.server_idx];
                                    let root: PathBuf = srv
                                        .root
                                        .clone()
                                        .unwrap_or_else(|| PathBuf::from("www"));
                                    let resp = error_response(status, srv, &root);
                                    let mut bytes = serialize_response(&resp, false);
                                    conn.write_buf.append(&mut bytes);
                                    conn.state = core::net::connection::ConnState::Writing;
                                    let _ = event_loop.poller().register_write(ev.fd);
                                    break;
                                }
                                ParseResult::Complete(req, used) => {
                                    conn.read_buf.drain(0..used);
                                    conn.keep_alive = req.keep_alive;
                                    let srv = &cfg.servers[conn.server_idx];
                                    let root: PathBuf = srv.root.clone().unwrap_or_else(|| PathBuf::from("www"));
                                    let path_no_q = req.path.split('?').next().unwrap_or("");
                                    let is_cgi = path_no_q.starts_with("/cgi-bin/") && path_no_q.ends_with(".py");
                                    let resp = if is_cgi {
                                         match req.method {
                                         http::method::Method::Get | http::method::Method::Post | http::method::Method::Delete => {
                                         serve_cgi(srv, &root, &req)
                                    }
                                         _ => error_response(StatusCode::MethodNotAllowed, srv, &root),
                                      }
                                    } else if path_no_q == "/upload" {
                                        handle_upload(srv, &root, &req)  
                                    } else if !matches!(req.method, http::method::Method::Get) {
                                        error_response(StatusCode::MethodNotAllowed, srv, &root)
                                    } else {
                                        serve_static(srv, &root, &req.path, &["index.html".into()])
                                    };
                                    let mut bytes = serialize_response(&resp, conn.keep_alive);
                                    conn.write_buf.append(&mut bytes);
                                    conn.state = core::net::connection::ConnState::Writing;
                                    let _ = event_loop.poller().register_write(ev.fd);
                                    break;
                                }
                            }
                        } else if n == 0 {
                            conn.state = core::net::connection::ConnState::Closing;
                            break;
                        } else {
                            break; // EAGAIN/EWOULDBLOCK or error flagged by ev.error
                        }
                    }
                }

                // Writable path
                if ev.writable && !conn.write_buf.is_empty() {
                    let n = unsafe {
                        libc::write(
                            ev.fd,
                            conn.write_buf.as_ptr() as *const _,
                            conn.write_buf.len(),
                        )
                    };
                    if n > 0 {
                        let n = n as usize;
                        conn.write_buf.drain(0..n);
                    }
                    if conn.write_buf.is_empty() {
                        let _ = event_loop.poller().disable_write(ev.fd);
                        if conn.keep_alive {
                            conn.state = core::net::connection::ConnState::Reading;
                        } else {
                            conn.state = core::net::connection::ConnState::Closing;
                        }
                    }
                }

                // Cleanup
                if ev.error || ev.eof || matches!(conn.state, core::net::connection::ConnState::Closing)
                {
                    let _ = event_loop.poller().deregister(ev.fd);
                    mgr.remove(ev.fd);
                }
            }
        })?;

        for fd in mgr.sweep_timeouts() {
            let _ = event_loop.poller().deregister(fd);
            mgr.remove(fd);
        }
    }
}