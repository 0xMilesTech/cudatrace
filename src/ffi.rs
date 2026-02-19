#![allow(non_camel_case_types)]

pub use core::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

pub type c_long = i64;
pub type size_t = usize;
pub type ssize_t = isize;
pub type pid_t = i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: size_t,
}

pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

pub const EINTR: c_int = 4;

pub const O_WRONLY: c_int = 0o1;
pub const O_CREAT: c_int = 0o100;
pub const O_APPEND: c_int = 0o2000;
pub const O_CLOEXEC: c_int = 0o2000000;

pub const SYS_GETTID: c_long = 186;
pub const SYS_OPENAT: c_long = 257;
pub const SYS_CLOSE: c_long = 3;

pub const AT_FDCWD: c_int = -100;

pub const RTLD_NEXT: *mut c_void = (-1isize) as *mut c_void;

unsafe extern "C" {
    pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    pub fn dlvsym(
        handle: *mut c_void,
        symbol: *const c_char,
        version: *const c_char,
    ) -> *mut c_void;
    pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
    pub fn syscall(num: c_long, ...) -> c_long;
    pub fn process_vm_readv(
        pid: pid_t,
        local_iov: *const iovec,
        liovcnt: usize,
        remote_iov: *const iovec,
        riovcnt: usize,
        flags: usize,
    ) -> isize;
    pub fn getpid() -> pid_t;
    pub fn __errno_location() -> *mut c_int;
}
