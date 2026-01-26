use std::collections::HashMap;

use crate::core::net::connection::Connection;

pub struct ServerManager {
    pub conns: HashMap<i32, Connection>,
    pub pipe_map: HashMap<i32, i32>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            conns: HashMap::new(),
            pipe_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, fd_raw: i32, conn: Connection) {
        self.conns.insert(fd_raw, conn);
    }

    pub fn remove(&mut self, fd: i32) {
        // If it's a connection, remove it and any associated pipes?
        // We don't know the pipes easily unless we scan or store them in Connection.
        // For now, assume caller handles unregister_pipe.
        self.conns.remove(&fd);
    }

        pub fn sweep_timeouts(&mut self) -> Vec<i32> {

            self.conns

                .iter()

                .filter_map(|(&fd, c)| if c.is_timed_out() { Some(fd) } else { None })

                .collect()

        }

    }

    