use std::collections::HashMap;
use std::time::Duration;

use crate::core::net::connection::Connection;

pub struct ServerManager {
    conns: HashMap<i32, Connection>,
    pub idle_timeout: Duration,
}

impl ServerManager {
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            conns: HashMap::new(),
            idle_timeout,
        }
    }

    pub fn insert(&mut self, fd_raw: i32, conn: Connection) {
        self.conns.insert(fd_raw, conn);
    }

    pub fn get_mut(&mut self, fd: i32) -> Option<&mut Connection> {
        self.conns.get_mut(&fd)
    }

    pub fn remove(&mut self, fd: i32) {
        self.conns.remove(&fd);
    }

    pub fn sweep_timeouts(&mut self) -> Vec<i32> {
        let idle = self.idle_timeout;
        self.conns
            .iter()
            .filter_map(|(&fd, c)| if c.is_timed_out(idle) { Some(fd) } else { None })
            .collect()
    }
}