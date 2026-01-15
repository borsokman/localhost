#[path = "../core/mod.rs"]
mod core;
#[path = "../application/mod.rs"]
mod application;
#[path = "../http/mod.rs"]
mod http;
#[path = "../config/mod.rs"]
mod config;

use std::net::SocketAddr;
use std::time::Duration;
use std::os::fd::AsRawFd;

use application::handler::{static_file::serve_static, error_page_handler::error_response};
use application::server::manager::ServerManager;
use core::event::EventLoop;
use core::net::connection::Connection;
use core::net::socket::{accept_nonblocking, create_listening_socket};
use http::parser::{parse_request, ParseResult};
use http::serializer::serialize_response;
use crate::http::StatusCode;

fn main() -> Result<(), String> {
    let listen_addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    eprintln!("Listening on {}", listen_addr);
    let listener = create_listening_socket(listen_addr)?;
    let event_loop = EventLoop::new()?;
    event_loop.poller().register_read(listener.0)?;
    let mut mgr = ServerManager::new(Duration::from_secs(15));
    let cfg = config::load_config(std::path::Path::new("config.conf"))?;
    let server = &cfg.servers[0];
    let root = server.root.clone().unwrap_or_else(|| std::path::PathBuf::from("www"));

    loop {
        event_loop.tick(64, Some(1000), |ev| {
            if ev.fd == listener.0 && ev.readable {
                loop {
                    match accept_nonblocking(listener.0) {
                        Ok(Some(fd)) => {
                            let fd_raw = fd.as_raw_fd();
                            mgr.insert(fd_raw, Connection::new(fd));
                            let _ = event_loop.poller().register_read(fd_raw);
                        }
                        Ok(None) => break,
                        Err(e) => { eprintln!("accept error: {e}"); break; }
                    }
                }
            } else if let Some(conn) = mgr.get_mut(ev.fd) {
                conn.touch();
                if ev.readable {
                    let mut buf = [0u8; 4096];
                    loop {
                        let n = unsafe { libc::read(ev.fd, buf.as_mut_ptr() as *mut _, buf.len()) };
                        if n > 0 {
                            let n = n as usize;
                            conn.read_buf.extend_from_slice(&buf[..n]);
                            match parse_request(&conn.read_buf, 1_048_576) {
                                ParseResult::Incomplete => break,
                                ParseResult::Error(err) => {
                                    let status = if err == "body too large" {
                                        StatusCode::PayloadTooLarge
                                    } else {
                                        StatusCode::BadRequest
                                    };
                                    let resp = error_response(status, server, &root);
                                    let mut bytes = serialize_response(&resp, false);
                                    conn.write_buf.append(&mut bytes);
                                    conn.state = core::net::connection::ConnState::Writing;
                                    let _ = event_loop.poller().register_write(ev.fd);
                                    break;
                                }
                                ParseResult::Complete(req, used) => {
                                    conn.read_buf.drain(0..used);
                                    conn.keep_alive = req.keep_alive;
                                    if !matches!(req.method, http::method::Method::Get) {
                                        let resp = error_response(StatusCode::MethodNotAllowed, server, &root);
                                        let mut bytes = serialize_response(&resp, conn.keep_alive);
                                        conn.write_buf.append(&mut bytes);
                                    } else {
                                        let resp = serve_static(server, &root, &req.path, &["index.html".into()]);
                                        let mut bytes = serialize_response(&resp, conn.keep_alive);
                                        conn.write_buf.append(&mut bytes);
                                    }
                                    conn.state = core::net::connection::ConnState::Writing;
                                    let _ = event_loop.poller().register_write(ev.fd);
                                    break;
                                }
                            }
                        } else if n == 0 {
                            conn.state = core::net::connection::ConnState::Closing;
                            break;
                        } else {
                            break;
                        }
                    }
                }
                if ev.writable && !conn.write_buf.is_empty() {
                    let n = unsafe { libc::write(ev.fd, conn.write_buf.as_ptr() as *const _, conn.write_buf.len()) };
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
                if ev.error || ev.eof || matches!(conn.state, core::net::connection::ConnState::Closing) {
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
