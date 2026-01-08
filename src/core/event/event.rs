#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub fd: i32,
    pub readable: bool,
    pub writable: bool,
    pub error: bool,
    pub eof: bool,
}