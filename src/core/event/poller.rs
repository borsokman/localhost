use libc::{c_int, close, kevent, kqueue, timespec, EV_ADD, EV_CLEAR, EV_DELETE, EV_DISABLE, EV_EOF, EV_ERROR, EV_ENABLE, EVFILT_READ, EVFILT_WRITE};
use std::mem::{zeroed, MaybeUninit};
use std::os::fd::RawFd;
use std::ptr;

use super::event::Event;

pub struct Poller {
    kq: RawFd,
}

impl Poller {
    pub fn new() -> Result<Self, String> {
        let kq = unsafe { kqueue() };
        if kq == -1 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Self { kq })
    }

    pub fn register_read(&self, fd: RawFd) -> Result<(), String> {
        self.kev_change(fd, EVFILT_READ, EV_ADD | EV_ENABLE | EV_CLEAR)
    }

    pub fn register_write(&self, fd: RawFd) -> Result<(), String> {
        self.kev_change(fd, EVFILT_WRITE, EV_ADD | EV_ENABLE | EV_CLEAR)
    }

    pub fn deregister(&self, fd: RawFd) -> Result<(), String> {
        self.kev_change(fd, EVFILT_READ, EV_DELETE)
            .and_then(|_| self.kev_change(fd, EVFILT_WRITE, EV_DELETE).or(Ok(())))
    }

    pub fn disable_write(&self, fd: RawFd) -> Result<(), String> {
        self.kev_change(fd, EVFILT_WRITE, EV_DISABLE)
    }

    pub fn wait(&self, max_events: usize, timeout_ms: Option<i32>) -> Result<Vec<Event>, String> {
        let mut evlist: Vec<MaybeUninit<libc::kevent>> = Vec::with_capacity(max_events);
        evlist.resize_with(max_events, MaybeUninit::uninit);

        let mut ts_storage: Option<timespec> = None;
        let ts_ptr: *const timespec = match timeout_ms {
            None => ptr::null(),
            Some(ms) => {
                let mut ts: timespec = unsafe { zeroed() };
                ts.tv_sec = (ms / 1000) as i64;
                ts.tv_nsec = ((ms % 1000) * 1_000_000) as i64;
                ts_storage = Some(ts);
                ts_storage.as_ref().unwrap() as *const timespec
            }
        };

        let n = unsafe {
            kevent(
                self.kq,
                ptr::null(),
                0,
                evlist.as_mut_ptr() as *mut libc::kevent,
                max_events as c_int,
                ts_ptr,
            )
        };
        if n < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let mut out = Vec::with_capacity(n as usize);
        for kev in evlist.into_iter().take(n as usize) {
            let kev = unsafe { kev.assume_init() };
            let readable = kev.filter == EVFILT_READ;
            let writable = kev.filter == EVFILT_WRITE;
            let error = (kev.flags & EV_ERROR) != 0;
            let eof = (kev.flags & EV_EOF) != 0;
            out.push(Event {
                fd: kev.ident as i32,
                readable,
                writable,
                error,
                eof,
            });
        }
        Ok(out)
    }

    fn kev_change(&self, fd: RawFd, filter: i16, flags: u16) -> Result<(), String> {
        let mut changelist = [unsafe { zeroed::<libc::kevent>() }];
        changelist[0].ident = fd as libc::uintptr_t;
        changelist[0].filter = filter;
        changelist[0].flags = flags;
        let res = unsafe { kevent(self.kq, changelist.as_ptr(), 1, ptr::null_mut(), 0, ptr::null()) };
        if res == -1 {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(())
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        unsafe { close(self.kq) };
    }
}