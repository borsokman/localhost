mod core;
use core::event::{EventLoop, Poller};
use core::net::socket::{accept_nonblocking, create_listening_socket};
use std::net::SocketAddr;

fn main() -> Result<(), String> {
    let listen_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let listener = create_listening_socket(listen_addr)?;
    let event_loop = EventLoop::new()?;
    event_loop.poller().register_read(listener.0)?;

    loop {
        event_loop.tick(64, Some(1000), |ev| {
            if ev.fd == listener.0 && ev.readable {
                // Accept as many as possible
                loop {
                    match accept_nonblocking(listener.0) {
                        Ok(Some(fd)) => {
                            // For now just register for read; you’d store per-conn state elsewhere
                            let _ = event_loop.poller().register_read(fd.0);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            eprintln!("accept error: {e}");
                            break;
                        }
                    }
                }
            } else {
                // Placeholder: handle client fd readable/writable
                if ev.error || ev.eof {
                    let _ = event_loop.poller().deregister(ev.fd);
                    unsafe { libc::close(ev.fd) };
                }
            }
        })?;
    }
}