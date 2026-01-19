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

    pub fn register_pipe(&mut self, pipe_fd: i32, conn_fd: i32) {
        self.pipe_map.insert(pipe_fd, conn_fd);
    }

    pub fn unregister_pipe(&mut self, pipe_fd: i32) {
        self.pipe_map.remove(&pipe_fd);
    }

    pub fn get_mut(&mut self, fd: i32) -> Option<&mut Connection> {
        if self.conns.contains_key(&fd) {
            return self.conns.get_mut(&fd);
        }
        if let Some(&conn_fd) = self.pipe_map.get(&fd) {
            return self.conns.get_mut(&conn_fd);
        }
        None
    }

    pub fn get_conn_fd(&self, fd: i32) -> Option<i32> {
        if self.conns.contains_key(&fd) {
            Some(fd)
        } else {
            self.pipe_map.get(&fd).cloned()
        }
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

    