use libc::{
    accept, bind, fcntl, listen, setsockopt, socket, sockaddr, sockaddr_in, socklen_t, AF_INET,
    F_GETFL, F_SETFL, O_NONBLOCK, SOCK_STREAM, SOL_SOCKET, SO_NOSIGPIPE, SO_REUSEADDR,
};
use std::io;
use std::mem::{size_of, zeroed};
use std::net::SocketAddr;
use std::os::fd::RawFd;

use super::fd::Fd;

pub fn create_listening_socket(addr: SocketAddr) -> Result<Fd, String> {
    let fd = unsafe { socket(AF_INET, SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error().to_string());
    }

    let yes: i32 = 1;
    unsafe {
        setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes as *const _ as *const _, size_of::<i32>() as socklen_t);
        setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &yes as *const _ as *const _, size_of::<i32>() as socklen_t);
    }

    set_nonblocking(fd)?;

    let sa = to_sockaddr_in(addr)?;
    let res = unsafe {
        bind(
            fd,
            &sa as *const sockaddr_in as *const sockaddr,
            size_of::<sockaddr_in>() as u32,
        )
    };
    if res < 0 {
        let e = io::Error::last_os_error().to_string();
        unsafe { libc::close(fd) };
        return Err(e);
    }

    if unsafe { listen(fd, 128) } < 0 {
        let e = io::Error::last_os_error().to_string();
        unsafe { libc::close(fd) };
        return Err(e);
    }

    Ok(Fd(fd))
}

pub fn accept_nonblocking(listen_fd: RawFd) -> Result<Option<Fd>, String> {
    let mut addr: sockaddr_in = unsafe { zeroed() };
    let mut len = size_of::<sockaddr_in>() as socklen_t;
    let fd = unsafe {
        accept(
            listen_fd,
            &mut addr as *mut _ as *mut sockaddr,
            &mut len as *mut socklen_t,
        )
    };
    if fd < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock
            || err.raw_os_error() == Some(libc::EWOULDBLOCK)
            || err.raw_os_error() == Some(libc::EAGAIN)
        {
            return Ok(None);
        }
        return Err(err.to_string());
    }
    set_nonblocking(fd)?;
    Ok(Some(Fd(fd)))
}

fn set_nonblocking(fd: RawFd) -> Result<(), String> {
    let flags = unsafe { fcntl(fd, F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    if unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn to_sockaddr_in(addr: SocketAddr) -> Result<sockaddr_in, String> {
    match addr {
        SocketAddr::V4(v4) => {
            let mut sa: sockaddr_in = unsafe { zeroed() };
            sa.sin_family = AF_INET as u16;
            sa.sin_port = v4.port().to_be();
            sa.sin_addr.s_addr = u32::from_ne_bytes(v4.ip().octets()).to_be();
            Ok(sa)
        }
        SocketAddr::V6(_) => Err("IPv6 not supported yet".into()),
    }
}