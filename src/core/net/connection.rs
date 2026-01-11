use std::time::{Duration, Instant};
use super::fd::Fd;
use std::os::fd::AsRawFd;

pub enum ConnState {
    Reading,
    Writing,
    Closing,
}

pub struct Connection {
    pub fd: Fd,
    pub fd_raw: i32,
    pub read_buf: Vec<u8>,
    pub write_buf: Vec<u8>,
    pub state: ConnState,
    pub last_activity: Instant,
    pub keep_alive: bool,
}

impl Connection {
    pub fn new(fd: Fd) -> Self {
        let fd_raw = fd.as_raw_fd();
        Self {
            fd,
            fd_raw,
            read_buf: Vec::with_capacity(4096),
            write_buf: Vec::new(),
            state: ConnState::Reading,
            last_activity: Instant::now(),
            keep_alive: true,
        }
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_timed_out(&self, idle: Duration) -> bool {
        self.last_activity.elapsed() >= idle
    }
}