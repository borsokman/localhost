use std::time::{Duration, Instant};
use super::fd::Fd;
use std::os::fd::AsRawFd;
use std::net::SocketAddr;

pub enum ConnState {
    Reading,
    Writing,
    Closing,
    Cgi {
        pid: i32,
        input: Option<i32>,
        output: i32,
        data: Vec<u8>,
    },
}

pub struct Connection {
    pub fd: Fd,
    pub fd_raw: i32,
    pub local_addr: SocketAddr,
    /// Per-connection read buffer (NGINX-style, filled by one read per event)
    pub read_buf: Vec<u8>,
    /// Per-connection write buffer (NGINX-style, drained by one write per event)
    pub write_buf: Vec<u8>,
    pub state: ConnState,
    pub last_activity: Instant,
    pub keep_alive: bool,
    pub timeout: Duration,
}

impl Connection {
    pub fn new(fd: Fd, local_addr: SocketAddr, timeout: Duration) -> Self {
        let fd_raw = fd.as_raw_fd();
        Self {
            fd,
            fd_raw,
            local_addr,
            read_buf: Vec::with_capacity(8192), // 8KB buffer, typical for NGINX
            write_buf: Vec::new(),
            state: ConnState::Reading,
            last_activity: Instant::now(),
            keep_alive: true,
            timeout,
        }
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_timed_out(&self) -> bool {
        self.last_activity.elapsed() >= self.timeout
    }
}
    