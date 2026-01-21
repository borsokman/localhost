use libc::{
    accept, bind, c_int, fcntl, listen, sa_family_t, setsockopt, socket, sockaddr, sockaddr_in,
    sockaddr_in6, sockaddr_storage, socklen_t, AF_INET, AF_INET6, F_GETFL, F_SETFL, O_NONBLOCK,
    SOCK_STREAM, SOL_SOCKET, SO_LINGER, SO_NOSIGPIPE, SO_REUSEADDR,
};

// SO_REUSEPORT is available on macOS and Linux
// On macOS, it's defined as 0x0200
#[cfg(target_os = "macos")]
const SO_REUSEPORT: c_int = 0x0200;
#[cfg(target_os = "linux")]
const SO_REUSEPORT: c_int = 0x0F;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const SO_REUSEPORT: c_int = 0x0200; // fallback
use std::io;
use std::mem::{size_of, zeroed};
use std::net::SocketAddr;
use std::os::fd::RawFd;

use super::fd::Fd;

pub fn create_listening_socket(addr: SocketAddr) -> Result<Fd, String> {
    let (storage, len, domain) = to_sockaddr(&addr)?;
    let fd = unsafe { socket(domain, SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error().to_string());
    }

    let yes: i32 = 1;
    unsafe {
        setsockopt(
            fd,
            SOL_SOCKET,
            SO_REUSEADDR,
            &yes as *const _ as *const _,
            size_of::<i32>() as socklen_t,
        );
        // SO_REUSEPORT allows multiple sockets to bind to the same port,
        // which helps with port exhaustion under high load
        let _ = setsockopt(
            fd,
            SOL_SOCKET,
            SO_REUSEPORT,
            &yes as *const _ as *const _,
            size_of::<i32>() as socklen_t,
        );
        setsockopt(
            fd,
            SOL_SOCKET,
            SO_NOSIGPIPE,
            &yes as *const _ as *const _,
            size_of::<i32>() as socklen_t,
        );
    }

    set_nonblocking(fd)?;

    let res = unsafe {
        bind(
            fd,
            &storage as *const sockaddr_storage as *const sockaddr,
            len,
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
    let mut addr: sockaddr_storage = unsafe { zeroed() };
    let mut len = size_of::<sockaddr_storage>() as socklen_t;
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
    let yes: i32 = 1;
    unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_NOSIGPIPE,
            &yes as *const _ as *const _,
            size_of::<i32>() as socklen_t,
        );
        // Set SO_LINGER with timeout 0 to skip TIME_WAIT and free ports immediately
        // This helps prevent port exhaustion under high load
        #[repr(C)]
        struct linger {
            l_onoff: c_int,
            l_linger: c_int,
        }
        let linger_val = linger {
            l_onoff: 1,
            l_linger: 0,
        };
        let _ = libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            SO_LINGER,
            &linger_val as *const _ as *const _,
            size_of::<linger>() as socklen_t,
        );
    }
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

fn to_sockaddr(addr: &SocketAddr) -> Result<(sockaddr_storage, socklen_t, c_int), String> {
    let mut storage: sockaddr_storage = unsafe { zeroed() };
    match addr {
        SocketAddr::V4(v4) => {
            let mut sa: sockaddr_in = unsafe { zeroed() };
            sa.sin_family = AF_INET as sa_family_t;
            sa.sin_port = v4.port().to_be();
            sa.sin_addr.s_addr = u32::from_ne_bytes(v4.ip().octets()).to_be();
            unsafe {
                std::ptr::write(&mut storage as *mut _ as *mut sockaddr_in, sa);
            }
            Ok((storage, size_of::<sockaddr_in>() as socklen_t, AF_INET))
        }
        SocketAddr::V6(v6) => {
            let mut sa: sockaddr_in6 = unsafe { zeroed() };
            sa.sin6_family = AF_INET6 as sa_family_t;
            sa.sin6_port = v6.port().to_be();
            sa.sin6_flowinfo = v6.flowinfo();
            sa.sin6_scope_id = v6.scope_id();
            sa.sin6_addr.s6_addr = v6.ip().octets();
            unsafe {
                std::ptr::write(&mut storage as *mut _ as *mut sockaddr_in6, sa);
            }
            Ok((storage, size_of::<sockaddr_in6>() as socklen_t, AF_INET6))
        }
    }
}